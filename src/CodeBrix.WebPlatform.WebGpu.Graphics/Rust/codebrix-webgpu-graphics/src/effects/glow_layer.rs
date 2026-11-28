//! [`GlowLayer`]: soft additive glows in the 3D scene - lamps, sparks, markers, the light of
//! windows at night - with [`Glow`] and [`GlowPulse`], and how a glow faces ([`GlowFacing`]) and
//! fades when the camera comes close ([`GlowNearFade`]) (Not in C#).

use glam::{Vec2, Vec3, Vec4};

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::Camera;
use crate::effects::glow_drawing::{GlowAxes, GlowBlend, GlowDrawing, GlowInstance, GlowLook, GlowStream, AXES_SIZE, INSTANCE_SIZE};
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

/// How a glow added with [`GlowLayer::add_oriented`] faces (Not in C#).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GlowFacing {
    /// Facing the camera from every side, round - what [`GlowLayer::add`] draws.
    #[default]
    Camera,
    /// FIXED IN THE SCENE: the glow spans `right` and `up` from its centre, each times half its
    /// size, so it lies or stands where it is put and is seen at an angle. Unit axes give a glow as
    /// wide as a camera-facing one; LONGER OR SHORTER AXES STRETCH OR SQUEEZE IT (the round
    /// falloff becomes an ellipse). The top of the layer's texture is at the end of `up`. Seen
    /// from both sides; edge-on it vanishes. Axes that are not at right angles give a slanted
    /// shape; a zero axis draws nothing.
    Fixed {
        /// The direction of the glow's width, in scene units per half size.
        right: Vec3,
        /// The direction of the glow's height, in scene units per half size.
        up: Vec3,
    },
}

impl GlowFacing {
    /// A glow fixed flat across `normal` (it faces along it; zero means +Y), its height `aspect`
    /// times its width: `GlowFacing::flat(Vec3::Y, 1.0)` lies round on the ground (width along
    /// +X, height along -Z), `GlowFacing::flat(Vec3::Z, 3.0)` stands upright facing +Z, three
    /// times as tall as it is wide (width along +X, height along +Y). Any normal that is not
    /// straight up or down gives an upright glow (its height along +Y).
    pub fn flat(normal: Vec3, aspect: f32) -> GlowFacing {
        let (right, up) = flat_axes(normal);
        GlowFacing::Fixed { right, up: up * aspect.max(0.0) }
    }

    /// The axes record of the second instance stream, or None for a camera-facing glow.
    pub(crate) fn axes(self) -> Option<GlowAxes> {
        match self {
            GlowFacing::Camera => None,
            GlowFacing::Fixed { right, up } => Some(GlowAxes { right, up }),
        }
    }
}

/// Unit width and height axes of a quad facing `normal`: right x up = normal.
fn flat_axes(normal: Vec3) -> (Vec3, Vec3) {
    let normal = normal.try_normalize().unwrap_or(Vec3::Y);
    let right = if normal.y.abs() > 0.999 { Vec3::X } else { Vec3::Y.cross(normal).normalize() };
    (right, normal.cross(right).normalize())
}

/// The NEAR FADE of a [`GlowLayer`] (Not in C#): a glow fades out as the camera comes close to
/// it, so that a glow the camera is inside does not wash over the whole view. Distances are the
/// camera's distance from the glow's centre IN GLOW SIZES (a 2-wide glow 3 units away is 1.5
/// sizes away): no change from `end` on, gone at `start` and closer, a smooth step between. ONE
/// setting for the layer's camera-facing and oriented glows alike: a glow lying on the ground
/// under the camera fades too when the camera is within `end` of its sizes from its centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowNearFade {
    /// Where the glow is gone, in sizes (at least 0). Default 0.5 (the camera at the glow's
    /// edge).
    pub start: f32,
    /// Where the fade begins, in sizes (taken as just beyond `start` when it is not). Default 2.
    pub end: f32,
}

impl Default for GlowNearFade {
    fn default() -> GlowNearFade {
        GlowNearFade { start: 0.5, end: 2.0 }
    }
}

impl GlowNearFade {
    /// The shader's terms: start at least 0, end beyond start (so end > 0 means "on").
    pub(crate) fn shader_terms(self) -> Vec2 {
        let start = self.start.max(0.0);
        let start = if start.is_finite() { start } else { 0.0 };
        let end = self.end.max(start + 1e-3);
        Vec2::new(start, if end.is_finite() { end } else { start + 1e-3 })
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
/// straight alpha, times the glow's colour). Each glow faces the camera - or, added with
/// [`GlowLayer::add_oriented`], lies or stands fixed in the scene, stretched as its
/// [`GlowFacing`] says (those are drawn in a second instanced draw). An opt-in NEAR FADE
/// ([`GlowLayer::set_near_fade`], off by default) fades glows the camera comes close to. Glows are tested against
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
    oriented: Vec<GlowInstance>,
    oriented_axes: Vec<GlowAxes>,
    buffer: GpuBuffer,
    capacity: u64,
    cursor: u64,
    /// The oriented glows' axes, made when the first one is drawn.
    axes_buffer: Option<GpuBuffer>,
    axes_capacity: u64,
    axes_cursor: u64,
    near_fade: Option<GlowNearFade>,
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
        Ok(GlowLayer {
            drawing,
            glows: Vec::new(),
            oriented: Vec::new(),
            oriented_axes: Vec::new(),
            buffer,
            capacity,
            cursor: 0,
            axes_buffer: None,
            axes_capacity: 0,
            axes_cursor: 0,
            near_fade: None,
            frame_id: None,
            current: None,
            camera: None,
            texture: None,
            ignored: 0,
            last_glow_count: 0,
            last_draw_call_count: 0,
        })
    }

    /// The near fade (Not in C#; None, the default: glows never fade near the camera).
    pub fn near_fade(&self) -> Option<GlowNearFade> {
        self.near_fade
    }

    /// Fades every glow of the layer as the camera comes close to it ([`GlowNearFade`]: from
    /// `end` glow sizes away to gone at `start`); None turns it off. Applies from the next `end`.
    pub fn set_near_fade(&mut self, fade: Option<GlowNearFade>) {
        self.near_fade = fade;
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

    /// The draw calls the last `end` recorded: 1 when it had glows (2 when some were camera-facing
    /// and some oriented), 0 when it had none.
    pub fn last_draw_call_count(&self) -> usize {
        self.last_draw_call_count
    }

    /// The pipelines made so far (one per target format combination, and one more per
    /// combination that drew oriented glows).
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
            self.axes_cursor = 0;
        }
        self.glows.clear();
        self.oriented.clear();
        self.oriented_axes.clear();
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

    /// Adds a glow that faces as `facing` says (Not in C#): [`GlowFacing::Camera`] is
    /// [`GlowLayer::add`]; [`GlowFacing::Fixed`] lies or stands fixed in the scene, stretched by its
    /// axes. Oriented glows are drawn after the camera-facing ones of the pair, in one more
    /// instanced draw. Outside begin/end it is ignored, and the next `end` reports it.
    pub fn add_oriented(&mut self, glow: &Glow, facing: GlowFacing) {
        let Some(axes) = facing.axes() else {
            self.add(glow);
            return;
        };
        if self.current.is_none() {
            self.ignored += 1;
            return;
        }
        if glow.size > 0.0 && glow.intensity > 0.0 {
            self.oriented.push(glow.to_instance());
            self.oriented_axes.push(axes);
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
        let oriented = self.oriented.len() as u64;
        if count + oriented > 0 {
            let needed = self.cursor + count + oriented;
            if needed > self.capacity {
                // The old buffer stays alive for work already recorded in this frame.
                let capacity = needed.next_power_of_two().max(self.capacity * 2);
                let context = self.buffer.context().clone();
                self.buffer = context.create_buffer(capacity * INSTANCE_SIZE, wgpu::BufferUsages::VERTEX, Some("glow layer"))?;
                self.capacity = capacity;
                self.cursor = 0;
            }
            let look = GlowLook { near_fade: self.near_fade.map(GlowNearFade::shader_terms).unwrap_or(Vec2::ZERO), ..GlowLook::plain(GlowBlend::Additive, self.texture.as_ref()) };
            if count > 0 {
                let offset = self.cursor * INSTANCE_SIZE;
                self.buffer.write(&self.glows, offset)?;
                self.drawing.draw(pass, &camera, current.time, GlowStream { instances: &self.buffer, offset, count: count as u32, axes: None }, &look)?;
                self.cursor += count;
                self.last_draw_call_count += 1;
            }
            if oriented > 0 {
                let needed = self.axes_cursor + oriented;
                if self.axes_buffer.is_none() || needed > self.axes_capacity {
                    let capacity = needed.next_power_of_two().max(self.axes_capacity * 2).max(16);
                    let context = self.buffer.context().clone();
                    self.axes_buffer = Some(context.create_buffer(capacity * AXES_SIZE, wgpu::BufferUsages::VERTEX, Some("glow layer axes"))?);
                    self.axes_capacity = capacity;
                    self.axes_cursor = 0;
                }
                let Some(axes) = self.axes_buffer.as_ref() else {
                    return Err(GraphicsException::new("The glow layer's axes buffer is missing."));
                };
                let offset = self.cursor * INSTANCE_SIZE;
                let axes_offset = self.axes_cursor * AXES_SIZE;
                self.buffer.write(&self.oriented, offset)?;
                axes.write(&self.oriented_axes, axes_offset)?;
                self.drawing.draw(pass, &camera, current.time, GlowStream { instances: &self.buffer, offset, count: oriented as u32, axes: Some((axes, axes_offset)) }, &look)?;
                self.cursor += oriented;
                self.axes_cursor += oriented;
                self.last_draw_call_count += 1;
            }
            self.last_glow_count = (count + oriented) as usize;
        }
        if ignored > 0 {
            return Err(GraphicsException::new(format!("{ignored} glow(s) were added outside begin/end and were not drawn; add glows between begin and end.")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec2, Vec3, Vec4};

    use super::{Glow, GlowFacing, GlowNearFade, GlowPulse};
    use crate::effects::glow_drawing::GlowAxes;

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

    #[test]
    fn an_oriented_glow_packs_its_axes_into_the_second_stream() {
        assert_eq!(GlowFacing::default(), GlowFacing::Camera);
        assert_eq!(GlowFacing::Camera.axes(), None, "camera-facing glows have no axes record");
        let fixed = GlowFacing::Fixed { right: Vec3::new(2.0, 0.0, 0.0), up: Vec3::new(0.0, 0.5, 0.0) };
        assert_eq!(fixed.axes(), Some(GlowAxes { right: Vec3::new(2.0, 0.0, 0.0), up: Vec3::new(0.0, 0.5, 0.0) }), "the lengths are kept: they stretch the glow");
        let record = GlowAxes { right: Vec3::new(1.0, 2.0, 3.0), up: Vec3::new(4.0, 5.0, 6.0) };
        let bytes: &[u8] = bytemuck::bytes_of(&record);
        let floats: Vec<f32> = bytes.chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        assert_eq!(floats, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "right.xyz then up.xyz, 24 bytes");
        // The corners of the quad as the shader places them: centre + (right x, up y) * size / 2.
        let corner = |facing: GlowFacing, x: f32, y: f32| {
            let axes = facing.axes().expect("fixed");
            Vec3::new(1.0, 0.0, 0.0) + (axes.right * x + axes.up * y) * (2.0 * 0.5)
        };
        assert_eq!(corner(fixed, 1.0, 1.0), Vec3::new(3.0, 0.5, 0.0));
    }

    #[test]
    fn flat_glows_lie_on_the_ground_or_stand_up_with_their_aspect() {
        let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-5;
        let GlowFacing::Fixed { right, up } = GlowFacing::flat(Vec3::Y, 1.0) else { panic!("fixed") };
        assert!(close(right, Vec3::X) && close(up, -Vec3::Z), "{right} {up}");
        assert!(close(right.cross(up), Vec3::Y), "it faces its normal");
        let GlowFacing::Fixed { right, up } = GlowFacing::flat(Vec3::Z, 3.0) else { panic!("fixed") };
        assert!(close(right, Vec3::X) && close(up, Vec3::new(0.0, 3.0, 0.0)), "upright, three times as tall: {right} {up}");
        for normal in [Vec3::new(1.0, 0.0, 1.0), Vec3::new(-0.3, 0.2, 0.9), -Vec3::Y, Vec3::new(0.0, -1.0, 0.0001)] {
            let GlowFacing::Fixed { right, up } = GlowFacing::flat(normal, 2.0) else { panic!("fixed") };
            assert!((right.length() - 1.0).abs() < 1e-5 && (up.length() - 2.0).abs() < 1e-5);
            assert!(right.dot(up).abs() < 1e-5);
            assert!(close(right.cross(up / 2.0), normal.normalize()), "{normal}");
        }
        let GlowFacing::Fixed { up, .. } = GlowFacing::flat(Vec3::ZERO, -1.0) else { panic!("fixed") };
        assert_eq!(up, Vec3::ZERO, "a negative aspect draws nothing");
    }

    #[test]
    fn the_near_fade_is_off_by_default_and_its_terms_are_kept_in_order() {
        assert_eq!(GlowNearFade::default(), GlowNearFade { start: 0.5, end: 2.0 });
        assert_eq!(GlowNearFade::default().shader_terms(), Vec2::new(0.5, 2.0));
        assert_eq!(GlowNearFade { start: -1.0, end: 1.0 }.shader_terms(), Vec2::new(0.0, 1.0));
        let flipped = GlowNearFade { start: 2.0, end: 1.0 }.shader_terms();
        assert!(flipped.x == 2.0 && flipped.y > 2.0, "end is kept beyond start");
        let zero = GlowNearFade { start: 0.0, end: 0.0 }.shader_terms();
        assert!(zero.y > 0.0, "on means end above 0");
        let odd = GlowNearFade { start: f32::NAN, end: f32::INFINITY }.shader_terms();
        assert!(odd.x == 0.0 && odd.y.is_finite() && odd.y > 0.0);
    }
}
