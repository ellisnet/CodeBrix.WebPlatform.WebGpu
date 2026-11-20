//! [`SpriteBatch`]: batched 2D sprites and shapes - textured quads (whole textures or atlas
//! regions), rectangles, lines, circles, triangles and quads, with a transform stack and a colour
//! per item (C#: `SpriteBatch`), with the options structs of its C# default arguments.

use std::collections::HashMap;

use glam::{Affine2, Mat4, Vec2};

use crate::common::{Color4, RectangleF};
use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing2d::sprite_builder::{RunTexture, SpriteBuilder};
use crate::drawing2d::SpriteVertex;
use crate::frames::{BufferRange, DrawIndexedOptions, RenderPass};
use crate::images::AnimatedTexture;
use crate::pipelines::{BlendPreset, ColorTargetOptions, DepthOptions, DepthPreset, GpuRenderPipeline, VertexLayoutOptions};
use crate::resources::{BindingEntry, BindingLayoutEntry, GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuSampler, GpuShader, GpuTexture, SamplerDescription};
use crate::shaders::BuiltInShaders;

/// The bytes of one uniform slot (the WebGPU minimum alignment of a dynamic offset).
const UNIFORM_SLOT_SIZE: u64 = 256;
/// The bytes of one vertex.
const VERTEX_SIZE: u64 = 24;

/// The defaulted arguments of [`SpriteBatch::begin`] (C#: `transform = null` (identity),
/// `blendPreset = BlendPreset.AlphaBlend`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteBatchBeginOptions {
    /// An initial transform, applied to every position before the projection (glam's `Affine2`
    /// for C#'s `Matrix3x2`). A canvas layer usually passes
    /// `Affine2::from_scale(Vec2::splat(pixel_ratio))`, so that it draws in CSS pixels.
    pub transform: Affine2,
    /// How items blend with what is under them. The default, straight alpha, is right for
    /// colours and textures whose alpha is NOT premultiplied; it writes the same premultiplied
    /// result the canvas host's convention asks for.
    pub blend: BlendPreset,
}

impl Default for SpriteBatchBeginOptions {
    fn default() -> SpriteBatchBeginOptions {
        SpriteBatchBeginOptions { transform: Affine2::IDENTITY, blend: BlendPreset::AlphaBlend }
    }
}

/// The defaulted arguments of [`SpriteBatch::draw_circle`] (C#: `thickness = 1`, `segments = 0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircleOptions {
    /// The outline's thickness in pixels.
    pub thickness: f32,
    /// The number of segments (0: chosen from the radius).
    pub segments: u32,
}

impl Default for CircleOptions {
    fn default() -> CircleOptions {
        CircleOptions { thickness: 1.0, segments: 0 }
    }
}

/// A texture as the batch remembers it between a draw and `end`: the `wgpu` handles (cheap
/// clones), compared by the texture.
#[derive(Clone, Debug)]
struct TextureKey {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl PartialEq for TextureKey {
    fn eq(&self, other: &Self) -> bool {
        // The VIEW decides what is sampled: the layer views of one texture array (an animated
        // texture's frames) share the texture but are different pictures.
        self.texture == other.texture && self.view == other.view
    }
}

/// What `begin` was given, until `end`.
#[derive(Clone, Copy, Debug)]
struct Drawing {
    frame_id: u64,
    blend: BlendPreset,
    projection: Mat4,
}

/// A batched 2D renderer for sprites and shapes (C#: `SpriteBatch`): textured quads (whole
/// textures or atlas regions), rectangles, lines, circles, triangles and quads, with a transform
/// stack and a colour per item. Coordinates are pixels of the pass's target with the origin at
/// its top-left corner and y pointing down. Everything between [`SpriteBatch::begin`] and
/// [`SpriteBatch::end`] becomes ONE indexed draw per run of the same texture - shapes never break
/// a run, so sprites from one atlas and any number of shapes cost a single draw. Items are drawn
/// in the order they were added (a later item covers an earlier one): there is no sort mode, as
/// in C#.
///
/// Several batches (or several begin/end pairs) may draw in one frame: each keeps its data in its
/// own region of the batch's buffers, started over when a new frame begins.
///
/// THE DIFFERENCES FROM C#, both forced by Rust's borrows: `begin` takes the pass to read its
/// target, and `end` takes the same pass again to record the draws into it (C#'s `End()` keeps the
/// pass from `Begin`). And the drawing calls return nothing: a call outside begin/end is ignored
/// and reported as an error by the next `end` (C# throws at the call).
pub struct SpriteBatch {
    context: GraphicsContext,
    shader: GpuShader,
    globals_layout: GpuBindGroupLayout,
    texture_layout: GpuBindGroupLayout,
    linear_sampler: GpuSampler,
    nearest_sampler: GpuSampler,
    pipelines: HashMap<(wgpu::TextureFormat, Option<wgpu::TextureFormat>, BlendPreset), GpuRenderPipeline>,
    texture_groups: HashMap<(wgpu::TextureView, bool), GpuBindGroup>,
    uniforms: GpuBuffer,
    uniform_group: GpuBindGroup,
    uniform_capacity: u64,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    index_quad_capacity: usize,
    builder: SpriteBuilder<TextureKey>,
    frame_id: Option<u64>,
    uniform_slot: u64,
    vertex_cursor: u64,
    drawing: Option<Drawing>,
    ignored: usize,
    use_nearest_filtering: bool,
    last_draw_call_count: usize,
    last_quad_count: usize,
}

impl SpriteBatch {
    /// The quads the buffers hold before they grow, when none is given (C#: 2048).
    pub const DEFAULT_QUAD_CAPACITY: usize = 2048;

    /// A batch with room for [`SpriteBatch::DEFAULT_QUAD_CAPACITY`] quads before its buffers grow
    /// (C#: `new SpriteBatch(context)`). Pipelines are built when first needed, one per target
    /// format and blend combination.
    pub fn new(context: &GraphicsContext) -> Result<SpriteBatch, GraphicsException> {
        SpriteBatch::with_capacity(context, SpriteBatch::DEFAULT_QUAD_CAPACITY)
    }

    /// A batch with room for `initial_quad_capacity` quads (at least 64) before its buffers grow
    /// (C#: `new SpriteBatch(context, initialQuadCapacity)`).
    pub fn with_capacity(context: &GraphicsContext, initial_quad_capacity: usize) -> Result<SpriteBatch, GraphicsException> {
        let quads = initial_quad_capacity.max(64);
        let shader = context.create_shader(BuiltInShaders::SPRITE, Some("sprite batch"));
        let globals_layout = context.create_bind_group_layout(Some("sprite globals"), &[BindingLayoutEntry::uniform(0, wgpu::ShaderStages::VERTEX).with_dynamic_offset(true).with_min_binding_size(64)]);
        let texture_layout = context.create_bind_group_layout(Some("sprite texture"), &[BindingLayoutEntry::texture(0, wgpu::ShaderStages::FRAGMENT), BindingLayoutEntry::sampler(1, wgpu::ShaderStages::FRAGMENT)]);
        let linear_sampler = context.create_sampler(&SamplerDescription::linear_clamp());
        let nearest_sampler = context.create_sampler(&SamplerDescription::nearest_clamp());
        let (uniforms, uniform_group) = create_uniforms(context, &globals_layout, 16)?;
        let vertex_buffer = context.create_buffer(quads as u64 * 4 * VERTEX_SIZE, wgpu::BufferUsages::VERTEX, Some("sprite vertices"))?;
        let index_buffer = create_indices(context, quads)?;
        Ok(SpriteBatch {
            context: context.clone(),
            shader,
            globals_layout,
            texture_layout,
            linear_sampler,
            nearest_sampler,
            pipelines: HashMap::new(),
            texture_groups: HashMap::new(),
            uniforms,
            uniform_group,
            uniform_capacity: 16,
            vertex_buffer,
            index_buffer,
            index_quad_capacity: quads,
            builder: SpriteBuilder::with_capacity(quads),
            frame_id: None,
            uniform_slot: 0,
            vertex_cursor: 0,
            drawing: None,
            ignored: 0,
            use_nearest_filtering: false,
            last_draw_call_count: 0,
            last_quad_count: 0,
        })
    }

    /// The context (C#: `Context`).
    pub fn context(&self) -> &GraphicsContext {
        &self.context
    }

    /// True when textures are sampled with nearest-texel filtering (pixel art, exact atlases)
    /// instead of linear (C#: `UseNearestFiltering`, default false).
    pub fn use_nearest_filtering(&self) -> bool {
        self.use_nearest_filtering
    }

    /// Chooses nearest-texel (true) or linear (false) filtering for the textures drawn from now
    /// on (C#: `UseNearestFiltering = value`).
    pub fn set_use_nearest_filtering(&mut self, value: bool) {
        self.use_nearest_filtering = value;
    }

    /// True between [`SpriteBatch::begin`] and [`SpriteBatch::end`] (C#: `IsDrawing`).
    pub fn is_drawing(&self) -> bool {
        self.drawing.is_some()
    }

    /// The current transform (C#: `Transform`).
    pub fn transform(&self) -> Affine2 {
        self.builder.transform()
    }

    /// How many draw calls the last [`SpriteBatch::end`] recorded (C#: `LastDrawCallCount`).
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// How many quads the last [`SpriteBatch::end`] drew; shapes count as quads, a triangle as
    /// one (C#: `LastQuadCount`).
    pub fn last_quad_count(&self) -> usize {
        self.last_quad_count
    }

    /// Starts collecting for `pass` (C#: `Begin(renderPass, transform, blendPreset)`). The
    /// projection maps the pass target's pixels (top-left origin, y down).
    pub fn begin(&mut self, pass: &RenderPass<'_>, options: SpriteBatchBeginOptions) -> Result<(), GraphicsException> {
        if self.drawing.is_some() {
            return Err(GraphicsException::new("SpriteBatch::begin was called twice without end; call end(pass) first."));
        }
        let frame_id = pass.frame_id();
        if self.frame_id != Some(frame_id) {
            self.frame_id = Some(frame_id);
            self.uniform_slot = 0;
            self.vertex_cursor = 0;
        }
        let target = pass.target();
        let projection = glam::camera::rh::proj::directx::orthographic(0.0, target.width().max(1) as f32, target.height().max(1) as f32, 0.0, -1.0, 1.0);
        self.builder.reset(options.transform);
        self.drawing = Some(Drawing { frame_id, blend: options.blend, projection });
        Ok(())
    }

    /// Pushes a transform, combined with the current one: the new one applies FIRST (C#:
    /// `PushTransform`; in C# terms `transform * Transform`, in glam's `transform() * transform`).
    pub fn push_transform(&mut self, transform: Affine2) {
        if self.drawing.is_none() {
            self.ignored += 1;
            return;
        }
        self.builder.push_transform(transform);
    }

    /// Pops the last pushed transform (C#: `PopTransform`). An error when only the initial
    /// transform is left.
    pub fn pop_transform(&mut self) -> Result<(), GraphicsException> {
        if self.builder.pop_transform() {
            Ok(())
        } else {
            Err(GraphicsException::new("SpriteBatch::pop_transform has no matching push_transform; push before you pop."))
        }
    }

    /// Draws a whole texture into a rectangle (C#: `Draw(texture, destination, tint)`); the tint
    /// multiplies the texture (white leaves it unchanged).
    pub fn draw(&mut self, texture: &GpuTexture, destination: RectangleF, tint: Color4) {
        let source = RectangleF::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);
        self.draw_region(texture, destination, source, tint);
    }

    /// Draws a region of a texture (an atlas cell, in texels) into a rectangle (C#:
    /// `Draw(texture, destination, source, tint)`).
    pub fn draw_region(&mut self, texture: &GpuTexture, destination: RectangleF, source: RectangleF, tint: Color4) {
        if !self.accepts() {
            return;
        }
        self.builder.draw_region(key(texture), size(texture), destination, source, tint);
    }

    /// Draws a region of a texture with its `origin` (in texels from the region's top-left
    /// corner) at `position`, turned by `rotation` radians clockwise (y points down) and scaled
    /// (C#: `Draw(texture, position, source, tint, rotation, origin, scale)`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_rotated(&mut self, texture: &GpuTexture, position: Vec2, source: RectangleF, tint: Color4, rotation: f32, origin: Vec2, scale: Vec2) {
        if !self.accepts() {
            return;
        }
        self.builder.draw_rotated(key(texture), size(texture), position, source, tint, rotation, origin, scale);
    }

    /// Draws the frame of an animated texture shown after `elapsed` seconds into a rectangle (C#:
    /// `Draw(AnimatedTexture, elapsed, destination, tint)`). Pass the frame's time (or the time
    /// since the animation started): the batch reads no clock.
    pub fn draw_animated(&mut self, texture: &AnimatedTexture, elapsed: f64, destination: RectangleF, tint: Color4) {
        self.draw(texture.get_frame_texture(elapsed), destination, tint);
    }

    /// Draws the frame of an animated texture shown after `elapsed` seconds, positioned, rotated
    /// and scaled about an origin (C#: `Draw(AnimatedTexture, elapsed, position, tint, rotation,
    /// origin, scale)`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_animated_rotated(&mut self, texture: &AnimatedTexture, elapsed: f64, position: Vec2, tint: Color4, rotation: f32, origin: Vec2, scale: Vec2) {
        let source = RectangleF::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);
        self.draw_rotated(texture.get_frame_texture(elapsed), position, source, tint, rotation, origin, scale);
    }

    /// Fills a rectangle (C#: `FillRectangle`).
    pub fn fill_rectangle(&mut self, rectangle: RectangleF, color: Color4) {
        if self.accepts() {
            self.builder.fill_rectangle(rectangle, color);
        }
    }

    /// Outlines a rectangle; the outline lies inside it (C#: `DrawRectangle(rectangle, color,
    /// thickness)`; C#'s default thickness is 1).
    pub fn draw_rectangle(&mut self, rectangle: RectangleF, color: Color4, thickness: f32) {
        if self.accepts() {
            self.builder.draw_rectangle(rectangle, color, thickness);
        }
    }

    /// Draws a line segment `thickness` pixels wide (C#: `DrawLine`; default thickness 1).
    pub fn draw_line(&mut self, from: Vec2, to: Vec2, color: Color4, thickness: f32) {
        if self.accepts() {
            self.builder.draw_line(from, to, color, thickness);
        }
    }

    /// Fills a triangle (C#: `FillTriangle`).
    pub fn fill_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color4) {
        if self.accepts() {
            self.builder.fill_triangle(a, b, c, color);
        }
    }

    /// Fills a convex quadrilateral, corners in order around it (C#: `FillQuad`).
    pub fn fill_quad(&mut self, a: Vec2, b: Vec2, c: Vec2, d: Vec2, color: Color4) {
        if self.accepts() {
            self.builder.fill_quad(a, b, c, d, color);
        }
    }

    /// Fills a circle of `segments` triangles, 0 to choose them from the radius (C#: `FillCircle`;
    /// default 0).
    pub fn fill_circle(&mut self, center: Vec2, radius: f32, color: Color4, segments: u32) {
        if self.accepts() {
            self.builder.fill_circle(center, radius, color, segments);
        }
    }

    /// Outlines a circle; the outline is centred on the radius (C#: `DrawCircle(center, radius,
    /// color, thickness, segments)`).
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color4, options: CircleOptions) {
        if self.accepts() {
            self.builder.draw_circle(center, radius, color, options.thickness, options.segments);
        }
    }

    /// Uploads everything collected since [`SpriteBatch::begin`] and records the draws into
    /// `pass` - the pass `begin` was given (C#: `End()`). An error when the batch is not drawing,
    /// when `pass` belongs to another frame, or when drawing calls arrived outside begin/end
    /// (they were ignored; whatever was collected properly is still drawn).
    pub fn end(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        let ignored = std::mem::take(&mut self.ignored);
        let Some(drawing) = self.drawing.take() else {
            return Err(GraphicsException::new(ignored_message("SpriteBatch::end was called without begin; call begin(pass, options) first", ignored)));
        };
        if pass.frame_id() != drawing.frame_id {
            return Err(GraphicsException::new("SpriteBatch::end was given a pass of another frame than begin; end the batch with the pass it began with."));
        }
        self.last_draw_call_count = 0;
        self.last_quad_count = self.builder.quad_count();
        if self.last_quad_count > 0 {
            self.record(pass, drawing)?;
        }
        if ignored > 0 {
            return Err(GraphicsException::new(ignored_message("The sprite batch drew what it collected", ignored)));
        }
        Ok(())
    }

    /// Forgets the batch's cached bind groups of a texture (C#: `Forget`): call it when a texture
    /// the batch drew is dropped, to let it go sooner.
    pub fn forget(&mut self, texture: &GpuTexture) {
        for nearest in [false, true] {
            self.texture_groups.remove(&(texture.view().clone(), nearest));
        }
    }

    /// True when drawing; otherwise the call is counted as ignored.
    fn accepts(&mut self) -> bool {
        if self.drawing.is_some() {
            true
        } else {
            self.ignored += 1;
            false
        }
    }

    /// Writes the collected quads and records one draw per run.
    fn record(&mut self, pass: &mut RenderPass<'_>, drawing: Drawing) -> Result<(), GraphicsException> {
        let quads = self.builder.quad_count();
        let bytes = self.builder.vertices().len() as u64 * VERTEX_SIZE;
        if self.vertex_cursor + bytes > self.vertex_buffer.size() {
            let size = (self.vertex_buffer.size() * 2).max(self.vertex_cursor + bytes);
            self.vertex_buffer = self.context.create_buffer(size, wgpu::BufferUsages::VERTEX, Some("sprite vertices"))?;
            self.vertex_cursor = 0;
        }
        if quads > self.index_quad_capacity {
            let capacity = quads.max(self.index_quad_capacity * 2);
            self.index_buffer = create_indices(&self.context, capacity)?;
            self.index_quad_capacity = capacity;
        }
        if self.uniform_slot >= self.uniform_capacity {
            let capacity = self.uniform_capacity * 2;
            let (uniforms, group) = create_uniforms(&self.context, &self.globals_layout, capacity)?;
            self.uniforms = uniforms;
            self.uniform_group = group;
            self.uniform_capacity = capacity;
            self.uniform_slot = 0;
        }
        let uniform_offset = self.uniform_slot * UNIFORM_SLOT_SIZE;
        self.uniforms.write_value(&drawing.projection, uniform_offset)?;
        self.vertex_buffer.write(self.builder.vertices(), self.vertex_cursor)?;

        let target = pass.target().clone();
        let pipeline = self.pipeline(target.color_format(), target.depth_format(), drawing.blend)?;
        pass.set_pipeline(&pipeline);
        pass.set_bind_group_with_offsets(0, &self.uniform_group, &[uniform_offset as u32]);
        pass.set_vertex_buffer(0, &self.vertex_buffer, BufferRange { offset: self.vertex_cursor, size: Some(bytes) });
        pass.set_index_buffer(&self.index_buffer, wgpu::IndexFormat::Uint32, BufferRange::default());
        let runs: Vec<_> = self.builder.runs().to_vec();
        for run in runs {
            let group = self.texture_group(&run.texture);
            pass.set_bind_group(1, &group);
            pass.draw_indexed((run.quad_count * 6) as u32, DrawIndexedOptions { first_index: (run.first_quad * 6) as u32, ..DrawIndexedOptions::default() });
            self.last_draw_call_count += 1;
        }
        self.uniform_slot += 1;
        self.vertex_cursor += bytes;
        self.builder.reset(Affine2::IDENTITY);
        Ok(())
    }

    /// The pipeline for a target's formats and a blend preset (built on first use).
    fn pipeline(&mut self, color: wgpu::TextureFormat, depth: Option<wgpu::TextureFormat>, blend: BlendPreset) -> Result<GpuRenderPipeline, GraphicsException> {
        if let Some(pipeline) = self.pipelines.get(&(color, depth, blend)) {
            return Ok(pipeline.clone());
        }
        let mut builder = self
            .context
            .create_render_pipeline(&self.shader)
            .with_label(&format!("sprite batch {color:?} {blend:?}"))
            .with_vertex_buffer_of::<SpriteVertex>(VertexLayoutOptions::default())
            .with_color_target(color, ColorTargetOptions::blended(blend))
            .with_bind_group_layouts(&[&self.globals_layout, &self.texture_layout]);
        if let Some(depth) = depth {
            builder = builder.with_depth(depth, DepthOptions { preset: DepthPreset::None, ..DepthOptions::default() });
        }
        let pipeline = builder.build()?;
        self.pipelines.insert((color, depth, blend), pipeline.clone());
        Ok(pipeline)
    }

    /// The bind group of a run's texture with the batch's current filtering (cached).
    fn texture_group(&mut self, texture: &RunTexture<TextureKey>) -> GpuBindGroup {
        let key = match texture {
            RunTexture::Texture(key) => key.clone(),
            RunTexture::White => {
                let (texture, view) = self.context.white_texture();
                TextureKey { texture, view }
            }
        };
        let nearest = self.use_nearest_filtering;
        let cache_key = (key.view.clone(), nearest);
        if let Some(group) = self.texture_groups.get(&cache_key) {
            return group.clone();
        }
        let sampler = if nearest { &self.nearest_sampler } else { &self.linear_sampler };
        let group = self.context.create_bind_group(&self.texture_layout, Some("sprite texture"), &[BindingEntry::for_texture_view(0, &key.view), BindingEntry::for_sampler(1, sampler)]);
        self.texture_groups.insert(cache_key, group.clone());
        group
    }
}

impl GraphicsContext {
    /// A sprite batch on this context (C#: `CreateSpriteBatch()`, the same as `new
    /// SpriteBatch(context)`).
    pub fn create_sprite_batch(&self) -> Result<SpriteBatch, GraphicsException> {
        SpriteBatch::new(self)
    }
}

/// The batch's key of a texture.
fn key(texture: &GpuTexture) -> TextureKey {
    TextureKey { texture: texture.handle().clone(), view: texture.view().clone() }
}

/// A texture's size in texels.
fn size(texture: &GpuTexture) -> Vec2 {
    Vec2::new(texture.width() as f32, texture.height() as f32)
}

/// The uniform buffer of `slots` projection slots and its bind group.
fn create_uniforms(context: &GraphicsContext, layout: &GpuBindGroupLayout, slots: u64) -> Result<(GpuBuffer, GpuBindGroup), GraphicsException> {
    let uniforms = context.create_buffer(slots * UNIFORM_SLOT_SIZE, wgpu::BufferUsages::UNIFORM, Some("sprite globals"))?;
    let group = context.create_bind_group(layout, Some("sprite globals"), &[BindingEntry::for_buffer(0, &uniforms).with_range(0, Some(64))]);
    Ok((uniforms, group))
}

/// The index buffer of `quads` quads: (0, 1, 2) and (0, 2, 3) per quad.
fn create_indices(context: &GraphicsContext, quads: usize) -> Result<GpuBuffer, GraphicsException> {
    context.create_index_buffer_u32(&quad_indices(quads), Some("sprite indices"))
}

/// Two triangles per quad, counter-clockwise as the corners are given.
pub(crate) fn quad_indices(quads: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity(quads * 6);
    for quad in 0..quads as u32 {
        let v = quad * 4;
        indices.extend_from_slice(&[v, v + 1, v + 2, v, v + 2, v + 3]);
    }
    indices
}

/// The message for draws that arrived outside begin/end.
fn ignored_message(start: &str, ignored: usize) -> String {
    if ignored == 0 {
        format!("{start}.")
    } else {
        format!("{start}; {ignored} drawing call(s) arrived outside begin/end and were ignored - call begin(pass, options) before drawing and end(pass) after.")
    }
}

#[cfg(test)]
mod tests {
    use super::{ignored_message, quad_indices, CircleOptions, SpriteBatchBeginOptions};
    use crate::pipelines::BlendPreset;

    #[test]
    fn the_options_carry_the_csharp_defaults() {
        let begin = SpriteBatchBeginOptions::default();
        assert_eq!(begin.blend, BlendPreset::AlphaBlend);
        assert_eq!(begin.transform, glam::Affine2::IDENTITY);
        assert_eq!(CircleOptions::default(), CircleOptions { thickness: 1.0, segments: 0 });
    }

    #[test]
    fn each_quad_is_two_triangles() {
        assert_eq!(quad_indices(2), vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn the_messages_say_what_to_do() {
        assert_eq!(ignored_message("No", 0), "No.");
        assert!(ignored_message("No", 3).contains("3 drawing call(s) arrived outside begin/end"));
    }
}
