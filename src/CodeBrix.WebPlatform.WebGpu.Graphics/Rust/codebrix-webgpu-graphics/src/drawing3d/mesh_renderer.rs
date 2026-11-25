//! [`MeshRenderer`]: draws [`Mesh`]es with [`LitMaterial`]s - Blinn-Phong shading with one
//! directional light and an ambient term, depth testing when the pass has depth, and instancing
//! (C#: `MeshRenderer`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::{Mat3, Mat4, Vec4};

use crate::common::Color4;
use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::lit_material::MATERIAL_UNIFORM_SIZE;
use crate::drawing3d::{Camera, LitMaterial, MaterialOptions, Mesh, MeshVertex, SceneLighting};
use crate::frames::{BufferRange, DrawIndexedOptions, RenderPass};
use crate::pipelines::{BlendPreset, ColorTargetOptions, CullPreset, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayout, VertexLayoutOptions, VertexType};
use crate::presentation::TargetFormats;
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, SamplerDescription};
use crate::shaders::BuiltInShaders;

/// The bytes of one frame-uniform slot (the WebGPU minimum alignment of a dynamic offset).
const UNIFORM_SLOT_SIZE: u64 = 256;
/// The bytes of the frame uniforms.
const FRAME_UNIFORM_SIZE: u64 = 128;

/// Numbers the renderers, so that a material knows which one made it.
static NEXT_RENDERER_ID: AtomicU64 = AtomicU64::new(1);

/// Per-instance data of the lit shader: the model matrix and the three columns of the normal
/// matrix (112 bytes; C#: the internal `InstanceData`, whose bytes are the same).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InstanceData {
    pub(crate) model: Mat4,
    pub(crate) normal: [Vec4; 3],
}

const _: () = assert!(std::mem::size_of::<InstanceData>() == 112);

// SAFETY: #[repr(C)]; a Mat4 and three Vec4 (16-byte multiples) with no padding (size asserted
// above); every bit pattern of the f32 fields is a valid value.
unsafe impl bytemuck::Zeroable for InstanceData {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for InstanceData {}

impl InstanceData {
    /// The instance data of a world matrix: the matrix, and the inverse transpose of its upper
    /// 3 x 3 (the identity for a matrix that cannot be inverted), column by column - the rows of
    /// C#'s row-vector normal matrix.
    pub(crate) fn for_world(world: Mat4) -> InstanceData {
        let inverse = if world.determinant().abs() > f32::MIN_POSITIVE { world.inverse() } else { Mat4::IDENTITY };
        let normal = inverse.transpose();
        InstanceData { model: world, normal: [normal.x_axis.truncate().extend(0.0), normal.y_axis.truncate().extend(0.0), normal.z_axis.truncate().extend(0.0)] }
    }
}

impl VertexType for InstanceData {
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
        let mut builder = VertexLayout::describe(options);
        for _ in 0..7 {
            builder = builder.add(wgpu::VertexFormat::Float32x4);
        }
        builder.build(112)
    }
}

/// True when a matrix mirrors: its upper 3 x 3 has a negative determinant (C#: `IsMirroring`).
pub(crate) fn is_mirroring(world: &Mat4) -> bool {
    Mat3::from_mat4(*world).determinant() < 0.0
}

/// One draw of a begin/end pair: a geometry, a material and a range of instances.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawItem<G, M> {
    pub(crate) geometry: G,
    pub(crate) material: M,
    pub(crate) first_instance: usize,
    pub(crate) instance_count: usize,
    pub(crate) mirrored: bool,
}

/// The CPU half of the renderer: the instances and the draws of one begin/end pair (tested on the
/// host with plain values for the geometry and the material).
pub(crate) struct MeshBatch<G, M> {
    pub(crate) instances: Vec<InstanceData>,
    pub(crate) draws: Vec<DrawItem<G, M>>,
}

impl<G: Clone + PartialEq, M: Clone + PartialEq> MeshBatch<G, M> {
    pub(crate) fn new() -> MeshBatch<G, M> {
        MeshBatch { instances: Vec::new(), draws: Vec::new() }
    }

    pub(crate) fn clear(&mut self) {
        self.instances.clear();
        self.draws.clear();
    }

    /// Adds one instance per matrix. A mirroring matrix turns clockwise triangles into front
    /// faces, so each run of matrices with the same winding becomes its own draw; a draw that
    /// continues the last one (same geometry, material and winding) joins it (C#'s
    /// `DrawGeometry` and `AddDraw`).
    pub(crate) fn add(&mut self, geometry: &G, material: &M, worlds: &[Mat4]) {
        let mut start = 0;
        while start < worlds.len() {
            let mirrored = is_mirroring(&worlds[start]);
            let mut end = start + 1;
            while end < worlds.len() && is_mirroring(&worlds[end]) == mirrored {
                end += 1;
            }
            let first = self.instances.len();
            self.instances.extend(worlds[start..end].iter().map(|world| InstanceData::for_world(*world)));
            let count = end - start;
            match self.draws.last_mut() {
                Some(last) if last.geometry == *geometry && last.material == *material && last.mirrored == mirrored && last.first_instance + last.instance_count == first => last.instance_count += count,
                _ => self.draws.push(DrawItem { geometry: geometry.clone(), material: material.clone(), first_instance: first, instance_count: count, mirrored }),
            }
            start = end;
        }
    }
}

/// The frame uniforms of the lit shader (128 bytes; C#: `FrameUniforms`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameUniforms {
    view_projection: Mat4,
    camera_position: Vec4,
    light_direction: Vec4,
    light_color: Vec4,
    ambient_color: Vec4,
}

const _: () = assert!(std::mem::size_of::<FrameUniforms>() == FRAME_UNIFORM_SIZE as usize);

// SAFETY: #[repr(C)]; a Mat4 and four Vec4 with no padding (size asserted above).
unsafe impl bytemuck::Zeroable for FrameUniforms {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for FrameUniforms {}

impl FrameUniforms {
    /// C#'s frame data: the key light's direction (straight down when it has none), its colour
    /// times its intensity, and the ambient colour.
    fn new(camera: &Camera, lighting: &SceneLighting, aspect_ratio: f32) -> FrameUniforms {
        let direction = if lighting.light_direction.length_squared() > 0.0 { lighting.light_direction.normalize() } else { -glam::Vec3::Y };
        FrameUniforms {
            view_projection: camera.get_view_projection_matrix(aspect_ratio),
            camera_position: camera.position.extend(1.0),
            light_direction: direction.extend(0.0),
            light_color: lighting.light_color.to_vector4() * lighting.light_intensity,
            ambient_color: lighting.ambient_color.to_vector4(),
        }
    }
}

/// A mesh's buffers as a draw remembers them until `end` (cheap `wgpu` clones).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeshGeometry {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// What `begin` was given, until `end`.
#[derive(Clone, Copy, Debug)]
struct Drawing {
    frame_id: u64,
    frame: FrameUniforms,
}

/// Draws [`Mesh`]es with [`LitMaterial`]s (C#: `MeshRenderer`): Blinn-Phong shading with one
/// directional light and an ambient term, an optional base-colour texture, depth testing when the
/// pass target has a depth view, and per-instance model matrices - consecutive draws of the same
/// mesh and material become ONE instanced draw. [`MeshRenderer::begin`] with a camera and
/// lighting, [`MeshRenderer::draw`] or [`MeshRenderer::draw_instanced`] any number of times, then
/// [`MeshRenderer::end`]: the opaque draws first, then the translucent ones (blended, depth
/// read-only), each group in the order drawn.
///
/// A mirroring world matrix (negative determinant) draws with clockwise front faces, so mirrored
/// instances stay visible and shade like the original. Several begin/end pairs may run in one
/// frame; each keeps its own data.
///
/// THE DIFFERENCE FROM C# (Rust's borrows): `begin` takes the pass to read its target, and `end`
/// takes the same pass again to record the draws into it.
pub struct MeshRenderer {
    id: u64,
    context: GraphicsContext,
    shader: GpuShader,
    frame_layout: GpuBindGroupLayout,
    material_layout: GpuBindGroupLayout,
    material_sampler: GpuSampler,
    frame_uniforms: GpuBuffer,
    frame_group: GpuBindGroup,
    uniform_capacity: u64,
    uniform_slot: u64,
    instance_buffer: GpuBuffer,
    instance_cursor: u64,
    frame_id: Option<u64>,
    drawing: Option<Drawing>,
    batch: MeshBatch<MeshGeometry, LitMaterial>,
    /// The pipelines by target (colour, depth, sample count: a multisampled target gets its own)
    /// and kind of draw.
    pipelines: HashMap<PipelineKey, GpuRenderPipeline>,
    last_draw_call_count: usize,
    last_instance_count: usize,
}

impl MeshRenderer {
    /// A renderer; its pipelines are built when first needed, one per target format combination
    /// (colour format, depth format and sample count, so it draws into 1- and 4-sample targets
    /// alike) and kind of draw (C#: `new MeshRenderer(context)`).
    pub fn new(context: &GraphicsContext) -> Result<MeshRenderer, GraphicsException> {
        let shader = context.create_shader(BuiltInShaders::LIT_MESH, Some("lit mesh"));
        let frame_layout = context.create_bind_group_layout(Some("mesh frame"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT).with_dynamic_offset(true).with_min_binding_size(FRAME_UNIFORM_SIZE)]);
        let material_layout = context.create_bind_group_layout(
            Some("mesh material"),
            &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::FRAGMENT).with_min_binding_size(MATERIAL_UNIFORM_SIZE), BindingLayoutEntry::texture(1, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(2, wgpu::ShaderStages::FRAGMENT)],
        );
        let material_sampler = context.create_sampler(&SamplerDescription::linear_repeat());
        let (frame_uniforms, frame_group) = create_frame_uniforms(context, &frame_layout, 8)?;
        let instance_buffer = context.create_buffer(112 * 1024, wgpu::BufferUsages::VERTEX, Some("mesh instances"))?;
        Ok(MeshRenderer {
            id: NEXT_RENDERER_ID.fetch_add(1, Ordering::Relaxed),
            context: context.clone(),
            shader,
            frame_layout,
            material_layout,
            material_sampler,
            frame_uniforms,
            frame_group,
            uniform_capacity: 8,
            uniform_slot: 0,
            instance_buffer,
            instance_cursor: 0,
            frame_id: None,
            drawing: None,
            batch: MeshBatch::new(),
            pipelines: HashMap::new(),
            last_draw_call_count: 0,
            last_instance_count: 0,
        })
    }

    /// The context (C#: `Context`).
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// True between [`MeshRenderer::begin`] and [`MeshRenderer::end`] (C#: `IsDrawing`).
    pub fn is_drawing(&self) -> bool {
        self.drawing.is_some()
    }

    /// How many draw calls the last [`MeshRenderer::end`] recorded (C#: `LastDrawCallCount`).
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// How many instances the last [`MeshRenderer::end`] drew (C#: `LastInstanceCount`).
    pub fn last_instance_count(&self) -> usize {
        self.last_instance_count
    }

    /// A material for this renderer (C#: `CreateMaterial(baseColor, baseColorTexture, doubleSided,
    /// label, sampler)`).
    pub fn create_material(&self, base_color: Color4, options: MaterialOptions<'_>) -> Result<LitMaterial, GraphicsException> {
        let uniforms_label = options.label.map_or_else(|| "material".to_string(), |l| format!("{l} uniforms"));
        let uniforms = self.context.create_uniform_buffer(MATERIAL_UNIFORM_SIZE, Some(&uniforms_label))?;
        let (_, white_view) = self.context.white_texture();
        let view = options.base_color_texture.map_or(white_view, |texture| texture.view().clone());
        let sampler = options.sampler.map_or(&self.material_sampler, |sampler| sampler);
        let group = self.context.create_bind_group(&self.material_layout, Some("material"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(MATERIAL_UNIFORM_SIZE)), BindingEntry::for_texture_view(1, &view), BindingEntry::for_sampler(2, sampler)]);
        Ok(LitMaterial::new(self.id, uniforms, group, base_color, options.base_color_texture.map(|t| t.handle().clone()), options.sampler.map(|s| s.handle().clone()), options.double_sided, options.label))
    }

    /// Starts drawing for `pass` with a camera and lighting, None for the [`SceneLighting`]
    /// defaults (C#: `Begin(renderPass, camera, lighting)`). The aspect ratio is the pass
    /// target's.
    pub fn begin(&mut self, pass: &RenderPass<'_>, camera: &Camera, lighting: Option<&SceneLighting>) -> Result<(), GraphicsException> {
        if self.drawing.is_some() {
            return Err(GraphicsException::new("MeshRenderer::begin was called twice without end; call end(pass) first."));
        }
        let frame_id = pass.frame_id();
        if self.frame_id != Some(frame_id) {
            self.frame_id = Some(frame_id);
            self.uniform_slot = 0;
            self.instance_cursor = 0;
        }
        let defaults = SceneLighting::default();
        let frame = FrameUniforms::new(camera, lighting.unwrap_or(&defaults), pass.target().aspect_ratio());
        self.batch.clear();
        self.drawing = Some(Drawing { frame_id, frame });
        Ok(())
    }

    /// Draws a mesh once with a world matrix (C#: `Draw(mesh, material, world)`).
    pub fn draw(&mut self, mesh: &Mesh, material: &LitMaterial, world: Mat4) -> Result<(), GraphicsException> {
        self.draw_instanced(mesh, material, &[world])
    }

    /// Draws a mesh once per world matrix (C#: `DrawInstanced(mesh, material, worlds)`). An error
    /// outside begin/end, or for a material of another renderer.
    pub fn draw_instanced(&mut self, mesh: &Mesh, material: &LitMaterial, worlds: &[Mat4]) -> Result<(), GraphicsException> {
        if self.drawing.is_none() {
            return Err(GraphicsException::new("MeshRenderer::draw was called outside begin/end; call begin(pass, camera, lighting) first."));
        }
        if material.renderer_id() != self.id {
            return Err(GraphicsException::new("The material belongs to another MeshRenderer; create it with this renderer's create_material."));
        }
        let geometry = MeshGeometry { vertex_buffer: mesh.vertex_buffer().handle().clone(), index_buffer: mesh.index_buffer().handle().clone(), index_count: mesh.index_count() as u32 };
        self.batch.add(&geometry, material, worlds);
        Ok(())
    }

    /// Uploads the frame and instance data and records the draws into `pass` - the pass `begin`
    /// was given (C#: `End()`).
    pub fn end(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        let Some(drawing) = self.drawing.take() else {
            return Err(GraphicsException::new("MeshRenderer::end was called without begin; call begin(pass, camera, lighting) first."));
        };
        if pass.frame_id() != drawing.frame_id {
            return Err(GraphicsException::new("MeshRenderer::end was given a pass of another frame than begin; end the renderer with the pass it began with."));
        }
        self.last_draw_call_count = 0;
        self.last_instance_count = self.batch.instances.len();
        if self.batch.draws.is_empty() {
            return Ok(());
        }
        let bytes = self.batch.instances.len() as u64 * 112;
        if self.instance_cursor + bytes > self.instance_buffer.size() {
            let size = (self.instance_buffer.size() * 2).max(self.instance_cursor + bytes);
            self.instance_buffer = self.context.create_buffer(size, wgpu::BufferUsages::VERTEX, Some("mesh instances"))?;
            self.instance_cursor = 0;
        }
        if self.uniform_slot >= self.uniform_capacity {
            let capacity = self.uniform_capacity * 2;
            let (uniforms, group) = create_frame_uniforms(&self.context, &self.frame_layout, capacity)?;
            self.frame_uniforms = uniforms;
            self.frame_group = group;
            self.uniform_capacity = capacity;
            self.uniform_slot = 0;
        }
        let offset = self.uniform_slot * UNIFORM_SLOT_SIZE;
        self.frame_uniforms.write_value(&drawing.frame, offset)?;
        self.instance_buffer.write(&self.batch.instances, self.instance_cursor)?;
        let mut bound: Option<GpuRenderPipeline> = None;
        self.draw_ordered(pass, offset as u32, bytes, &mut bound, false)?;
        self.draw_ordered(pass, offset as u32, bytes, &mut bound, true)?;
        self.uniform_slot += 1;
        self.instance_cursor += bytes;
        self.batch.clear();
        Ok(())
    }

    /// Records the draws of one group (opaque or translucent), in order.
    fn draw_ordered(&mut self, pass: &mut RenderPass<'_>, offset: u32, bytes: u64, bound: &mut Option<GpuRenderPipeline>, translucent: bool) -> Result<(), GraphicsException> {
        let formats = pass.target().formats();
        let draws: Vec<DrawItem<MeshGeometry, LitMaterial>> = self.batch.draws.iter().filter(|item| item.material.is_translucent() == translucent).cloned().collect();
        let mut bound_material: Option<LitMaterial> = None;
        let mut bound_vertices: Option<wgpu::Buffer> = None;
        let mut bound_indices: Option<wgpu::Buffer> = None;
        for item in draws {
            item.material.upload()?;
            let pipeline = self.pipeline(formats, item.material.double_sided(), translucent, item.mirrored)?;
            if bound.as_ref().map(|b| b.handle()) != Some(pipeline.handle()) {
                pass.set_pipeline(&pipeline);
                pass.set_bind_group_with_offsets(0, &self.frame_group, &[offset]);
                pass.set_vertex_buffer(1, &self.instance_buffer, BufferRange { offset: self.instance_cursor, size: Some(bytes) });
                *bound = Some(pipeline);
                bound_material = None;
                bound_vertices = None;
                bound_indices = None;
            }
            if bound_material.as_ref() != Some(&item.material) {
                pass.set_bind_group(1, item.material.bind_group());
                bound_material = Some(item.material.clone());
            }
            if bound_vertices.as_ref() != Some(&item.geometry.vertex_buffer) {
                pass.handle().set_vertex_buffer(0, item.geometry.vertex_buffer.slice(..));
                bound_vertices = Some(item.geometry.vertex_buffer.clone());
            }
            if bound_indices.as_ref() != Some(&item.geometry.index_buffer) {
                pass.handle().set_index_buffer(item.geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                bound_indices = Some(item.geometry.index_buffer.clone());
            }
            pass.draw_indexed(item.geometry.index_count, DrawIndexedOptions { instance_count: item.instance_count as u32, first_instance: item.first_instance as u32, ..DrawIndexedOptions::default() });
            self.last_draw_call_count += 1;
        }
        Ok(())
    }

    /// The pipeline for a target (its formats and sample count) and a material's kind of draw
    /// (built on first use).
    fn pipeline(&mut self, formats: TargetFormats, double_sided: bool, translucent: bool, mirrored: bool) -> Result<GpuRenderPipeline, GraphicsException> {
        let key = PipelineKey { formats, double_sided, translucent, mirrored };
        if let Some(pipeline) = self.pipelines.get(&key) {
            return Ok(pipeline.clone());
        }
        let (color, depth) = (formats.color, formats.depth);
        let label = format!("lit mesh {color:?}{}{}{}", if double_sided { " double-sided" } else { "" }, if translucent { " translucent" } else { "" }, if mirrored { " mirrored" } else { "" });
        let mut builder = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&label)
            .with_vertex_buffer_of::<MeshVertex>(VertexLayoutOptions::default())
            .with_vertex_buffer_of::<InstanceData>(VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 4 })
            .with_color_target(color, ColorTargetOptions::blended(if translucent { BlendPreset::AlphaBlend } else { BlendPreset::Opaque }))
            .with_culling(if double_sided { CullPreset::None } else { CullPreset::Back }, if mirrored { wgpu::FrontFace::Cw } else { wgpu::FrontFace::Ccw })
            .with_bind_group_layouts(&[&self.frame_layout, &self.material_layout])
            .with_sample_count(formats.sample_count);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: if translucent { DepthPreset::ReadOnly } else { DepthPreset::ReadWrite }, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert(key, pipeline.clone());
        Ok(pipeline)
    }
}

/// What a mesh pipeline is kept by: the target's formats (with its sample count) and the kind of
/// draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PipelineKey {
    formats: TargetFormats,
    double_sided: bool,
    translucent: bool,
    mirrored: bool,
}

impl GraphicsContext {
    /// A mesh renderer on this context (C#: `CreateMeshRenderer()`, the same as `new
    /// MeshRenderer(context)`).
    pub fn create_mesh_renderer(&self) -> Result<MeshRenderer, GraphicsException> {
        MeshRenderer::new(self)
    }
}

/// The frame-uniform buffer of `slots` slots and its bind group.
fn create_frame_uniforms(context: &GraphicsContext, layout: &GpuBindGroupLayout, slots: u64) -> Result<(GpuBuffer, GpuBindGroup), GraphicsException> {
    let uniforms = context.create_buffer(slots * UNIFORM_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("mesh frame"))?;
    let group = context.create_bind_group(layout, Some("mesh frame"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(FRAME_UNIFORM_SIZE))]);
    Ok((uniforms, group))
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3, Vec4};

    use super::{is_mirroring, DrawItem, FrameUniforms, InstanceData, MeshBatch, PipelineKey};
    use crate::drawing3d::{Camera, SceneLighting};
    use crate::pipelines::{VertexLayoutOptions, VertexType};

    #[test]
    fn the_pipelines_are_kept_per_sample_count_of_the_target() {
        // A 4-sample target needs pipelines of its own (WebGPU refuses a 1-sample pipeline there,
        // and the pass's check panics): the key holds the target's formats with the count.
        let key = |samples: u32| PipelineKey { formats: crate::presentation::TargetFormats::new(wgpu::TextureFormat::Bgra8Unorm, Some(wgpu::TextureFormat::Depth32Float), samples), double_sided: false, translucent: false, mirrored: false };
        let keys: std::collections::HashSet<PipelineKey> = crate::presentation::SAMPLE_COUNTS.into_iter().map(key).collect();
        assert_eq!(keys.len(), 2);
        assert_eq!(key(4), key(4));
        assert_eq!(key(4).formats.sample_count, 4);
    }

    #[test]
    fn a_negative_scale_mirrors() {
        assert!(!is_mirroring(&Mat4::IDENTITY));
        assert!(is_mirroring(&Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))));
        assert!(!is_mirroring(&Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))));
    }

    #[test]
    fn instance_data_carries_the_normal_matrix_columns() {
        let world = Mat4::from_scale(Vec3::new(2.0, 4.0, 1.0)) * Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let data = InstanceData::for_world(world);
        assert_eq!(data.model, world);
        assert!(data.normal[0].distance(Vec4::new(0.5, 0.0, 0.0, 0.0)) < 1e-6);
        assert!(data.normal[1].distance(Vec4::new(0.0, 0.25, 0.0, 0.0)) < 1e-6);
        assert_eq!(InstanceData::for_world(Mat4::ZERO).normal[2], Vec4::new(0.0, 0.0, 1.0, 0.0));
        let layout = InstanceData::vertex_layout(VertexLayoutOptions { step_mode: wgpu::VertexStepMode::Instance, first_location: 4 });
        assert_eq!(layout.array_stride(), 112);
        assert_eq!(layout.elements().iter().map(|e| e.location).collect::<Vec<_>>(), vec![4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn consecutive_draws_of_one_mesh_and_material_become_one() {
        let mut batch = MeshBatch::<u32, u32>::new();
        batch.add(&1, &10, &[Mat4::IDENTITY]);
        batch.add(&1, &10, &[Mat4::IDENTITY, Mat4::IDENTITY]);
        batch.add(&2, &10, &[Mat4::IDENTITY]);
        batch.add(&2, &11, &[Mat4::IDENTITY]);
        assert_eq!(batch.instances.len(), 5);
        assert_eq!(
            batch.draws,
            vec![
                DrawItem { geometry: 1, material: 10, first_instance: 0, instance_count: 3, mirrored: false },
                DrawItem { geometry: 2, material: 10, first_instance: 3, instance_count: 1, mirrored: false },
                DrawItem { geometry: 2, material: 11, first_instance: 4, instance_count: 1, mirrored: false },
            ]
        );
    }

    #[test]
    fn mirrored_runs_are_drawn_apart() {
        let mut batch = MeshBatch::<u32, u32>::new();
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        batch.add(&1, &10, &[Mat4::IDENTITY, mirror, mirror, Mat4::IDENTITY]);
        let shape: Vec<(usize, usize, bool)> = batch.draws.iter().map(|d| (d.first_instance, d.instance_count, d.mirrored)).collect();
        assert_eq!(shape, vec![(0, 1, false), (1, 2, true), (3, 1, false)]);
        batch.clear();
        assert!(batch.draws.is_empty() && batch.instances.is_empty());
    }

    #[test]
    fn the_frame_uniforms_follow_the_camera_and_the_key_light() {
        let mut lighting = SceneLighting::default();
        lighting.light_intensity = 2.0;
        lighting.light_direction = Vec3::ZERO;
        let camera = Camera::default();
        let frame = FrameUniforms::new(&camera, &lighting, 2.0);
        assert_eq!(frame.camera_position, Vec4::new(0.0, 0.0, 5.0, 1.0));
        assert_eq!(frame.light_direction, Vec4::new(0.0, -1.0, 0.0, 0.0));
        assert_eq!(frame.light_color, Vec4::new(2.0, 2.0, 2.0, 2.0));
        assert_eq!(frame.view_projection, camera.get_view_projection_matrix(2.0));
    }
}
