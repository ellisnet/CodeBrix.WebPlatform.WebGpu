//! [`GlowLayer`]: soft additive glows in the 3D scene - lamps, sparks, markers, the light of
//! windows at night - with [`Glow`] and [`GlowPulse`] (Not in C#).

use glam::{Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::Camera;
use crate::effects::glow_drawing::{GlowBlend, GlowDrawing, GlowInstance, INSTANCE_SIZE};
use crate::frames::RenderPass;
use crate::resources::{GpuBuffer, GpuTexture};

/// A pulse of a [`Glow`]'s brightness, evaluated on the GPU from the clock time given to
/// [`GlowLayer::begin`] (Not in C#): a raised cosine that is at full brightness at the start of
/// each cycle and at `1 - depth` halfway through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowPulse {
    /// Cycles per second.
    pub rate: f32,
    /// How far the brightness dips, 0 (no pulse) to 1 (down to dark).
    pub depth: f32,
    /// Where in its cycle the pulse is at time 0, in cycles (0..1): glows with different phases
    /// pulse out of step.
    pub phase: f32,
}

impl Default for GlowPulse {
    fn default() -> GlowPulse {
        GlowPulse { rate: 1.0, depth: 0.5, phase: 0.0 }
    }
}

/// One glow (Not in C#): a round soft light seen from every side - brightest at its centre, falling
/// smoothly to nothing at half its size from the centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glow {
    /// The centre, in scene units.
    pub position: Vec3,
    /// The width of the glow in scene units (it fades to nothing at half this distance from the
    /// centre).
    pub size: f32,
    /// The colour, LINEAR (not sRGB-encoded).
    pub color: Vec3,
    /// Multiplies the colour; above 1 on the HDR path makes the glow bright enough for bloom.
    pub intensity: f32,
    /// An optional pulse of the brightness.
    pub pulse: Option<GlowPulse>,
}

impl Glow {
    /// A glow without a pulse.
    pub fn new(position: Vec3, size: f32, color: Vec3, intensity: f32) -> Glow {
        Glow { position, size, color, intensity, pulse: None }
    }

    /// The same glow with a pulse.
    pub fn with_pulse(self, pulse: GlowPulse) -> Glow {
        Glow { pulse: Some(pulse), ..self }
    }

    /// The instance record the shader reads.
    pub(crate) fn to_instance(self) -> GlowInstance {
        let pulse = self.pulse.unwrap_or(GlowPulse { rate: 0.0, depth: 0.0, phase: 0.0 });
        GlowInstance { position_size: self.position.extend(self.size.max(0.0)), color: self.color.max(Vec3::ZERO).extend(1.0), pulse: Vec4::new(self.intensity.max(0.0), pulse.rate, pulse.depth.clamp(0.0, 1.0), pulse.phase) }
    }
}

/// What `begin` was given, until `end`.
#[derive(Clone, Copy, Debug)]
struct Drawing {
    frame_id: u64,
    time: f64,
}

/// SOFT ADDITIVE GLOWS IN THE 3D SCENE (Not in C#). Used like the
/// [`ModelRenderer`](crate::ModelRenderer): [`GlowLayer::begin`] with the pass, the camera and the
/// frame's clock time, [`GlowLayer::add`] each glow, [`GlowLayer::end`] - which draws every glow
/// of the pair in ONE instanced draw call. Draw it after the opaque parts and the skybox,
/// alongside the blended parts (`end_opaque`, skybox, `end_blended`, glows).
///
/// The look: round, brightest at the centre, a smooth falloff to zero, ADDED to what is there
/// (order-independent; the target's alpha is kept); no texture needed - or
/// [`GlowLayer::set_texture`] gives every glow of the layer another shape (its colour and alpha,
/// straight alpha, times the glow's colour). Each glow faces the camera. Glows are tested against
/// the pass's depth (hidden behind solid things) and never write depth. A pulse
/// ([`GlowPulse`]) is evaluated on the GPU from the time given to `begin`, so the canvas clock's
/// pause, speed and seeks drive it. On the HDR path (an RGBA16Float target), a glow's
/// `intensity` above the bloom threshold makes it BLOOM ([`crate::BloomRenderer`]).
///
/// Colour output follows the other renderers: linear values into float and sRGB targets, shader
/// encoding into 8/10-bit UNORM targets (where additive blending of encoded values is only
/// approximate: prefer an sRGB or float target).
///
/// LIMITS: where a glow meets a surface its edge is cut hard (there is no soft-particle depth
/// read in this version); a glow is a LOOK, not a light - it lights nothing around it (use a
/// light of the scene's lighting for that).
///
/// Several layers, or several begin/end pairs in one frame, each keep their glows in their own
/// region of the layer's buffer, started over when a new frame begins.
pub struct GlowLayer {
    drawing: GlowDrawing,
    glows: Vec<GlowInstance>,
    buffer: GpuBuffer,
    capacity: u64,
    cursor: u64,
    frame_id: Option<u64>,
    current: Option<Drawing>,
    camera: Option<Camera>,
    texture: Option<GpuTexture>,
    ignored: usize,
    last_glow_count: usize,
    last_draw_call_count: usize,
}

impl GlowLayer {
    /// The glows the buffer holds before it grows, when none is given.
    pub const DEFAULT_CAPACITY: usize = 256;

    /// A glow layer with room for [`GlowLayer::DEFAULT_CAPACITY`] glows per frame before its
    /// buffer grows.
    pub fn new(context: &GraphicsContext) -> Result<GlowLayer, GraphicsException> {
        GlowLayer::with_capacity(context, GlowLayer::DEFAULT_CAPACITY)
    }

    /// A glow layer with room for `capacity` glows (at least 16) per frame before its buffer
    /// grows.
    pub fn with_capacity(context: &GraphicsContext, capacity: usize) -> Result<GlowLayer, GraphicsException> {
        let capacity = capacity.max(16) as u64;
        let drawing = GlowDrawing::new(context, "glow layer")?;
        let buffer = context.create_buffer(capacity * INSTANCE_SIZE, wgpu::BufferUsages::VERTEX, Some("glow layer"))?;
        Ok(GlowLayer { drawing, glows: Vec::new(), buffer, capacity, cursor: 0, frame_id: None, current: None, camera: None, texture: None, ignored: 0, last_glow_count: 0, last_draw_call_count: 0 })
    }

    /// The shape texture every glow of the layer uses (None: the round falloff).
    pub fn texture(&self) -> Option<&GpuTexture> {
        self.texture.as_ref()
    }

    /// Gives every glow of the layer the shape of `texture` (its colour and straight alpha,
    /// multiplied by the glow's colour and intensity; the texture's top at the top of the glow);
    /// None goes back to the round falloff. The layer holds the texture.
    pub fn set_texture(&mut self, texture: Option<GpuTexture>) {
        self.texture = texture;
    }

    /// True between [`GlowLayer::begin`] and [`GlowLayer::end`].
    pub fn is_drawing(&self) -> bool {
        self.current.is_some()
    }

    /// The glows the last `end` drew.
    pub fn last_glow_count(&self) -> usize {
        self.last_glow_count
    }

    /// The draw calls the last `end` recorded (1, or 0 when it had no glows).
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// The pipelines made so far (one per target format combination).
    pub fn pipeline_count(&self) -> usize {
        self.drawing.pipeline_count()
    }

    /// Starts collecting glows for `pass`, seen by `camera`, at the clock `time` in seconds
    /// (`frame.info.time` of the canvas frame: what pulses are evaluated at). An error when the
    /// layer is already between begin and end.
    pub fn begin(&mut self, pass: &RenderPass<'_>, camera: &Camera, time: f64) -> Result<(), GraphicsException> {
        if self.current.is_some() {
            return Err(GraphicsException::new("The glow layer is already drawing; call end before begin again."));
        }
        if self.frame_id != Some(pass.frame_id()) {
            self.frame_id = Some(pass.frame_id());
            self.cursor = 0;
        }
        self.glows.clear();
        self.ignored = 0;
        self.camera = Some(*camera);
        self.current = Some(Drawing { frame_id: pass.frame_id(), time });
        Ok(())
    }

    /// Adds a glow. Outside begin/end it is ignored, and the next `end` reports it.
    pub fn add(&mut self, glow: &Glow) {
        if self.current.is_none() {
            self.ignored += 1;
            return;
        }
        if glow.size > 0.0 && glow.intensity > 0.0 {
            self.glows.push(glow.to_instance());
        }
    }

    /// Adds several glows.
    pub fn add_all<'a>(&mut self, glows: impl IntoIterator<Item = &'a Glow>) {
        for glow in glows {
            self.add(glow);
        }
    }

    /// Draws the glows added since `begin` into `pass` (the same pass) in one instanced draw.
    /// An error when `begin` was not called, when glows were added outside begin/end, or when
    /// `pass` is of another frame.
    pub fn end(&mut self, pass: &mut RenderPass<'_>) -> Result<(), GraphicsException> {
        let Some(current) = self.current.take() else {
            return Err(GraphicsException::new("The glow layer's end was called without begin; call begin(pass, camera, time) first."));
        };
        let camera = self.camera.take().unwrap_or_default();
        self.last_glow_count = 0;
        self.last_draw_call_count = 0;
        if current.frame_id != pass.frame_id() {
            return Err(GraphicsException::new("The glow layer's end was given a pass of another frame; end in the pass that begin was given."));
        }
        let ignored = std::mem::take(&mut self.ignored);
        let count = self.glows.len() as u64;
        if count > 0 {
            let needed = self.cursor + count;
            if needed > self.capacity {
                // The old buffer stays alive for work already recorded in this frame.
                let capacity = needed.next_power_of_two().max(self.capacity * 2);
                let context = self.buffer.context().clone();
                self.buffer = context.create_buffer(capacity * INSTANCE_SIZE, wgpu::BufferUsages::VERTEX, Some("glow layer"))?;
                self.capacity = capacity;
                self.cursor = 0;
            }
            let offset = self.cursor * INSTANCE_SIZE;
            self.buffer.write(&self.glows, offset)?;
            self.drawing.draw(pass, &camera, current.time, &self.buffer, offset, count as u32, GlowBlend::Additive, self.texture.as_ref())?;
            self.cursor += count;
            self.last_glow_count = count as usize;
            self.last_draw_call_count = 1;
        }
        if ignored > 0 {
            return Err(GraphicsException::new(format!("{ignored} glow(s) were added outside begin/end and were not drawn; add glows between begin and end.")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, Vec4};

    use super::{Glow, GlowPulse};

    #[test]
    fn a_glow_packs_into_the_shaders_record() {
        let glow = Glow::new(Vec3::new(1.0, 2.0, 3.0), 0.5, Vec3::new(1.0, 0.5, -0.2), 4.0).with_pulse(GlowPulse { rate: 2.0, depth: 1.5, phase: 0.25 });
        let record = glow.to_instance();
        assert_eq!(record.position_size, Vec4::new(1.0, 2.0, 3.0, 0.5));
        assert_eq!(record.color, Vec4::new(1.0, 0.5, 0.0, 1.0), "negative colour is clamped, opacity 1");
        assert_eq!(record.pulse, Vec4::new(4.0, 2.0, 1.0, 0.25), "depth is clamped to 1");
        let steady = Glow::new(Vec3::ZERO, 1.0, Vec3::ONE, 1.0).to_instance();
        assert_eq!(steady.pulse, Vec4::new(1.0, 0.0, 0.0, 0.0), "no pulse: depth 0");
    }
}
