//! The GPU half of [`ModelRenderer`](crate::ModelRenderer)'s shadows (Not in C#): the shadow maps
//! (the sun's - one layer per cascade - and one layer per shadow-casting spotlight), the depth-only pipelines the casters
//! are drawn with, the light matrices' uniform slots, and the comparison sampler the PBR shader
//! tests with. What each pass draws is worked out by the draw plan; the matrices by
//! `lighting::shadows`.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use crate::context::{GraphicsContext, GraphicsException};
use crate::pipelines::{CullPreset, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayoutOptions};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, SamplerDescription};
use crate::shaders::{PbrFeatures, ShadowDepthShader};
use crate::models::pbr_data::PbrFrameUniforms;
use crate::models::ModelVertex;

/// The format of every shadow map: core WebGPU, sampled with a comparison sampler.
pub(crate) const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The bytes of one light-matrix slot (the WebGPU dynamic-offset alignment).
const SHADOW_SLOT_SIZE: u64 = 256;
/// The slope-scaled depth bias of the shadow passes (against shadow acne on sloped surfaces).
const SHADOW_SLOPE_BIAS: f32 = 2.0;
/// The samples per texel of every shadow map and depth pipeline: one, whatever the scene's
/// target uses (a comparison sampler cannot read a multisampled texture).
pub(crate) const SHADOW_SAMPLE_COUNT: u32 = 1;
/// The layers of the spotlights' shadow map.
pub(crate) const SPOT_LAYERS: u32 = 4;

/// The sun's shadow map of a [`crate::ModelRenderer`] pair, for receivers of the application's or
/// the crate's own ([`crate::ModelRenderer::sun_shadow_map`]; Not in C#) - the water surface's
/// glints and caustics, or a shader of the application's. Sample it as the model shader does:
/// the world point moved `normal_offset` along its normal, through `matrix` to clip space, x and y
/// to 0..1 (y down), then `softness` x `softness` comparisons one `texel` apart against the
/// point's depth minus `depth_bias` with a `LessEqual` comparison sampler, averaged, and mixed
/// from 1 by `strength` (1 outside the map). The view is valid until the renderer's next shadow
/// passes overwrite the map (its next `begin` with the sun's shadow).
///
/// WITH CASCADES ([`crate::SunShadowSettings::cascades`] above 1) this is the LAST cascade's map
/// and numbers: that cascade covers the whole of the camera's view from the camera out to the
/// reach (inside the region), so it alone is right everywhere any cascade is - coarser near the
/// camera than the model shader's nearer cascades. [`SunShadowMap::cascades`] has every cascade's
/// numbers, for a receiver that picks between them as the model shader does (the water surface
/// does).
#[derive(Clone, Debug)]
pub struct SunShadowMap {
    /// The map: a 2D `Depth32Float` view (bind it as `texture_depth_2d`) - with cascades, of the
    /// last cascade's layer.
    pub view: wgpu::TextureView,
    /// World to the map's clip space (the sun's orthographic view-projection; with cascades, the
    /// last cascade's).
    pub matrix: Mat4,
    /// One texel of the map in its 0..1 coordinates (1 / its size).
    pub texel: f32,
    /// What is taken from a point's depth before the comparison (the map's 0..1 depth).
    pub depth_bias: f32,
    /// How far a point is moved along its normal before the test, in world units (against acne).
    pub normal_offset: f32,
    /// Comparisons per side of the filter grid (1 to 5).
    pub softness: u32,
    /// How dark the shadow is, 0..1 (the lighting's `sun_shadows.strength`).
    pub strength: f32,
    /// Every cascade's numbers (None for the single map).
    cascades: Option<SunShadowCascades>,
}

impl SunShadowMap {
    /// Every cascade's numbers when the map has more than one cascade (Not in C#); None for the
    /// single map. The view's texture holds one layer per cascade, the nearest first: a receiver
    /// that picks between them binds `view.texture().create_view(..)` with dimension `D2Array`
    /// (`texture_depth_2d_array`).
    pub fn cascades(&self) -> Option<&SunShadowCascades> {
        self.cascades.as_ref()
    }

    /// The map's numbers without its view.
    pub(crate) fn numbers(&self) -> SunShadowNumbers {
        let layer = self.cascades.map_or(0, |c| c.count.clamp(1, 4) - 1);
        SunShadowNumbers { matrix: self.matrix, texel: self.texel, depth_bias: self.depth_bias, normal_offset: self.normal_offset, softness: self.softness, strength: self.strength, layer, cascades: self.cascades }
    }
}

/// The sun's cascades of a [`SunShadowMap`] (Not in C#; [`SunShadowMap::cascades`]): what the
/// model shader picks a cascade with and samples each with. A point's cascade is the first whose
/// far end (`splits`) lies beyond the point's depth along `view_axis` from the camera; in the last
/// `blend` share of a cascade's slice it is mixed into the next one (the last one into no shadow
/// towards the reach; beyond the reach no shadow). Cascade `i` is sampled as the single map is,
/// with `matrices[i]`, `depth_biases[i]` and `normal_offsets[i]`, from layer `i` of the map's
/// texture; the texel, softness and strength are the map's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunShadowCascades {
    /// The cascades, 2 to 4.
    pub count: u32,
    /// The camera's view direction (unit), the axis the depth is measured along.
    pub view_axis: Vec3,
    /// The share of a slice's depth its blend band takes (0 to 0.5).
    pub blend: f32,
    /// The far end of each cascade (entries past the last repeat it).
    pub splits: [f32; 4],
    /// Each cascade's view-projection (world to its clip space); unused entries zero.
    pub matrices: [Mat4; 4],
    /// Each cascade's depth bias.
    pub depth_biases: [f32; 4],
    /// Each cascade's normal offset, world units.
    pub normal_offsets: [f32; 4],
}

/// The numbers of a [`SunShadowMap`] without its view, read from the pair's frame uniforms (what
/// the model shader tests with, so the two agree): None when the pair does not cast the sun's
/// shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SunShadowNumbers {
    pub(crate) matrix: Mat4,
    pub(crate) texel: f32,
    pub(crate) depth_bias: f32,
    pub(crate) normal_offset: f32,
    pub(crate) softness: u32,
    pub(crate) strength: f32,
    /// The layer of the sun's map the numbers are of: the last cascade's (0 for the single map).
    pub(crate) layer: u32,
    /// Every cascade's numbers (None for the single map).
    pub(crate) cascades: Option<SunShadowCascades>,
}

impl SunShadowNumbers {
    /// The map with its view.
    pub(crate) fn with_view(self, view: wgpu::TextureView) -> SunShadowMap {
        SunShadowMap { view, matrix: self.matrix, texel: self.texel, depth_bias: self.depth_bias, normal_offset: self.normal_offset, softness: self.softness, strength: self.strength, cascades: self.cascades }
    }
}

/// The sun's shadow numbers of a pair's frame uniforms (`local.y` is 1 when the key light casts
/// the sun's shadow; `sun_shadow` = strength, grid, 1 / size, bias; `local.z` the normal offset):
/// the single map's, or with cascades (`local.w` above 1) the LAST cascade's (its matrix, bias
/// and offset), with every cascade's numbers.
pub(crate) fn sun_shadow_numbers(frame: &PbrFrameUniforms) -> Option<SunShadowNumbers> {
    if frame.local.y < 0.5 {
        return None;
    }
    let last = (frame.local.w as usize).clamp(1, 4) - 1;
    let (matrix, depth_bias, normal_offset) = if last == 0 { (frame.sun_matrix, frame.sun_shadow.w, frame.local.z) } else { (frame.sun_cascade_matrices[last - 1], frame.sun_biases[last], frame.sun_offsets[last]) };
    let cascades = (last > 0).then(|| {
        let mut matrices = [Mat4::ZERO; 4];
        matrices[0] = frame.sun_matrix;
        matrices[1..=last].copy_from_slice(&frame.sun_cascade_matrices[..last]);
        SunShadowCascades { count: last as u32 + 1, view_axis: frame.sun_view.truncate(), blend: frame.sun_view.w, splits: frame.sun_splits.to_array(), matrices, depth_biases: frame.sun_biases.to_array(), normal_offsets: frame.sun_offsets.to_array() }
    });
    Some(SunShadowNumbers { matrix, texel: frame.sun_shadow.z, depth_bias, normal_offset, softness: frame.sun_shadow.y.max(1.0) as u32, strength: frame.sun_shadow.x, layer: last as u32, cascades })
}

/// One depth texture of a shadow set: its size, the texture, the view the PBR shader samples, and
/// one view per layer the passes draw into.
struct ShadowTexture {
    size: u32,
    _texture: wgpu::Texture,
    sampled: wgpu::TextureView,
    layers: Vec<wgpu::TextureView>,
}

impl ShadowTexture {
    /// A map of `size` x `size` texels and `layers` layers, sampled as a 2D-array view (also with
    /// one layer: the sun's single map).
    fn new(context: &GraphicsContext, size: u32, layers: u32, label: &str) -> ShadowTexture {
        let texture = context.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: layers },
            mip_level_count: 1,
            sample_count: SHADOW_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor { label: Some(label), dimension: Some(wgpu::TextureViewDimension::D2Array), ..wgpu::TextureViewDescriptor::default() });
        let layers = (0..layers)
            .map(|layer| texture.create_view(&wgpu::TextureViewDescriptor { label: Some(label), dimension: Some(wgpu::TextureViewDimension::D2), base_array_layer: layer, array_layer_count: Some(1), ..wgpu::TextureViewDescriptor::default() }))
            .collect();
        ShadowTexture { size, _texture: texture, sampled, layers }
    }
}

/// The shadow maps one begin/end pair draws into and its frame bind group samples: the sun's map
/// (one layer per cascade) and the spotlights' four-layer map, each made when first needed (and
/// made again when its size setting or the sun's cascade count changes). A frame uses one set per pair that has shadows, so that a later pair's
/// passes never overwrite what an earlier pair's draws still read.
struct ShadowSet {
    id: u64,
    sun: Option<ShadowTexture>,
    spots: Option<ShadowTexture>,
}

/// Where a shadow pass draws: a layer of the sun's map (a cascade; 0 for the single map), or a
/// layer of the spotlights' map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShadowTarget {
    Sun(u32),
    Spot(u32),
}

/// The renderer's shadow state.
pub(crate) struct ModelShadows {
    context: GraphicsContext,
    layout: GpuBindGroupLayout,
    uniforms: GpuBuffer,
    group: GpuBindGroup,
    capacity: u64,
    cursor: u64,
    pipelines: HashMap<PbrFeatures, GpuRenderPipeline>,
    sets: Vec<ShadowSet>,
    set_cursor: usize,
    next_id: u64,
    placeholder_sun: ShadowTexture,
    placeholder_spots: ShadowTexture,
    sampler: GpuSampler,
    frame_id: Option<u64>,
}

impl ModelShadows {
    /// The state, with 1 x 1 placeholder maps for pairs without shadows (no other GPU memory until
    /// shadows are first used).
    pub(crate) fn new(context: &GraphicsContext) -> Result<ModelShadows, GraphicsException> {
        let layout = context.create_bind_group_layout(Some("model shadow pass"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX).with_dynamic_offset(true).with_min_binding_size(64)]);
        let capacity = 8;
        let uniforms = context.create_buffer(capacity * SHADOW_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model shadow matrices"))?;
        let group = create_group(context, &layout, &uniforms);
        let sampler = context.create_sampler(&SamplerDescription { compare: Some(wgpu::CompareFunction::LessEqual), mipmap_filter: wgpu::MipmapFilterMode::Nearest, max_lod: 0.0, label: Some("model shadow comparison".to_string()), ..SamplerDescription::linear_clamp() });
        Ok(ModelShadows {
            placeholder_sun: ShadowTexture::new(context, 1, 1, "model sun shadow placeholder"),
            placeholder_spots: ShadowTexture::new(context, 1, 1, "model spot shadow placeholder"),
            context: context.clone(),
            layout,
            uniforms,
            group,
            capacity,
            cursor: 0,
            pipelines: HashMap::new(),
            sets: Vec::new(),
            set_cursor: 0,
            next_id: 1,
            sampler,
            frame_id: None,
        })
    }

    /// The bind-group layout entries the PBR frame group adds for the shadows: binding 5 the
    /// sun's map (2D-array depth, a layer per cascade), 6 the spotlights' map (2D-array depth), 7
    /// the comparison sampler.
    pub(crate) fn frame_layout_entries() -> [BindingLayoutEntry; 3] {
        let fragment = wgpu::ShaderStages::FRAGMENT;
        [
            BindingLayoutEntry::texture(5, fragment).with_sample_type(wgpu::TextureSampleType::Depth).with_view_dimension(wgpu::TextureViewDimension::D2Array),
            BindingLayoutEntry::texture(6, fragment).with_sample_type(wgpu::TextureSampleType::Depth).with_view_dimension(wgpu::TextureViewDimension::D2Array),
            BindingLayoutEntry::sampler(7, fragment).with_sampler_type(wgpu::SamplerBindingType::Comparison),
        ]
    }

    /// The comparison sampler.
    pub(crate) fn sampler(&self) -> &GpuSampler {
        &self.sampler
    }

    /// Starts a frame's counting over when `frame_id` is a new frame.
    pub(crate) fn start_frame(&mut self, frame_id: u64) {
        if self.frame_id != Some(frame_id) {
            self.frame_id = Some(frame_id);
            self.set_cursor = 0;
            self.cursor = 0;
        }
    }

    /// The set for the next pair with shadows, with a sun map of `sun` = (texels, layers - one
    /// per cascade) and a spot map of `spots` texels where asked (None: that map is not needed by
    /// this pair, any kept one stays); returns its index.
    pub(crate) fn take_set(&mut self, sun: Option<(u32, u32)>, spots: Option<u32>) -> usize {
        if self.set_cursor == self.sets.len() {
            self.sets.push(ShadowSet { id: self.next_id, sun: None, spots: None });
            self.next_id += 1;
        }
        let index = self.set_cursor;
        self.set_cursor += 1;
        let context = self.context.clone();
        let set = &mut self.sets[index];
        let mut changed = false;
        if let Some((size, layers)) = sun {
            if set.sun.as_ref().map(|t| (t.size, t.layers.len() as u32)) != Some((size, layers)) {
                set.sun = Some(ShadowTexture::new(&context, size, layers, "model sun shadow map"));
                changed = true;
            }
        }
        if let Some(size) = spots {
            if set.spots.as_ref().map(|t| t.size) != Some(size) {
                set.spots = Some(ShadowTexture::new(&context, size, SPOT_LAYERS, "model spot shadow maps"));
                changed = true;
            }
        }
        if changed {
            set.id = self.next_id;
            self.next_id += 1;
        }
        index
    }

    /// The identity of what a frame bind group samples: 0 for the placeholders, else the set's id
    /// (it changes when a map is made again).
    pub(crate) fn set_id(&self, set: Option<usize>) -> u64 {
        set.map_or(0, |index| self.sets[index].id)
    }

    /// The views a frame bind group binds for a set (None: the placeholders): sun, spots.
    pub(crate) fn sampled_views(&self, set: Option<usize>) -> (&wgpu::TextureView, &wgpu::TextureView) {
        let set = set.map(|index| &self.sets[index]);
        let sun = set.and_then(|s| s.sun.as_ref()).unwrap_or(&self.placeholder_sun);
        let spots = set.and_then(|s| s.spots.as_ref()).unwrap_or(&self.placeholder_spots);
        (&sun.sampled, &spots.sampled)
    }

    /// The view a pass draws into.
    pub(crate) fn target_view(&self, set: usize, target: ShadowTarget) -> Option<&wgpu::TextureView> {
        let set = &self.sets[set];
        match target {
            ShadowTarget::Sun(layer) => set.sun.as_ref().and_then(|t| t.layers.get(layer as usize)),
            ShadowTarget::Spot(layer) => set.spots.as_ref().and_then(|t| t.layers.get(layer as usize)),
        }
    }

    /// Writes the passes' light matrices into consecutive slots after what earlier pairs of the
    /// frame wrote (growing the buffer when needed) and returns each pass's dynamic offset.
    pub(crate) fn write_matrices(&mut self, matrices: &[Mat4]) -> Result<Vec<u32>, GraphicsException> {
        if self.cursor + matrices.len() as u64 > self.capacity {
            self.capacity = (self.capacity * 2).max(self.cursor + matrices.len() as u64);
            self.uniforms = self.context.create_buffer(self.capacity * SHADOW_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("model shadow matrices"))?;
            self.group = create_group(&self.context, &self.layout, &self.uniforms);
            self.cursor = 0;
        }
        let mut offsets = Vec::with_capacity(matrices.len());
        for matrix in matrices {
            let offset = self.cursor * SHADOW_SLOT_SIZE;
            self.uniforms.write_value(matrix, offset)?;
            offsets.push(offset as u32);
            self.cursor += 1;
        }
        Ok(offsets)
    }

    /// The bind group of the light matrices (group 0 of the depth pipelines).
    pub(crate) fn group(&self) -> &GpuBindGroup {
        &self.group
    }

    /// The depth pipeline for a part's features (only skinning, morphing, alpha testing and a
    /// level-of-detail fade's dither matter), made on first use: no colour target, both faces, a
    /// slope-scaled depth bias, a fragment stage only for alpha-tested and fading parts, and
    /// ALWAYS ONE SAMPLE ([`SHADOW_SAMPLE_COUNT`]),
    /// whatever the sample count of the target the scene is drawn into: the maps are sampled with
    /// a comparison sampler, which a multisampled texture does not allow. Groups 1 to 3 are the
    /// PBR pipelines' material, instance and draw layouts.
    pub(crate) fn pipeline(&mut self, features: PbrFeatures, layouts: [&GpuBindGroupLayout; 3]) -> Result<&GpuRenderPipeline, GraphicsException> {
        let features = ShadowDepthShader::features_of(features);
        if !self.pipelines.contains_key(&features) {
            let label = format!("model shadow depth {features}");
            let shader = self.context.create_shader(&ShadowDepthShader::source(features), Some(&label));
            let mut builder = self
                .context
                .create_render_pipeline(&shader)
                .with_label(&label)
                .with_vertex_entry("vs_shadow", None)
                .with_vertex_buffer_of::<ModelVertex>(VertexLayoutOptions::default())
                .with_bind_group_layouts(&[&self.layout, layouts[0], layouts[1], layouts[2]])
                .with_culling(CullPreset::None, wgpu::FrontFace::Ccw)
                .with_depth(SHADOW_FORMAT, DepthOptions { preset: DepthPreset::ReadWrite, bias: 0, slope_scale: SHADOW_SLOPE_BIAS })
                .with_sample_count(SHADOW_SAMPLE_COUNT);
            // A depth-only pipeline: alpha-tested and fading parts keep a fragment stage that only
            // discards.
            builder = if ShadowDepthShader::needs_fragment(features) { builder.with_fragment_entry("fs_shadow", None) } else { builder.without_fragment() };
            let pipeline = builder.build()?;
            self.pipelines.insert(features, pipeline);
        }
        Ok(&self.pipelines[&features])
    }

    /// The depth pipelines made so far (at most sixteen).
    pub(crate) fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }
}

fn create_group(context: &GraphicsContext, layout: &GpuBindGroupLayout, uniforms: &GpuBuffer) -> GpuBindGroup {
    context.create_bind_group(layout, Some("model shadow pass"), &[BindingEntry::for_buffer(0, uniforms).with_range(0, Some(64))])
}

#[cfg(test)]
mod msaa_tests {
    use super::SHADOW_SAMPLE_COUNT;
    use crate::shaders::{PbrFeatures, ShadowDepthShader};

    #[test]
    fn shadow_maps_and_depth_pipelines_stay_single_sampled() {
        // The comparison sampler the PBR shader reads the maps with cannot read a multisampled
        // texture, so the maps and the depth pipelines have one sample whatever the scene's target
        // uses: the depth pipelines are keyed by the caster's features only, never by a target.
        assert_eq!(SHADOW_SAMPLE_COUNT, 1);
        let keys: std::collections::HashSet<PbrFeatures> = [PbrFeatures::NONE, PbrFeatures::ENCODE_SRGB, PbrFeatures::ENCODE_SRGB | PbrFeatures::FADED].into_iter().map(ShadowDepthShader::features_of).collect();
        assert_eq!(keys.len(), 1, "what the target decides (its encoding) never reaches the depth pipeline's key");
    }
}

#[cfg(test)]
mod cascade_tests {
    use glam::Vec3;

    use super::sun_shadow_numbers;
    use crate::drawing3d::{Camera, SceneLighting};
    use crate::lighting::shadows::{sun_cascades, SunCascades};
    use crate::lighting::SunShadowSettings;
    use crate::models::pbr_data::PbrFrameUniforms;

    fn frame_for(lighting: &SceneLighting, camera: &Camera) -> (PbrFrameUniforms, SunCascades) {
        let cascades = sun_cascades(&lighting.sun_shadows, lighting.light_direction, camera, 1.5).expect("a region");
        let mut frame = PbrFrameUniforms::new(camera, lighting, 1.5);
        frame.set_shadows(lighting, cascades.views.first(), &[]);
        frame.set_sun_cascades(&cascades);
        (frame, cascades)
    }

    #[test]
    fn the_map_handed_on_is_the_single_one_or_the_last_cascade() {
        // What the water (or any receiver of one map) samples: with cascades, the last one's layer
        // and numbers - it covers the whole view from the camera to the reach.
        let camera = Camera { position: Vec3::new(10.0, 3.0, 20.0), target: Vec3::new(30.0, 0.0, -10.0), far_plane: 4000.0, ..Camera::default() };
        let mut lighting = SceneLighting { sun_shadows: SunShadowSettings::covering(Vec3::new(-1000.0, -10.0, -1000.0), Vec3::new(1000.0, 100.0, 1000.0)), ..SceneLighting::default() };
        let (frame, cascades) = frame_for(&lighting, &camera);
        let single = sun_shadow_numbers(&frame).expect("the sun casts");
        assert_eq!((single.layer, single.matrix), (0, cascades.views[0].view_projection));
        assert_eq!((single.depth_bias, single.normal_offset), (frame.sun_shadow.w, frame.local.z));
        assert!(single.cascades.is_none(), "the single map hands on no cascades");
        for count in [2u32, 3, 4] {
            lighting.sun_shadows = lighting.sun_shadows.with_cascades(count, 600.0);
            let (frame, cascades) = frame_for(&lighting, &camera);
            let numbers = sun_shadow_numbers(&frame).expect("the sun casts");
            let last = count as usize - 1;
            assert_eq!(numbers.layer, last as u32);
            assert_eq!(numbers.matrix, cascades.views[last].view_projection);
            assert_eq!((numbers.depth_bias, numbers.normal_offset), (frame.sun_biases[last], frame.sun_offsets[last]));
            assert!(numbers.normal_offset > frame.local.z, "coarser than the first cascade");
            assert!((numbers.texel - 1.0 / 2048.0).abs() < 1e-9 && numbers.softness == 3);
            // Every cascade's numbers are handed on too (for a receiver that picks, the water).
            let all = numbers.cascades.expect("cascades");
            assert_eq!(all.count, count);
            assert_eq!((all.view_axis, all.blend), (frame.sun_view.truncate(), frame.sun_view.w));
            assert_eq!(all.splits, frame.sun_splits.to_array());
            assert_eq!((all.depth_biases, all.normal_offsets), (frame.sun_biases.to_array(), frame.sun_offsets.to_array()));
            for (i, view) in cascades.views.iter().enumerate() {
                assert_eq!(all.matrices[i], view.view_projection, "{count} {i}");
            }
            assert!(all.matrices[count as usize..].iter().all(|m| *m == glam::Mat4::ZERO));
            // The last cascade holds every receiver the nearer ones do: the camera's surroundings too.
            for point in [camera.position + camera.forward() * 2.0, camera.position + camera.forward() * 590.0] {
                assert!(cascades.views[last].volume.contains_point(point));
            }
        }
    }
}
