//! The CPU half of the renderer: a [`VectorScene`] becomes vertex lists and a list of GPU
//! operations for one frame.
//!
//! - GEOMETRY CACHE. kurbo flattens fills and expands strokes (joins, caps, miter limit,
//!   dashes) in the shape's OWN space, at a tolerance chosen from the transform's scale rounded
//!   to quarter octaves; the resulting triangle fan is kept, keyed by the shape's elements, the
//!   stroke style and that scale step. A shape that moves, turns or scales a little reuses its
//!   fan: each frame only transforms the fan's points (no flattening, no stroking).
//! - STENCIL-THEN-COVER. A path's fan counts the winding into the stencil's low 5 bits; a cover
//!   (the transformed bounding box) paints where the winding is non-zero (or odd) and clears it.
//! - BATCHING. Consecutive fills whose device bounding boxes do not overlap (on the same target,
//!   with the same fill rule and image) share ONE stencil draw and ONE cover draw.
//! - CLIPS are a depth in the stencil's top 3 bits (7 nested); groups that cannot be folded are
//!   drawn into an offscreen target (one per nesting level) and composited.
//! - FOLDING. A group with normal blending and full alpha is a clip (no offscreen); a group of
//!   one draw with normal blending puts its alpha into the draw's paint; clips and groups whose
//!   shape covers the whole canvas cost nothing.
//! - EFFECTS (blur, glow, drop shadow) are a group drawn offscreen whose composite goes through
//!   the effect passes of the GPU half (a resolve, separable blurs, a composite of shadow, glow
//!   and content); the region they touch is the content's box grown by the effects' reach.
//! - Thin strokes: a stroke narrower than one device pixel is drawn one pixel wide with its alpha
//!   reduced in proportion (MSAA would otherwise break it into dots).
//! - STROKE SETS (Not in C#) are one operation (`Op::Strokes`) whose records the GPU half uploads
//!   once per set: the builder writes only the draw's block (the element's device matrix, the
//!   time, the anchors' transforms, the boil's redraw number), grows the content by the anchors'
//!   boxes, and culls the draw before its first stroke starts or when it misses the canvas. A
//!   set that picks its colours names its source for the GPU half (a picture of the frame's
//!   list, a picture target, an application's texture) and writes the pickup into its block.
//! - STROKE CHECKPOINTS (Not in C#): a stroke draw that asks for its checkpoint cache gets a KEY
//!   (`checkpoint_keys`, after the frame is built): its own identity (set, tips, colour source,
//!   its block apart from the time, its target level, the canvas size) and a hash of EVERYTHING
//!   encoded before it in the frame (the root clear, every operation with its vertices, paints,
//!   pictures, ramps, particles, effect passes and earlier stroke draws). The GPU half restores a
//!   checkpoint only under the same key, so a restored frame is identical to the full redraw.
//! - STROKE TRANSITIONS (Not in C#): a morph's draw names its morph and which of its two orders
//!   it draws in (the first painting's below fraction 0.5, the second's from then on).
//! - SURFACE FINISH (Not in C#): a finished layer's effect group carries the finish; every stroke
//!   draw inside it is listed for the group's HEIGHT PASS (`EffectOp::height_draws`) and writes
//!   the layer's impasto into its block.
//! - WATER (Not in C#): a refraction's and a reflection's water rides on the layer's effect group
//!   (`EffectOp::water`): a ripple field's records go into the frame's ONE `ripples` buffer (its
//!   header, with the offset, into the composite's pass data); a height picture names its texture
//!   for the GPU half (a picture of the frame's list, a picture target, an application's view).
//!   The group's region grows by the refraction's largest offset and by the mirrored part of the
//!   content box (`ReflectionParams::region`).

use std::collections::HashMap;
use std::ops::Range;

use kurbo::{Affine, PathEl, Point, Rect, StrokeCtx, StrokeOpts};
use peniko::{BlendMode, Brush, Compose, Fill, Mix};

use crate::effects::effect_sizes::EffectParams;
use crate::rendering::core::paint::{self, PaintData, RampCache};
use crate::rendering::vector_scene::{Hasher, Item, VectorScene};

/// Flattening tolerance, device pixels.
pub(crate) const TOLERANCE: f64 = 0.25;
pub(crate) const MAX_DEPTH: u32 = 7;
pub(crate) const MAX_LEVELS: usize = 4;
const MAX_BATCH: usize = 256;
/// Frames a cached fan may go unused before it is dropped.
const CACHE_KEEP_FRAMES: u64 = 90;

/// One vertex: a device-space position and the paint the cover uses.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vertex {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) paint: u32,
}

/// How a group is put onto the target below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompositeMode {
    /// A Porter-Duff compose with normal mixing: fixed-function blending.
    Compose(u8),
    /// A mix mode (and any compose): the shader reads a copy of the backdrop.
    Mix,
    /// Effects (index into the frame's effect list): the group goes through the effect passes.
    Effect(u32),
}

/// One group with effects, as the GPU half draws it: the parameters, the device box of what the
/// group drew, the region the result covers (the box grown by the effects' reach, in whole
/// pixels), the offscreen passes that make it and the textures its composite reads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectOp {
    pub(crate) params: EffectParams,
    pub(crate) content: Rect,
    pub(crate) region: Rect,
    pub(crate) steps: Vec<FxStep>,
    /// The textures the composite reads: content, glow coverage, shadow coverage.
    pub(crate) reads: [FxTexture; 3],
    /// The stroke draws (indexes into the frame's stroke operations) whose height the group's
    /// surface finish lights: drawn into the height record just before the group's composite.
    pub(crate) height_draws: Vec<u32>,
    /// The group's water (Not in C#): the two slots' pass data and textures.
    pub(crate) water: Option<EffectWater>,
}

/// An effect group's water as the GPU half binds it (Not in C#): slot A the refraction's water,
/// slot B the reflection's own; each slot's pass-data words and, for a height picture, its
/// texture; the range of the frame's ripple records its fields use (for the checkpoint keys).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectWater {
    pub(crate) slots: [crate::effects::effect_sizes::WaterSlot; 2],
    pub(crate) textures: [Option<WaterBinding>; 2],
    pub(crate) ripples: Range<u32>,
}

/// The texture a water slot's height picture is read from (Not in C#).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WaterBinding {
    /// A picture of the frame's list (1-based, as `Op::Fill`'s image).
    Image(u32),
    /// A picture target's texture.
    Target(crate::rendering::PictureTarget),
    /// The application's texture view.
    Texture(codebrix_webgpu::wgpu::TextureView),
}

/// What an effect texture holds (one texture per role and level, reused by every effect of
/// the frame in turn).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FxRole {
    /// The group's pixels, resolved from the multisampled target (level 1).
    Resolved,
    /// A box-filtered copy at a coarser level.
    Down,
    /// The horizontal pass of a blur.
    Temp,
    /// The blurred content.
    Blur,
    /// The blurred coverage of the glow.
    Glow,
    /// The blurred coverage of the shadow.
    Shadow,
}

/// An effect texture: its role and its level (canvas pixels per texel: 1, 2, 4 ...).
pub(crate) type FxTexture = (FxRole, u32);

/// One offscreen pass of an effect. `quad` indexes the cover list (its vertices' paint field is
/// the pass's index in the frame's pass data).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FxStep {
    /// The group's multisampled target averaged into `Resolved` (transparent outside the content).
    Resolve { quad: Range<u32> },
    /// A box filter from one texture to a coarser one.
    Down { src: FxTexture, dst: FxTexture, quad: Range<u32> },
    /// One direction of a separable Gaussian blur.
    Blur { src: FxTexture, dst: FxTexture, quad: Range<u32> },
}

/// One pass's parameters as the effect shaders read them (544 bytes, `#[repr(C)]` plain data).
/// Down and blur passes: v0 the source's valid texels (x0 y0 x1 y1), v1 (source level, target
/// level, direction x, direction y), v4 (target width, height in texels), u0 (kernel offset,
/// kernel radius). Resolve: v0 the content's device box. Composite: v0 the region, v1 (content,
/// glow and shadow levels, flags: 1 glow, 2 shadow), v2 the glow colour, v3 the shadow colour,
/// v4 (shadow offset x, y, glow strength, alpha); and a layer's whole-frame operations and
/// transition: v5 (tone k, tone gain, gamma, bloom gain), v6 the vignette (centre x, y, radius
/// x, y), v7 (vignette strength, minimum, transition fraction, softness), v8 the transition
/// colour, v9 the transition's frame (x0 y0 x1 y1), u1.x the frame flags; and a layer's surface
/// finish (effects/surface_finish.rs `finish_pass_data`): v10 the light (x, y, z towards it, the
/// relief), v11 (gloss, weave depth, weave spacing in layer units, weave irregularity), v12 the
/// device-to-layer matrix a b c d, v13 (its e f, the weave's angle in radians, the layer's
/// device pixels per unit), u2 (finish flags: 1 on, 2 weave; the weave's seed); and a layer's WATER
/// (effects/effect_sizes.rs `water_pass_data`; Not in C#): v14 device -> water a b c d, v15 (its e
/// f, the caustics' z, their frequency), v16 water -> device a b c d (linear), v17 (refraction
/// strength, shading, glint amount, glint sharpness), v18 (light x y z towards it, caustic
/// strength), v19 (glint colour linear rgb, caustic focus), v20 (caustic colour rgb, the
/// refraction's largest offset in device pixels), v21 slot A's place (x0 y0 x1 y1), v22 (slot A's
/// picture scale and bias, slot B's), v23 the waterline (device point x y, unit normal into the
/// water x y), v24 (reflection opacity, fade and streak in device pixels, water strength), v25 the
/// tint (linear rgb, amount), v26 slot B's place, v27 (the mirror's gap and the reflection water's
/// largest offset, device pixels), u3 slot A's ripple header words (record offset, drops, swell
/// waves, flags), u4 (water flags: 1 refraction, 2 glints, 4 caustics, 8 reflection, 16 the
/// reflection's water is slot A's; slot A's kind, slot B's kind - 0 none, 1 ripples, 2 a height
/// picture; the caustics' seed), u5 slot B's ripple header words.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FxPassData {
    pub(crate) v0: [f32; 4],
    pub(crate) v1: [f32; 4],
    pub(crate) v2: [f32; 4],
    pub(crate) v3: [f32; 4],
    pub(crate) v4: [f32; 4],
    pub(crate) u0: [u32; 4],
    pub(crate) v5: [f32; 4],
    pub(crate) v6: [f32; 4],
    pub(crate) v7: [f32; 4],
    pub(crate) v8: [f32; 4],
    pub(crate) v9: [f32; 4],
    pub(crate) u1: [u32; 4],
    pub(crate) v10: [f32; 4],
    pub(crate) v11: [f32; 4],
    pub(crate) v12: [f32; 4],
    pub(crate) v13: [f32; 4],
    pub(crate) u2: [u32; 4],
    pub(crate) v14: [f32; 4],
    pub(crate) v15: [f32; 4],
    pub(crate) v16: [f32; 4],
    pub(crate) v17: [f32; 4],
    pub(crate) v18: [f32; 4],
    pub(crate) v19: [f32; 4],
    pub(crate) v20: [f32; 4],
    pub(crate) v21: [f32; 4],
    pub(crate) v22: [f32; 4],
    pub(crate) v23: [f32; 4],
    pub(crate) v24: [f32; 4],
    pub(crate) v25: [f32; 4],
    pub(crate) v26: [f32; 4],
    pub(crate) v27: [f32; 4],
    pub(crate) u3: [u32; 4],
    pub(crate) u4: [u32; 4],
    pub(crate) u5: [u32; 4],
}

/// The texels of a level that a device region covers, grown by one texel and kept inside the
/// level's texture (x0 y0 x1 y1).
pub(crate) fn level_rect(region: Rect, level: u32, width: u32, height: u32) -> Rect {
    let k = level as f64;
    let (tw, th) = (width.div_ceil(level) as f64, height.div_ceil(level) as f64);
    Rect::new(((region.x0 / k).floor() - 1.0).max(0.0), ((region.y0 / k).floor() - 1.0).max(0.0), ((region.x1 / k).ceil() + 1.0).min(tw), ((region.y1 / k).ceil() + 1.0).min(th))
}

/// One GPU operation. `stencil` ranges index the stencil vertex list, the others the cover list.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Op {
    Fill { stencil: Range<u32>, cover: Range<u32>, even_odd: bool, image: u32 },
    ClipPush { stencil: Range<u32>, cover: Range<u32>, even_odd: bool },
    ClipPop { cover: Range<u32> },
    LayerBegin { clear: Range<u32> },
    LayerEnd { mode: CompositeMode, clip: Option<(Range<u32>, Range<u32>)>, quad: Range<u32> },
    /// Soft discs, one instance each (`instances` indexes the frame's particle instances).
    Particles { instances: Range<u32> },
    /// A stroke set in one instanced draw (`draw` indexes the frame's stroke operations).
    Strokes { draw: u32 },
}

/// One stroke draw as the GPU half draws it: the set (its records uploaded once), the tips, the
/// source its colours are picked from, the draw block's byte offset in the frame's block buffer,
/// and how many records to draw.
#[derive(Clone, Debug)]
pub(crate) struct StrokeOp {
    pub(crate) set: crate::strokes::BrushStrokeSet,
    pub(crate) tips: Option<crate::strokes::BrushTips>,
    pub(crate) source: StrokeSource,
    pub(crate) offset: u32,
    pub(crate) instances: u32,
    /// The checkpoint cache asked for (and usable by the shape's facts), with the frame's whole
    /// prefix and the draw's key.
    pub(crate) checkpoint: Option<StrokeAsk>,
    /// A transition: its state and which order it draws in (0 the first painting's, 1 the
    /// second's).
    pub(crate) morph: Option<(crate::shapes::brush_strokes_shape::MorphState, u8)>,
}

/// A stroke draw's checkpoint request (Not in C#): the options, how many strokes from the first
/// are whole at the frame's time, and the draw's key (see the module's text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StrokeAsk {
    pub(crate) options: crate::strokes::StrokeCheckpoints,
    pub(crate) whole: u32,
    pub(crate) key: u64,
}

/// The texture a stroke draw picks its colours from (Not in C#).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StrokeSource {
    /// None: the records' own colours (the blank source is bound).
    None,
    /// A picture of the frame's list (1-based, as `Op::Fill`'s image).
    Image(u32),
    /// A picture target's texture.
    Target(crate::rendering::PictureTarget),
    /// The application's texture view.
    Texture(codebrix_webgpu::wgpu::TextureView),
}

/// A paint a peniko brush cannot say: noise computed per pixel (over a gradient brush, its ramp)
/// or an ownership map coloured per pixel (over an image brush, its picture).
#[derive(Clone, Copy)]
enum Special<'a> {
    Noise(&'a crate::shapes::noise_shape::NoisePaintParams),
    Owners(&'a crate::shapes::ownership_shape::OwnersPaintParams, Option<&'a peniko::ImageData>),
}

/// One particle as the GPU draws it (48 bytes, `#[repr(C)]` plain data): its centre and the two
/// axes of its disc on the device (the transform's linear part times the radius), its colour
/// linear premultiplied, and its softness.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ParticleInstance {
    pub(crate) center: [f32; 2],
    pub(crate) axis_x: [f32; 2],
    pub(crate) axis_y: [f32; 2],
    pub(crate) softness: f32,
    pub(crate) pad: f32,
    pub(crate) color: [f32; 4],
}

enum Frame {
    Clip { cover: Range<u32> },
    Nothing,
    Elided { clip: Option<Range<u32>> },
    Layer { mode: CompositeMode, alpha: f32, mix: u32, compose: u32, shape_rect: Option<Rect>, shape_els: Option<Vec<PathEl>>, content: Rect, begin: usize },
}

struct CacheEntry {
    fan: Vec<[f32; 2]>,
    bounds: Rect,
    last_used: u64,
}

/// Counters of one frame, and the CPU time it took.
///
/// THE CPU TIMINGS (`record_ms`, `build_ms`, `encode_ms`; Not in C#) are milliseconds of the
/// page's thread, measured by the layer around each step of the frame: what the application's
/// drawing closure took to record the scene, what the renderer took to turn the scene into
/// geometry, and what it took to encode the GPU work. They do NOT include the GPU's own time (the
/// work runs on the GPU after the frame is submitted) nor the other layers of the canvas. In a
/// browser the clock is `performance.now()`, which browsers coarsen (to 0.1 ms or more unless the
/// page is cross-origin isolated): read an average over many frames, not one frame's value.
/// Nothing drawn depends on these readings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderStats {
    /// Fills and strokes in the scene.
    pub draws: usize,
    /// Of them, skipped because they were off the canvas or invisible.
    pub culled: usize,
    /// Fans taken from the geometry cache.
    pub cache_hits: usize,
    /// Fans flattened or stroked this frame.
    pub cache_misses: usize,
    /// Fans in the cache.
    pub cached: usize,
    /// Groups drawn offscreen.
    pub offscreen: usize,
    /// Groups folded into a clip or a paint (no offscreen).
    pub folded: usize,
    /// Group composites that read a copy of the backdrop (mix modes).
    pub backdrop_copies: usize,
    /// Fill operations after batching.
    pub fill_batches: usize,
    /// Draw calls recorded.
    pub draw_calls: usize,
    /// Render passes recorded.
    pub passes: usize,
    /// Vertices uploaded.
    pub vertices: usize,
    /// Things the renderer could not do (too deep a clip or group nesting).
    pub unsupported: usize,
    /// Elements, groups and layers drawn through effects (blur, glow, drop shadow).
    pub effects: usize,
    /// Offscreen passes the effects took (resolves, downsamplings, blur passes).
    pub effect_passes: usize,
    /// Pictures uploaded whole to the GPU this frame (a picture drawn for the first time, or an
    /// ownership map's picture needing one more texture). Not in C#.
    pub image_uploads: usize,
    /// The bytes of those whole uploads. Not in C#.
    pub image_upload_bytes: usize,
    /// Ownership-map colour tables uploaded this frame ALONE, into a texture that already holds
    /// the map's owner numbers (a shape made with
    /// [`OwnershipShape::with_colors`](crate::shapes::OwnershipShape::with_colors)). Not in C#.
    pub color_table_uploads: usize,
    /// The bytes of those colour-table uploads. Not in C#.
    pub color_table_upload_bytes: usize,
    /// Stroke sets drawn this frame - one draw call each
    /// ([`BrushStrokesShape`](crate::shapes::BrushStrokesShape)). Not in C#.
    pub stroke_draws: usize,
    /// Stroke instances those draws drew (each set's records up to its last started one, less those
    /// a checkpoint held - `strokes_restored`). Not in C#.
    pub strokes: usize,
    /// Stroke sets uploaded to the GPU this frame (a set drawn for the first time). Not in C#.
    pub stroke_uploads: usize,
    /// The bytes of those uploads (64 per stroke). Not in C#.
    pub stroke_upload_bytes: usize,
    /// Brush tips uploaded to the GPU this frame (tips drawn for the first time). Not in C#.
    pub tip_uploads: usize,
    /// The bytes of those uploads (every mip level). Not in C#.
    pub tip_upload_bytes: usize,
    /// Stroke draws that started from a checkpoint this frame (the strokes it holds were not
    /// drawn; see [`StrokeCheckpoints`](crate::strokes::StrokeCheckpoints)). Not in C#.
    pub checkpoint_hits: usize,
    /// Checkpoints made this frame (a texture copy each). Not in C#.
    pub checkpoint_bakes: usize,
    /// Stroke draws that asked for checkpoints and were drawn from their first stroke this frame
    /// (no checkpoint at or before the time under this frame's key, or the cache could not be used).
    /// Not in C#.
    pub checkpoint_misses: usize,
    /// The strokes those hits took from their checkpoints instead of drawing them (not counted in
    /// `strokes`). Not in C#.
    pub strokes_restored: usize,
    /// Checkpoints held after this frame. Not in C#.
    pub checkpoints: usize,
    /// Their GPU bytes (16 per canvas pixel each). Not in C#.
    pub checkpoint_bytes: usize,
    /// Ripple drops the frame's refractions and reflections evaluate per pixel (each field's live
    /// drops, counted once per effect group that draws it; a field with walls evaluates each 9
    /// times). Not in C#.
    pub ripples_evaluated: usize,
    /// Live ripple drops left out because a field had more than
    /// [`RippleField::MAX_LIVE_DROPS`](crate::water::RippleField::MAX_LIVE_DROPS) at the frame's
    /// time (the weakest). Not in C#.
    pub ripples_skipped: usize,
    /// CPU milliseconds the layer's drawing closure took to record the frame's scene (the
    /// application's own code: a session's `render` call, for example). Not in C#.
    pub record_ms: f64,
    /// CPU milliseconds the renderer took to turn the scene into this frame's geometry, paints
    /// and operations (flattening, stroking, the geometry cache, batching); 0, as every counter,
    /// before the layer is prepared. Not in C#.
    pub build_ms: f64,
    /// CPU milliseconds the renderer took to encode the frame's GPU work (buffer uploads, passes
    /// and draw calls recorded into the frame's encoder). Not in C#.
    pub encode_ms: f64,
}

impl RenderStats {
    /// The frame's CPU milliseconds in this layer: `record_ms + build_ms + encode_ms`. Not in
    /// C#.
    pub fn cpu_ms(&self) -> f64 {
        self.record_ms + self.build_ms + self.encode_ms
    }
}

/// The frame being built, and what is kept between frames.
#[derive(Default)]
pub(crate) struct Builder {
    pub(crate) stencil: Vec<Vertex>,
    pub(crate) cover: Vec<Vertex>,
    pub(crate) paints: Vec<PaintData>,
    pub(crate) ops: Vec<Op>,
    pub(crate) images: Vec<crate::rendering::core::texture_ledger::FrameImage>,
    pub(crate) particles: Vec<ParticleInstance>,
    /// The frame's stroke operations, and their draw blocks (words, each block 256-byte aligned).
    pub(crate) stroke_ops: Vec<StrokeOp>,
    pub(crate) stroke_blocks: Vec<u32>,
    pub(crate) effects: Vec<EffectOp>,
    pub(crate) fx_passes: Vec<FxPassData>,
    pub(crate) fx_kernels: Vec<f32>,
    /// The frame's ripple records (Not in C#): every water slot's drops and swell waves, read by
    /// the composite at its header's offset (vec4 units).
    pub(crate) ripples: Vec<[f32; 4]>,
    pub(crate) ramps: RampCache,
    pub(crate) levels_needed: usize,
    pub(crate) root_content: Rect,
    pub(crate) root_clear: Range<u32>,
    pub(crate) final_quad: Range<u32>,
    pub(crate) stats: RenderStats,
    stack: Vec<Frame>,
    cache: HashMap<u64, CacheEntry>,
    stroke_ctx: StrokeCtx,
    frame: u64,
    batch_boxes: Vec<Rect>,
    alpha_next: f32,
    elide_rect: Option<Rect>,
    viewport: Rect,
    /// Per scene item: true for a group that holds (at any depth) a group with another blend
    /// than normal source-over - it must stay isolated (offscreen), never folded into a clip.
    isolate: Vec<bool>,
    /// Per draw (by its position in the scene): how many frames in a row it missed the cache.
    /// A draw that keeps missing (animated geometry: a morph, a trim) is not stored at all.
    miss_streak: Vec<u8>,
    /// The fan of a draw that is not stored.
    scratch: Vec<[f32; 2]>,
    /// Per ramp slot, the frame its ramp was last uploaded (part of a checkpoint key: a slot that
    /// gets another ramp changes the key of what was drawn with it).
    ramp_serials: HashMap<u32, u64>,
}

/// Where a draw's fan is.
enum FanRef {
    Cached(u64),
    Scratch,
}

fn empty_at(n: usize) -> Range<u32> {
    n as u32..n as u32
}

/// The largest scale a transform applies to a length.
fn max_scale(t: Affine) -> f64 {
    let c = t.as_coeffs();
    (c[0] * c[0] + c[1] * c[1]).sqrt().max((c[2] * c[2] + c[3] * c[3]).sqrt())
}

/// The scale rounded UP to a quarter octave (so a cached fan is fine enough), and its step.
pub(crate) fn scale_step(scale: f64) -> (f64, i32) {
    let step = (scale.max(1e-6).log2() * 4.0).ceil() as i32;
    (2f64.powf(step as f64 / 4.0), step)
}

/// Builds a triangle fan (from the first point of the path) over flattened elements.
struct Fan<'a> {
    out: &'a mut Vec<[f32; 2]>,
    pivot: Option<Point>,
    first: Point,
    last: Point,
    min: Point,
    max: Point,
}

impl Fan<'_> {
    fn tri(&mut self, b: Point) {
        let Some(p) = self.pivot else {
            return;
        };
        for q in [p, self.last, b] {
            self.out.push([q.x as f32, q.y as f32]);
        }
    }

    fn grow(&mut self, p: Point) {
        self.min = Point::new(self.min.x.min(p.x), self.min.y.min(p.y));
        self.max = Point::new(self.max.x.max(p.x), self.max.y.max(p.y));
    }

    fn el(&mut self, el: PathEl) {
        match el {
            PathEl::MoveTo(p) => {
                if self.last != self.first {
                    self.tri(self.first);
                }
                if self.pivot.is_none() {
                    self.pivot = Some(p);
                }
                self.first = p;
                self.last = p;
                self.grow(p);
            }
            PathEl::LineTo(p) => {
                self.tri(p);
                self.last = p;
                self.grow(p);
            }
            PathEl::ClosePath => {
                if self.last != self.first {
                    self.tri(self.first);
                }
                self.last = self.first;
            }
            _ => {}
        }
    }
}

/// Flattens elements into a fan; returns its bounds (None when it is empty).
pub(crate) fn fan(out: &mut Vec<[f32; 2]>, elements: impl IntoIterator<Item = PathEl>, tolerance: f64) -> Option<Rect> {
    let start = out.len();
    let mut f = Fan { out, pivot: None, first: Point::ZERO, last: Point::ZERO, min: Point::new(f64::MAX, f64::MAX), max: Point::new(f64::MIN, f64::MIN) };
    kurbo::flatten(elements, tolerance, |el| f.el(el));
    if f.last != f.first {
        let first = f.first;
        f.tri(first);
    }
    let (min, max) = (f.min, f.max);
    if out.len() == start || min.x > max.x {
        out.truncate(start);
        return None;
    }
    Some(Rect::from_points(min, max))
}

/// The device rectangle a shape is, when it is an axis-aligned rectangle under the transform.
fn device_rect(elements: &[PathEl], transform: Affine) -> Option<Rect> {
    let c = transform.as_coeffs();
    if c[1].abs() > 1e-9 || c[2].abs() > 1e-9 {
        return None;
    }
    let mut pts = Vec::with_capacity(5);
    for el in elements {
        match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => pts.push(transform * *p),
            PathEl::ClosePath => {}
            _ => return None,
        }
    }
    if pts.len() < 4 || pts.len() > 5 {
        return None;
    }
    let mut r = Rect::from_points(pts[0], pts[0]);
    for p in &pts {
        r = r.union_pt(*p);
    }
    let corner = |p: &Point| (p.x == r.x0 || p.x == r.x1) && (p.y == r.y0 || p.y == r.y1);
    if pts.iter().all(corner) {
        Some(r)
    } else {
        None
    }
}

fn is_normal(blend: &BlendMode) -> bool {
    blend.mix == Mix::Normal && blend.compose == Compose::SrcOver
}

/// Marks the groups that hold a non-normal group somewhere inside them (one pass, a stack).
fn isolation(items: &[Item], out: &mut Vec<bool>) {
    out.clear();
    out.resize(items.len(), false);
    // (item index, is a group, holds a non-normal group)
    let mut stack: Vec<(usize, bool, bool)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            Item::PushLayer { .. } | Item::PushEffects { .. } => stack.push((i, true, false)),
            Item::PushClip { .. } => stack.push((i, false, false)),
            Item::Pop => {
                if let Some((index, is_group, holds)) = stack.pop() {
                    if is_group {
                        out[index] = holds;
                    }
                    let own = match &items[index] {
                        Item::PushLayer { blend, .. } => !is_normal(blend),
                        _ => false,
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.2 |= holds || own;
                    }
                }
            }
            _ => {}
        }
    }
    // Unclosed groups: whatever they hold so far.
    while let Some((index, is_group, holds)) = stack.pop() {
        if is_group {
            out[index] = holds;
        }
    }
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

impl Builder {
    /// Turns the scene into this frame's vertices, paints and operations.
    pub(crate) fn build(&mut self, scene: &VectorScene, width: u32, height: u32) {
        self.frame += 1;
        self.stencil.clear();
        self.cover.clear();
        self.paints.clear();
        self.ops.clear();
        self.images.clear();
        self.particles.clear();
        self.stroke_ops.clear();
        self.stroke_blocks.clear();
        self.effects.clear();
        self.fx_passes.clear();
        self.fx_kernels.clear();
        self.ripples.clear();
        self.stack.clear();
        self.batch_boxes.clear();
        self.stats = RenderStats::default();
        self.levels_needed = 1;
        self.root_content = Rect::ZERO;
        self.alpha_next = 1.0;
        self.elide_rect = None;
        self.viewport = Rect::new(0.0, 0.0, width as f64, height as f64);
        // Paint 0: the "no paint" of stencil, clear and clip vertices.
        self.paints.push(PaintData::default());
        let items = &scene.items;
        isolation(items, &mut self.isolate);
        let mut i = 0;
        while i < items.len() {
            match &items[i] {
                Item::PushLayer { blend, alpha, transform, shape } => {
                    let els = &scene.elements[shape.start as usize..shape.end as usize];
                    let single = matches!(items.get(i + 1), Some(Item::Fill { .. } | Item::Stroke { .. })) && matches!(items.get(i + 2), Some(Item::Pop));
                    self.push_layer(*blend, *alpha, *transform, els, single, self.isolate[i]);
                }
                Item::PushClip { rule, transform, shape } => {
                    let els = &scene.elements[shape.start as usize..shape.end as usize];
                    let frame = self.clip_frame(els, *transform, *rule == Fill::EvenOdd);
                    self.stack.push(frame);
                }
                Item::PushEffects { params, alpha } => self.push_effects(*params, *alpha, &scene.water),
                Item::Pop => self.pop(),
                Item::Fill { rule, transform, brush, brush_transform, path, key, tint } => {
                    let els = &scene.elements[path.start as usize..path.end as usize];
                    self.draw(els, *key, None, *rule == Fill::EvenOdd, *transform, &scene.brushes[*brush as usize], *brush_transform, *tint, None);
                }
                Item::FillNoise { transform, noise, brush, path, key } => {
                    let els = &scene.elements[path.start as usize..path.end as usize];
                    self.draw(els, *key, None, false, *transform, &scene.brushes[*brush as usize], None, None, Some(Special::Noise(noise)));
                }
                Item::FillOwners { transform, owners, brush, palette, place, path, key } => {
                    let els = &scene.elements[path.start as usize..path.end as usize];
                    let palette = palette.and_then(|p| match &scene.brushes[p as usize] {
                        Brush::Image(i) => Some(&i.image),
                        _ => None,
                    });
                    self.draw(els, *key, None, false, *transform, &scene.brushes[*brush as usize], Some(*place), None, Some(Special::Owners(owners, palette)));
                }
                Item::Particles { transform, particles, softness, alpha } => {
                    self.draw_particles(&scene.particles[particles.start as usize..particles.end as usize], *transform, *softness, *alpha);
                }
                Item::Strokes { transform, draw } => self.draw_strokes(&scene.stroke_draws[*draw as usize], *transform),
                Item::Stroke { stroke, transform, brush, brush_transform, path, key } => {
                    let els = &scene.elements[path.start as usize..path.end as usize];
                    self.draw(els, *key, Some(&scene.strokes[*stroke as usize]), false, *transform, &scene.brushes[*brush as usize], *brush_transform, None, None);
                }
            }
            i += 1;
        }
        // Unclosed pushes are closed.
        while !self.stack.is_empty() {
            self.pop();
        }
        let root = self.root_content.intersect(self.viewport);
        self.root_clear = self.quad(root, 0).unwrap_or(empty_at(self.cover.len()));
        self.final_quad = self.quad(root, 0).unwrap_or(empty_at(self.cover.len()));
        self.stats.fill_batches = self.ops.iter().filter(|o| matches!(o, Op::Fill { .. })).count();
        self.stats.vertices = self.stencil.len() + self.cover.len();
        self.stats.cached = self.cache.len();
        for (slot, _) in &self.ramps.uploads {
            self.ramp_serials.insert(*slot, self.frame);
        }
        self.checkpoint_keys();
        // Eviction: fans unused for a while go; when animated geometry (a new fan every frame)
        // makes the cache much larger than one frame's use, everything not used in the last two
        // frames goes at once.
        let used = self.stats.cache_hits + self.stats.cache_misses;
        let frame = self.frame;
        if self.cache.len() > 4096 && self.cache.len() > 3 * used {
            self.cache.retain(|_, e| frame - e.last_used <= 1);
        } else if frame % 30 == 0 {
            self.cache.retain(|_, e| frame - e.last_used <= CACHE_KEEP_FRAMES);
        }
    }

    fn depth_now(&self) -> u32 {
        let mut d = 0;
        for f in self.stack.iter().rev() {
            match f {
                Frame::Layer { .. } => break,
                Frame::Clip { .. } | Frame::Elided { clip: Some(_) } => d += 1,
                _ => {}
            }
        }
        d
    }

    fn level_now(&self) -> usize {
        1 + self.stack.iter().filter(|f| matches!(f, Frame::Layer { .. })).count()
    }

    fn grow_content(&mut self, r: Rect) {
        for f in self.stack.iter_mut().rev() {
            if let Frame::Layer { content, .. } = f {
                *content = if content.width() <= 0.0 { r } else { content.union(r) };
                return;
            }
        }
        self.root_content = if self.root_content.width() <= 0.0 { r } else { self.root_content.union(r) };
    }

    /// A device-space quad (two triangles) for the cover list.
    fn quad(&mut self, r: Rect, paint: u32) -> Option<Range<u32>> {
        let r = r.intersect(self.viewport);
        if !(r.width() > 0.0 && r.height() > 0.0) {
            return None;
        }
        let start = self.cover.len() as u32;
        let (x0, y0, x1, y1) = (r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32);
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y0), (x1, y1), (x0, y1)] {
            self.cover.push(Vertex { x, y, paint });
        }
        Some(start..self.cover.len() as u32)
    }

    /// A device-space fan (clip and group shapes: not cached) into the stencil list.
    fn device_fan(&mut self, elements: impl IntoIterator<Item = PathEl>) -> (Range<u32>, Rect) {
        let mut pts = Vec::new();
        let start = self.stencil.len();
        let bounds = fan(&mut pts, elements, TOLERANCE).unwrap_or(Rect::ZERO);
        self.stencil.extend(pts.iter().map(|p| Vertex { x: p[0], y: p[1], paint: 0 }));
        (start as u32..self.stencil.len() as u32, bounds)
    }

    /// A clip by device-space elements: the stencil and cover ranges (an empty clip still gets
    /// a degenerate cover so that the depth stays even), or None when too deep.
    fn clip_ops(&mut self, elements: Vec<PathEl>, even_odd: bool) -> Option<Range<u32>> {
        if self.depth_now() >= MAX_DEPTH {
            self.stats.unsupported += 1;
            return None;
        }
        let (stencil, bounds) = self.device_fan(elements);
        let cover = self.quad(bounds, 0).unwrap_or(empty_at(self.cover.len()));
        self.ops.push(Op::ClipPush { stencil, cover: cover.clone(), even_odd });
        Some(cover)
    }

    /// What a pushed clip becomes: nothing when its shape is a rectangle holding the canvas,
    /// otherwise a stencil clip.
    fn clip_frame(&mut self, els: &[PathEl], transform: Affine, even_odd: bool) -> Frame {
        if let Some(r) = device_rect(els, transform) {
            if r.contains_rect(self.viewport) {
                return Frame::Nothing;
            }
        }
        let device: Vec<PathEl> = els.iter().map(|e| transform * *e).collect();
        match self.clip_ops(device, even_odd) {
            Some(cover) => Frame::Clip { cover },
            None => Frame::Nothing,
        }
    }

    fn push_layer(&mut self, blend: BlendMode, alpha: f32, transform: Affine, els: &[PathEl], single: bool, isolate: bool) {
        let normal = blend.mix == Mix::Normal && blend.compose == Compose::SrcOver;
        if normal && alpha >= 1.0 && !isolate {
            // Full alpha, normal blending: the group is only its clip.
            self.stats.folded += 1;
            let frame = self.clip_frame(els, transform, false);
            self.stack.push(frame);
            return;
        }
        if normal && single {
            // One draw: its paint takes the group's alpha (exact - one path never overlaps
            // itself), its shape becomes a clip only where needed.
            self.stats.folded += 1;
            self.alpha_next = alpha;
            match device_rect(els, transform) {
                Some(r) => {
                    self.elide_rect = Some(r);
                    self.stack.push(Frame::Elided { clip: None });
                }
                None => {
                    let device: Vec<PathEl> = els.iter().map(|e| transform * *e).collect();
                    let clip = self.clip_ops(device, false);
                    self.stack.push(Frame::Elided { clip });
                }
            }
            return;
        }
        let level = self.level_now();
        if level >= MAX_LEVELS {
            // Too deep: drawn into the current target as if it were a clip.
            self.stats.unsupported += 1;
            let frame = self.clip_frame(els, transform, false);
            self.stack.push(frame);
            return;
        }
        self.stats.offscreen += 1;
        self.levels_needed = self.levels_needed.max(level + 1);
        let mode = if blend.mix == Mix::Normal { CompositeMode::Compose(blend.compose as u8) } else { CompositeMode::Mix };
        let shape_rect = device_rect(els, transform);
        let shape_els = if shape_rect.is_none() { Some(els.iter().map(|e| transform * *e).collect()) } else { None };
        let begin = self.ops.len();
        self.ops.push(Op::LayerBegin { clear: 0..0 });
        self.stack.push(Frame::Layer { mode, alpha, mix: blend.mix as u32, compose: blend.compose as u32, shape_rect, shape_els, content: Rect::ZERO, begin });
    }

    /// A group with effects: always offscreen (one level), composited through the effect passes.
    /// Its water (if any) is resolved now: ripple records into the frame's buffer, pictures into
    /// the frame's list.
    fn push_effects(&mut self, params: EffectParams, alpha: f32, water: &[crate::rendering::WaterDraw]) {
        let level = self.level_now();
        if level >= MAX_LEVELS {
            // Too deep: drawn as it is, without its effects.
            self.stats.unsupported += 1;
            self.stack.push(Frame::Nothing);
            return;
        }
        self.stats.offscreen += 1;
        self.stats.effects += 1;
        self.levels_needed = self.levels_needed.max(level + 1);
        let index = self.effects.len() as u32;
        let water = params.water.and_then(|w| water.get(w.draw as usize)).map(|d| self.effect_water(d));
        self.effects.push(EffectOp { params, content: Rect::ZERO, region: Rect::ZERO, steps: Vec::new(), reads: [(FxRole::Resolved, 1); 3], height_draws: Vec::new(), water });
        let begin = self.ops.len();
        self.ops.push(Op::LayerBegin { clear: 0..0 });
        self.stack.push(Frame::Layer { mode: CompositeMode::Effect(index), alpha, mix: 0, compose: Compose::SrcOver as u32, shape_rect: Some(self.viewport), shape_els: None, content: Rect::ZERO, begin });
    }

    /// An effect group's water this frame (Not in C#): each slot's ripple records appended to the
    /// frame's buffer (its header's offset moved there) or its picture named for the GPU half;
    /// counted into the stats.
    fn effect_water(&mut self, draw: &crate::rendering::WaterDraw) -> EffectWater {
        use crate::effects::effect_sizes::WaterSlot;
        use crate::water::water_source::{WaterTap, WaterTexture};
        let start = self.ripples.len() as u32;
        let mut slots = [WaterSlot::default(); 2];
        let mut textures: [Option<WaterBinding>; 2] = [None, None];
        for (i, tap) in [draw.refraction.as_ref(), draw.reflection.as_ref()].into_iter().enumerate() {
            match tap {
                None => {}
                Some(WaterTap::Ripples(p)) => {
                    let mut words = p.header.words;
                    words[0] = self.ripples.len() as u32;
                    self.ripples.extend_from_slice(&p.records);
                    self.stats.ripples_evaluated += words[1] as usize;
                    self.stats.ripples_skipped += p.skipped;
                    slots[i] = WaterSlot { kind: 1, words, place: p.header.place, scale: 0.0, bias: 0.0 };
                }
                Some(WaterTap::Picture { texture, place, scale, bias, .. }) => {
                    textures[i] = Some(match texture {
                        WaterTexture::Picture(picture) => {
                            let image = picture.image_data().clone();
                            self.images.push(crate::rendering::core::texture_ledger::FrameImage::picture(peniko::ImageBrush { image, sampler: peniko::ImageSampler::default() }));
                            WaterBinding::Image(self.images.len() as u32)
                        }
                        WaterTexture::Target(t) => WaterBinding::Target(t.clone()),
                        WaterTexture::Texture(v) => WaterBinding::Texture(v.clone()),
                    });
                    slots[i] = WaterSlot { kind: 2, words: [0; 4], place: *place, scale: *scale, bias: *bias };
                }
            }
        }
        EffectWater { slots, textures, ripples: start..self.ripples.len() as u32 }
    }

    /// A device quad for an effect pass (not clipped to the canvas: a coarse level's last texel
    /// may reach past it).
    fn fx_quad(&mut self, r: Rect, pass: u32) -> Range<u32> {
        let start = self.cover.len() as u32;
        let (x0, y0, x1, y1) = (r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32);
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y0), (x1, y1), (x0, y1)] {
            self.cover.push(Vertex { x, y, paint: pass });
        }
        start..self.cover.len() as u32
    }

    fn fx_pass(&mut self, data: FxPassData) -> u32 {
        self.fx_passes.push(data);
        (self.fx_passes.len() - 1) as u32
    }

    /// The valid texels of an effect texture written this effect (`Resolved`: the region).
    fn fx_valid(&self, texture: FxTexture, region: Rect) -> Rect {
        let (w, h) = (self.viewport.x1 as u32, self.viewport.y1 as u32);
        match texture {
            (FxRole::Resolved, _) => region,
            (_, level) => level_rect(region, level, w, h),
        }
    }

    /// One blur of `input` by `sigma` (device pixels) into `out`: at the level its size asks for,
    /// a box-filtered copy first when that level differs from the input's, then the horizontal and
    /// the vertical pass. Returns the texture holding the result.
    fn plan_blur(&mut self, steps: &mut Vec<FxStep>, region: Rect, input: FxTexture, sigma: f32, out: FxRole) -> FxTexture {
        let (w, h) = (self.viewport.x1 as u32, self.viewport.y1 as u32);
        let level = crate::effects::effect_sizes::level_for(sigma);
        let lr = level_rect(region, level, w, h);
        let device = Rect::new(lr.x0 * level as f64, lr.y0 * level as f64, lr.x1 * level as f64, lr.y1 * level as f64);
        let size = [w.div_ceil(level) as f32, h.div_ceil(level) as f32, 0.0, 0.0];
        let rect = |r: Rect| [r.x0 as f32, r.y0 as f32, r.x1 as f32, r.y1 as f32];
        let mut src = input;
        if input.1 != level {
            let dst = (FxRole::Down, level);
            let pass = self.fx_pass(FxPassData { v0: rect(self.fx_valid(input, region)), v1: [input.1 as f32, level as f32, 0.0, 0.0], v4: size, ..FxPassData::default() });
            let quad = self.fx_quad(device, pass);
            steps.push(FxStep::Down { src: input, dst, quad });
            src = dst;
        }
        let (kernel, radius) = crate::effects::effect_sizes::gaussian_kernel(sigma / level as f32);
        let offset = self.fx_kernels.len() as u32;
        self.fx_kernels.extend_from_slice(&kernel);
        let temp = (FxRole::Temp, level);
        let pass = self.fx_pass(FxPassData { v0: rect(self.fx_valid(src, region)), v1: [level as f32, level as f32, 1.0, 0.0], v4: size, u0: [offset, radius as u32, 0, 0], ..FxPassData::default() });
        let quad = self.fx_quad(device, pass);
        steps.push(FxStep::Blur { src, dst: temp, quad });
        let dst = (out, level);
        let pass = self.fx_pass(FxPassData { v0: rect(lr), v1: [level as f32, level as f32, 0.0, 1.0], v4: size, u0: [offset, radius as u32, 0, 0], ..FxPassData::default() });
        let quad = self.fx_quad(device, pass);
        steps.push(FxStep::Blur { src: temp, dst, quad });
        dst
    }

    /// Plans the passes of effect `k` (C#'s order: the content blurred; the glow and the shadow
    /// from its coverage) and returns the composite's pass index.
    fn plan_effect(&mut self, k: usize, alpha: f32) -> u32 {
        let (p, content, region) = (self.effects[k].params, self.effects[k].content, self.effects[k].region);
        let mut steps = Vec::new();
        let pass = self.fx_pass(FxPassData { v0: [content.x0 as f32, content.y0 as f32, content.x1 as f32, content.y1 as f32], v1: [1.0, 1.0, 0.0, 0.0], v4: [self.viewport.x1 as f32, self.viewport.y1 as f32, 0.0, 0.0], ..FxPassData::default() });
        let quad = self.fx_quad(region, pass);
        steps.push(FxStep::Resolve { quad });
        let mut input: FxTexture = (FxRole::Resolved, 1);
        if p.blur_sigma > 0.0 {
            input = self.plan_blur(&mut steps, region, input, p.blur_sigma, FxRole::Blur);
        }
        // A bloom (a layer's frame operation) blurs the content into the glow's texture; on a
        // layer with a glow the bloom is not drawn.
        let mut frame = p.frame.unwrap_or_default();
        if p.has_glow {
            frame.flags &= !8;
        }
        let bloom = frame.flags & 8 != 0 && frame.bloom_sigma > 0.0;
        let glow = if p.has_glow && p.glow_sigma > 0.0 {
            self.plan_blur(&mut steps, region, input, p.glow_sigma, FxRole::Glow)
        } else if bloom {
            self.plan_blur(&mut steps, region, input, frame.bloom_sigma, FxRole::Glow)
        } else {
            input
        };
        let shadow = if p.has_shadow && p.shadow_sigma > 0.0 { self.plan_blur(&mut steps, region, input, p.shadow_sigma, FxRole::Shadow) } else { input };
        let flags = (p.has_glow as u32) | ((p.has_shadow as u32) << 1);
        let mut data = FxPassData {
            v0: [region.x0 as f32, region.y0 as f32, region.x1 as f32, region.y1 as f32],
            v1: [input.1 as f32, glow.1 as f32, shadow.1 as f32, flags as f32],
            v2: p.glow_color,
            v3: p.shadow_color,
            v4: [p.shadow_offset[0], p.shadow_offset[1], p.glow_strength, alpha],
            u0: [0; 4],
            v5: [frame.tone[0], frame.tone[1], frame.gamma, frame.bloom_gain],
            v6: frame.vignette,
            v7: [frame.vignette_k[0], frame.vignette_k[1], frame.transition[0], frame.transition[1]],
            v8: frame.transition_color,
            v9: frame.extent,
            u1: [frame.flags, 0, 0, 0],
            ..crate::effects::surface_finish::finish_pass_data(p.finish.as_ref())
        };
        if let (Some(w), Some(ew)) = (p.water.as_ref(), self.effects[k].water.as_ref()) {
            crate::effects::effect_sizes::water_pass_data(&mut data, w, ew.slots[0], ew.slots[1]);
        }
        let composite = self.fx_pass(data);
        let e = &mut self.effects[k];
        e.steps = steps;
        e.reads = [input, glow, shadow];
        composite
    }

    fn pop(&mut self) {
        match self.stack.pop() {
            Some(Frame::Clip { cover }) => self.ops.push(Op::ClipPop { cover }),
            Some(Frame::Elided { clip }) => {
                if let Some(cover) = clip {
                    self.ops.push(Op::ClipPop { cover });
                }
                self.alpha_next = 1.0;
                self.elide_rect = None;
            }
            Some(Frame::Layer { mode, alpha, mix, compose, shape_rect, shape_els, content, begin }) => {
                self.paints.push(paint::composite(alpha, mix, compose));
                let mut paint = (self.paints.len() - 1) as u32;
                let (clip, area) = match (shape_rect, shape_els) {
                    (Some(r), _) => (None, r),
                    (None, els) => {
                        let (stencil, bounds) = self.device_fan(els.unwrap_or_default());
                        let cover = self.quad(bounds, 0).unwrap_or(empty_at(self.cover.len()));
                        (Some((stencil, cover)), bounds)
                    }
                };
                // Source-over (with any mix) changes only where the group drew; the other
                // composes change the whole shape.
                let src_over = compose == Compose::SrcOver as u32;
                let area = match mode {
                    // Effects reach beyond what the group drew (blurs, glows, offset shadows).
                    CompositeMode::Effect(k) => {
                        let has = content.width() > 0.0 && content.height() > 0.0;
                        let e = &mut self.effects[k as usize];
                        let mut region = if has { content.inflate(e.params.reach(), e.params.reach()).intersect(self.viewport) } else { Rect::ZERO };
                        // A reflection covers the mirrored part of the content (Not in C#).
                        if let (true, Some(r)) = (has, e.params.water.and_then(|w| w.reflection)) {
                            let mirrored = r.region(content).intersect(self.viewport);
                            if mirrored.width() > 0.0 && mirrored.height() > 0.0 {
                                region = if region.width() > 0.0 { region.union(mirrored) } else { mirrored };
                            }
                        }
                        // A layer in a transition (fade through a colour) covers its whole frame.
                        if let Some(f) = e.params.frame.filter(|f| (f.flags >> 8) & 15 != 0) {
                            region = region.union(Rect::new(f.extent[0] as f64, f.extent[1] as f64, f.extent[2] as f64, f.extent[3] as f64)).intersect(self.viewport);
                        }
                        let region = Rect::new(region.x0.floor(), region.y0.floor(), region.x1.ceil(), region.y1.ceil());
                        e.content = content;
                        e.region = region;
                        region
                    }
                    _ => if src_over { area.intersect(content) } else { area }.intersect(self.viewport),
                };
                let has_area = area.width() > 0.0 && area.height() > 0.0;
                if let (CompositeMode::Effect(k), true) = (mode, has_area) {
                    // The composite quad's paint field is its pass's index in the pass data.
                    paint = self.plan_effect(k as usize, alpha);
                    self.stats.effect_passes += self.effects[k as usize].steps.len();
                }
                let quad = self.quad(area, paint).unwrap_or(empty_at(self.cover.len()));
                if has_area {
                    self.grow_content(area);
                }
                if mode == CompositeMode::Mix && has_area {
                    self.stats.backdrop_copies += 1;
                }
                let has_content = content.width() > 0.0 && content.height() > 0.0;
                let cleared = match (has_content, has_area) {
                    (true, true) => content.union(area),
                    (true, false) => content,
                    (false, true) => area,
                    (false, false) => Rect::ZERO,
                };
                let clear = self.quad(cleared, 0).unwrap_or(0..0);
                if let Some(Op::LayerBegin { clear: c }) = self.ops.get_mut(begin) {
                    *c = clear;
                }
                self.ops.push(Op::LayerEnd { mode, clip, quad });
            }
            Some(Frame::Nothing) | None => {}
        }
        self.batch_boxes.clear();
    }

    /// The local-space fan of a fill or stroke, from the cache or made now.
    fn local_fan(&mut self, ordinal: usize, els: &[PathEl], key: u64, stroke: Option<&kurbo::Stroke>, step: i32, step_scale: f64) -> Option<(FanRef, Rect)> {
        let mut h = Hasher::new();
        h.word(key);
        h.word(step as u64);
        if let Some(s) = stroke {
            h.f64(s.width);
            h.word(s.join as u64);
            h.f64(s.miter_limit);
            h.word(s.start_cap as u64);
            h.word(s.end_cap as u64);
            h.f64(s.dash_offset);
            for d in s.dash_pattern.iter() {
                h.f64(*d);
            }
        } else {
            h.word(0xF111);
        }
        let k = h.finish();
        let frame = self.frame;
        if self.miss_streak.len() <= ordinal {
            self.miss_streak.resize(ordinal + 1, 0);
        }
        if let Some(entry) = self.cache.get_mut(&k) {
            entry.last_used = frame;
            self.stats.cache_hits += 1;
            self.miss_streak[ordinal] = 0;
            return if entry.fan.is_empty() { None } else { Some((FanRef::Cached(k), entry.bounds)) };
        }
        self.stats.cache_misses += 1;
        let store = self.miss_streak[ordinal] < 2;
        self.miss_streak[ordinal] = self.miss_streak[ordinal].saturating_add(1);
        let tolerance = TOLERANCE / step_scale;
        let mut out = if store { Vec::new() } else { std::mem::take(&mut self.scratch) };
        out.clear();
        let bounds = match stroke {
            Some(s) => {
                kurbo::stroke_with(els.iter().copied(), s, &StrokeOpts::default(), tolerance, &mut self.stroke_ctx);
                fan(&mut out, self.stroke_ctx.output().elements().iter().copied(), tolerance)
            }
            None => fan(&mut out, els.iter().copied(), tolerance),
        };
        let bounds = bounds.unwrap_or(Rect::ZERO);
        let empty = out.is_empty();
        let fan_ref = if store {
            self.cache.insert(k, CacheEntry { fan: out, bounds, last_used: frame });
            FanRef::Cached(k)
        } else {
            self.scratch = out;
            FanRef::Scratch
        };
        if empty {
            None
        } else {
            Some((fan_ref, bounds))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(&mut self, els: &[PathEl], key: u64, stroke: Option<&kurbo::Stroke>, even_odd: bool, transform: Affine, brush: &Brush, brush_transform: Option<Affine>, tint: Option<[f32; 4]>, special: Option<Special<'_>>) {
        self.stats.draws += 1;
        let mut alpha = std::mem::replace(&mut self.alpha_next, 1.0);
        let elide_rect = self.elide_rect.take();
        let scale = max_scale(transform);
        if !(scale > 1e-9) || paint::is_invisible(brush, alpha) {
            self.stats.culled += 1;
            return;
        }
        let (step_scale, step) = scale_step(scale);
        // A stroke narrower than a device pixel: one pixel wide, its alpha in proportion.
        let thin;
        let stroke = match stroke {
            Some(s) if s.width * scale < 1.0 => {
                alpha *= (s.width * scale) as f32;
                let mut t = s.clone();
                t.width = 1.0 / step_scale;
                thin = t;
                Some(&thin)
            }
            other => other,
        };
        let ordinal = self.stats.draws - 1;
        let Some((fan_ref, local)) = self.local_fan(ordinal, els, key, stroke, step, step_scale) else {
            self.stats.culled += 1;
            return;
        };
        // The cover: the transformed local bounds (a parallelogram), and its device box.
        let corners = [transform * Point::new(local.x0, local.y0), transform * Point::new(local.x1, local.y0), transform * Point::new(local.x1, local.y1), transform * Point::new(local.x0, local.y1)];
        let mut bbox = Rect::from_points(corners[0], corners[0]);
        for c in &corners[1..] {
            bbox = bbox.union_pt(*c);
        }
        let bbox = bbox.inflate(1.0, 1.0);
        if !overlaps(&bbox, &self.viewport) {
            self.stats.culled += 1;
            return;
        }
        // The paint.
        let to_device = transform * brush_transform.unwrap_or(Affine::IDENTITY);
        let mut image = 0u32;
        let paint = match brush {
            Brush::Solid(c) => Some(paint::solid(*c, alpha)),
            Brush::Gradient(g) => {
                let slot = self.ramps.slot(g, self.frame);
                match special {
                    Some(Special::Noise(n)) => paint::noise(n, to_device, alpha, slot),
                    _ => paint::gradient(g, to_device, alpha, slot),
                }
            }
            Brush::Image(img) => {
                use crate::rendering::core::texture_ledger::FrameImage;
                self.images.push(match special {
                    Some(Special::Owners(o, palette)) => FrameImage { brush: img.clone(), palette: palette.cloned(), palette_row: o.map_size.1 },
                    _ => FrameImage::picture(img.clone()),
                });
                image = self.images.len() as u32;
                match special {
                    Some(Special::Owners(o, _)) => paint::owners(img, o, to_device, alpha),
                    _ => paint::image(img, to_device, alpha, tint),
                }
            }
        };
        let Some(paint) = paint else {
            self.stats.culled += 1;
            return;
        };
        self.paints.push(paint);
        let paint = (self.paints.len() - 1) as u32;
        // An elided group whose rectangle does not hold the draw: clip by the rectangle.
        if let Some(r) = elide_rect {
            if !r.contains_rect(bbox) {
                let els = vec![PathEl::MoveTo(Point::new(r.x0, r.y0)), PathEl::LineTo(Point::new(r.x1, r.y0)), PathEl::LineTo(Point::new(r.x1, r.y1)), PathEl::LineTo(Point::new(r.x0, r.y1)), PathEl::ClosePath];
                let clip = self.clip_ops(els, false);
                if let Some(Frame::Elided { clip: c }) = self.stack.last_mut() {
                    *c = clip;
                }
                self.batch_boxes.clear();
            }
        }
        // The fan, transformed to device space.
        let s0 = self.stencil.len() as u32;
        let c = transform.as_coeffs();
        let (a, b, cc, d, e, f) = (c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32, c[4] as f32, c[5] as f32);
        let fan = match &fan_ref {
            FanRef::Cached(k) => &self.cache[k].fan,
            FanRef::Scratch => &self.scratch,
        };
        self.stencil.extend(fan.iter().map(|p| Vertex { x: a * p[0] + cc * p[1] + e, y: b * p[0] + d * p[1] + f, paint: 0 }));
        let s1 = self.stencil.len() as u32;
        let c0 = self.cover.len() as u32;
        for q in [corners[0], corners[1], corners[2], corners[0], corners[2], corners[3]] {
            self.cover.push(Vertex { x: q.x as f32, y: q.y as f32, paint });
        }
        let c1 = self.cover.len() as u32;
        self.grow_content(bbox.intersect(self.viewport));
        // Batch with the previous fill when nothing overlaps.
        let mut merged = false;
        if let Some(Op::Fill { stencil, cover, even_odd: eo, image: im }) = self.ops.last_mut() {
            if *eo == even_odd && *im == image && stencil.end == s0 && cover.end == c0 && !self.batch_boxes.is_empty() && self.batch_boxes.len() < MAX_BATCH && !self.batch_boxes.iter().any(|r| overlaps(r, &bbox)) {
                stencil.end = s1;
                cover.end = c1;
                merged = true;
            }
        }
        if !merged {
            self.batch_boxes.clear();
            self.ops.push(Op::Fill { stencil: s0..s1, cover: c0..c1, even_odd, image });
        }
        self.batch_boxes.push(bbox);
    }
}

impl Builder {
    /// Particles: one instance each (device centre and axes, linear premultiplied colour), ONE
    /// operation for the lot; culled when none reaches the canvas.
    fn draw_particles(&mut self, particles: &[crate::shapes::particles_shape::DrawnParticle], transform: Affine, softness: f32, alpha: f32) {
        self.stats.draws += 1;
        let c = transform.as_coeffs();
        let start = self.particles.len() as u32;
        let mut bbox: Option<Rect> = None;
        for p in particles {
            let r = p.size as f64 / 2.0;
            let center = transform * Point::new(p.x as f64, p.y as f64);
            let ax = [(c[0] * r) as f32, (c[1] * r) as f32];
            let ay = [(c[2] * r) as f32, (c[3] * r) as f32];
            let ex = ((c[0] * r).abs() + (c[2] * r).abs()) + 1.0;
            let ey = ((c[1] * r).abs() + (c[3] * r).abs()) + 1.0;
            let b = Rect::new(center.x - ex, center.y - ey, center.x + ex, center.y + ey);
            if !overlaps(&b, &self.viewport) {
                continue;
            }
            bbox = Some(bbox.map_or(b, |a| a.union(b)));
            let solid = paint::solid(p.color.to_peniko(1.0), alpha);
            self.particles.push(ParticleInstance { center: [center.x as f32, center.y as f32], axis_x: ax, axis_y: ay, softness, pad: 0.0, color: solid.color });
        }
        let end = self.particles.len() as u32;
        let Some(bbox) = bbox else {
            self.stats.culled += 1;
            return;
        };
        self.grow_content(bbox.intersect(self.viewport));
        self.batch_boxes.clear();
        self.ops.push(Op::Particles { instances: start..end });
    }
}

impl Builder {
    /// A stroke set: ONE operation and its draw block; the content box is the anchors' boxes
    /// through the element's and each anchor's transform (grown by what boil and flow can move a
    /// stroke, and by two device pixels); culled before the set's first stroke starts or when the
    /// box misses the canvas.
    pub(crate) fn draw_strokes(&mut self, draw: &crate::shapes::brush_strokes_shape::StrokeDraw, transform: Affine) {
        use crate::strokes::stroke_anchor::anchor_at;
        use crate::strokes::stroke_gpu::{draw_block_bytes, DRAW_HEADER_VEC4S, FLAG_BOIL, FLAG_FLOW, FLAG_MIRROR, FLAG_MORPH, FLAG_TIPS};
        self.stats.draws += 1;
        let set = &draw.set;
        // A transition draws every slot (each paints off, on or travels at the fraction).
        let instances = match &draw.morph {
            Some(m) => m.state.morph.len(),
            None => set.drawn_count(draw.time as f64),
        };
        if instances == 0 || !(max_scale(transform) > 1e-9) {
            self.stats.culled += 1;
            return;
        }
        let morph_to = draw.morph.as_ref().map(|m| m.state.morph.to());
        // How far boil and flow may carry a stroke beyond its whole box, set units.
        let reach = set.reach().max(morph_to.map_or(0.0, |t| t.reach())) as f64;
        let mut pad = 0.0;
        if let Some((b, _)) = &draw.boil {
            pad += b.offset as f64 * 1.5 + reach * b.size as f64;
        }
        if draw.boil.map_or(false, |(b, _)| b.turn != 0.0) || draw.flow.map_or(false, |f| f.turn != 0.0) {
            pad += reach;
        }
        let mut bbox: Option<Rect> = None;
        for (i, b) in set.anchor_bounds().iter().enumerate().chain(morph_to.into_iter().flat_map(|t| t.anchor_bounds().iter().enumerate())) {
            if b.width <= 0.0 && b.height <= 0.0 {
                continue;
            }
            let m = transform * anchor_at(&draw.anchors, i as u16);
            let r = Rect::new(b.x as f64 - pad, b.y as f64 - pad, b.right() as f64 + pad, b.bottom() as f64 + pad);
            let boxed = m.transform_rect_bbox(r);
            bbox = Some(bbox.map_or(boxed, |a| a.union(boxed)));
        }
        let Some(bbox) = bbox.map(|b| b.inflate(2.0, 2.0)) else {
            self.stats.culled += 1;
            return;
        };
        if !overlaps(&bbox, &self.viewport) {
            self.stats.culled += 1;
            return;
        }
        // The draw block (see strokes/stroke_gpu.rs for its layout).
        let offset = self.stroke_blocks.len() * 4;
        let f = |v: f64| (v as f32).to_bits();
        let c = transform.as_coeffs();
        let mut flags = 0;
        if draw.tips.is_some() {
            flags |= FLAG_TIPS;
        }
        if draw.mirror {
            flags |= FLAG_MIRROR;
        }
        let (epoch, boil) = match &draw.boil {
            Some((b, e)) => {
                flags |= FLAG_BOIL;
                (*e as i32 as u32, *b)
            }
            None => (0, crate::strokes::StrokeBoil::default()),
        };
        if draw.flow.is_some() {
            flags |= FLAG_FLOW;
        }
        if draw.morph.is_some() {
            flags |= FLAG_MORPH;
        }
        let (fraction, spread) = draw.morph.as_ref().map_or((0.0f32, 0.0f32), |m| (m.fraction, m.state.spread));
        let flow = draw.flow.unwrap_or_default();
        let anchors = draw.anchors.len().min(crate::strokes::stroke_gpu::MAX_ANCHORS);
        self.stroke_blocks.extend_from_slice(&[f(c[0]), f(c[1]), f(c[2]), f(c[3])]);
        self.stroke_blocks.extend_from_slice(&[f(c[4]), f(c[5]), draw.time.to_bits(), draw.opacity.to_bits()]);
        // The finished layer the set is on (the nearest enclosing effect group with a finish):
        // the set's height goes into that group's height pass, at the layer's impasto.
        let finished = self.stack.iter().rev().find_map(|f| match f {
            Frame::Layer { mode: CompositeMode::Effect(k), .. } => self.effects[*k as usize].params.finish.map(|fp| (*k, fp.impasto)),
            _ => None,
        });
        let impasto = finished.map_or(0.0, |(_, i)| i);
        // The picked colour: the source for the GPU half, the pickup into the block.
        let (source, pickup) = self.stroke_source(&draw.color);
        let words = &mut self.stroke_blocks;
        words.extend_from_slice(&[draw.head_softness.to_bits(), impasto.to_bits(), fraction.to_bits(), spread.to_bits()]);
        words.extend_from_slice(&[flags, anchors as u32, epoch, boil.seed]);
        words.extend_from_slice(&[boil.offset.to_bits(), ((boil.turn as f64).to_radians() as f32).to_bits(), boil.size.to_bits(), 0]);
        words.extend_from_slice(&[flow.frequency.to_bits(), flow.speed.to_bits(), ((flow.turn as f64).to_radians() as f32).to_bits(), 0]);
        words.extend_from_slice(&[flow.seed, 0, 0, 0]);
        words.extend_from_slice(&pickup);
        debug_assert_eq!(words.len() * 4 - offset, DRAW_HEADER_VEC4S * 16);
        for a in &draw.anchors[..anchors] {
            let k = a.as_coeffs();
            words.extend_from_slice(&[f(k[0]), f(k[1]), f(k[2]), f(k[3]), f(k[4]), f(k[5]), 0, 0]);
        }
        words.resize((offset + draw_block_bytes(anchors)) / 4, 0);
        self.stats.stroke_draws += 1;
        self.stats.strokes += instances;
        self.grow_content(bbox.intersect(self.viewport));
        self.batch_boxes.clear();
        // The checkpoint request: the strokes whole at the time compared as the shader compares
        // (never for a transition); the key comes once the frame is built.
        if draw.checkpoints_refused {
            self.stats.checkpoint_misses += 1;
        }
        let checkpoint = draw.checkpoints.filter(|_| draw.morph.is_none()).map(|options| StrokeAsk { options, whole: set.whole_count(draw.time) as u32, key: 0 });
        let morph = draw.morph.as_ref().map(|m| (m.state.clone(), if m.fraction < 0.5 { 0u8 } else { 1u8 }));
        self.stroke_ops.push(StrokeOp { set: set.clone(), tips: draw.tips.clone(), source, offset: offset as u32, instances: instances as u32, checkpoint, morph });
        let index = (self.stroke_ops.len() - 1) as u32;
        if let Some((k, _)) = finished {
            self.effects[k as usize].height_draws.push(index);
        }
        self.ops.push(Op::Strokes { draw: index });
    }

    /// The source a stroke draw picks its colours from (a picture joins the frame's pictures, so
    /// it is uploaded once as any picture) and the colour part of its block.
    fn stroke_source(&mut self, color: &crate::strokes::StrokeColor) -> (StrokeSource, Vec<u32>) {
        use crate::rendering::core::texture_ledger::FrameImage;
        use crate::strokes::stroke_color::SourceTexels;
        use crate::strokes::stroke_color_gpu::colour_words;
        use crate::strokes::{PictureEncoding, StrokeColor, StrokeColorSource};
        let StrokeColor::Picked(p) = color else {
            return (StrokeSource::None, colour_words(None));
        };
        let (source, texels) = match &p.source {
            StrokeColorSource::Picture(picture) => {
                let image = picture.image_data().clone();
                let texels = if image.alpha_type == peniko::ImageAlphaType::AlphaPremultiplied { SourceTexels::SrgbPremultiplied } else { SourceTexels::SrgbStraight };
                self.images.push(FrameImage::picture(peniko::ImageBrush { image, sampler: peniko::ImageSampler::default() }));
                (StrokeSource::Image(self.images.len() as u32), texels)
            }
            StrokeColorSource::Target(t) => (StrokeSource::Target(t.clone()), SourceTexels::SrgbPremultiplied),
            StrokeColorSource::Texture(e) => {
                let texels = match e.encoding {
                    PictureEncoding::SrgbEncoded => SourceTexels::SrgbStraight,
                    PictureEncoding::Linear => SourceTexels::LinearPremultiplied,
                };
                (StrokeSource::Texture(e.view.clone()), texels)
            }
        };
        (source, colour_words(Some((p, texels))))
    }
}

/// A 64-bit running hash of words (the checkpoint keys).
#[derive(Clone, Copy)]
struct KeyHash(u64);

impl KeyHash {
    fn word(&mut self, w: u64) {
        self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    fn words(&mut self, ws: &[u32]) {
        for pair in ws.chunks(2) {
            self.word(pair[0] as u64 | (pair.get(1).copied().unwrap_or(0) as u64) << 32);
        }
        self.word(ws.len() as u64);
    }

    fn finish(self) -> u64 {
        let mut z = self.0 ^ (self.0 >> 31);
        z = z.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        z ^ (z >> 29)
    }
}

/// The words of plain `#[repr(C)]` data of f32 / u32 fields.
fn as_words<T>(v: &[T]) -> &[u32] {
    // SAFETY: only called with #[repr(C)] plain-old-data structs of 4-byte f32/u32 fields
    // (Vertex, PaintData, ParticleInstance, FxPassData): no padding, 4-byte aligned.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u32, std::mem::size_of_val(v) / 4) }
}

impl Builder {
    /// The checkpoint keys of the frame's stroke draws that ask for them (see the module's text):
    /// one walk over the operations in the order the GPU half encodes them, hashing what each
    /// draws; a draw's key is its own identity over the hash of everything before it. Something
    /// whose pixels the CPU cannot name (a picture target's picture, an application's texture as
    /// a colour source) makes every later key unique to the frame.
    fn checkpoint_keys(&mut self) {
        if !self.stroke_ops.iter().any(|o| o.checkpoint.is_some()) {
            return;
        }
        let mut prior = KeyHash(0xcbf2_9ce4_8422_2325);
        prior.word(self.viewport.x1.to_bits() ^ self.viewport.y1.to_bits().rotate_left(17));
        self.hash_cover(&mut prior, &self.root_clear.clone());
        let mut volatile = false;
        let mut level = 0u64;
        let mut keys: Vec<(usize, u64)> = Vec::new();
        for op in &self.ops {
            match op {
                Op::Fill { stencil, cover, even_odd, image } => {
                    prior.word(1 | (*even_odd as u64) << 8);
                    self.hash_stencil(&mut prior, stencil);
                    self.hash_cover(&mut prior, cover);
                    if *image > 0 {
                        let im = &self.images[*image as usize - 1];
                        volatile |= crate::rendering::picture_target::PictureTarget::find(im.brush.image.data.id()).is_some();
                        let (id, width) = im.key();
                        prior.word(id);
                        prior.word(width as u64 | (im.palette_row as u64) << 32);
                        prior.word(im.palette_id().unwrap_or(0));
                    }
                }
                Op::ClipPush { stencil, cover, even_odd } => {
                    prior.word(2 | (*even_odd as u64) << 8);
                    self.hash_stencil(&mut prior, stencil);
                    self.hash_cover(&mut prior, cover);
                }
                Op::ClipPop { cover } => {
                    prior.word(3);
                    self.hash_cover(&mut prior, cover);
                }
                Op::LayerBegin { clear } => {
                    prior.word(4);
                    level += 1;
                    self.hash_cover(&mut prior, clear);
                }
                Op::LayerEnd { mode, clip, quad } => {
                    level = level.saturating_sub(1);
                    match mode {
                        CompositeMode::Compose(c) => prior.word(5 | (*c as u64) << 8),
                        CompositeMode::Mix => prior.word(6),
                        CompositeMode::Effect(k) => {
                            prior.word(7);
                            let e = &self.effects[*k as usize];
                            // Water (Not in C#): a ripple field's records are hashed (they move
                            // with the time); a picture by its key; a picture target's or an
                            // application's texture cannot be named by the CPU.
                            if let Some(w) = &e.water {
                                prior.words(as_words(&self.ripples[w.ripples.start as usize..w.ripples.end as usize]));
                                for t in w.textures.iter().flatten() {
                                    match t {
                                        WaterBinding::Image(i) => {
                                            let (id, width) = self.images[*i as usize - 1].key();
                                            prior.word(id);
                                            prior.word(width as u64);
                                        }
                                        WaterBinding::Target(_) | WaterBinding::Texture(_) => volatile = true,
                                    }
                                }
                            }
                            for step in &e.steps {
                                let q = match step {
                                    FxStep::Resolve { quad } | FxStep::Down { quad, .. } | FxStep::Blur { quad, .. } => quad,
                                };
                                self.hash_fx_quad(&mut prior, q);
                            }
                            self.hash_fx_quad(&mut prior, quad);
                        }
                    }
                    if let Some((stencil, cover)) = clip {
                        self.hash_stencil(&mut prior, stencil);
                        self.hash_cover(&mut prior, cover);
                    }
                    if !matches!(mode, CompositeMode::Effect(_)) {
                        self.hash_cover(&mut prior, quad);
                    }
                }
                Op::Particles { instances } => {
                    prior.word(8);
                    prior.words(as_words(&self.particles[instances.start as usize..instances.end as usize]));
                }
                Op::Strokes { draw } => {
                    let o = &self.stroke_ops[*draw as usize];
                    let block = self.stroke_block(o);
                    let mut own = KeyHash(0x243f_6a88_85a3_08d3);
                    own.word(o.set.id());
                    own.word(o.tips.as_ref().map_or(0, |t| t.id()));
                    let source = match &o.source {
                        StrokeSource::None => 0,
                        StrokeSource::Image(i) => self.images.get(*i as usize - 1).map_or(1, |im| im.key().0 ^ ((im.key().1 as u64) << 40)),
                        StrokeSource::Target(_) | StrokeSource::Texture(_) => {
                            volatile = true;
                            2
                        }
                    };
                    own.word(source);
                    own.word(o.morph.as_ref().map_or(0, |(m, v)| m.id << 1 | *v as u64));
                    // The block without its time (word 6): a whole stroke draws the same at any
                    // later time; the boil's number and the flow (words 14, 20-22) are part of it.
                    let mut timeless = block.to_vec();
                    timeless[6] = 0;
                    own.words(&timeless);
                    if o.checkpoint.is_some() {
                        let mut key = own;
                        key.word(prior.0);
                        key.word(level);
                        if volatile {
                            key.word(self.frame ^ 0x5eed_0000_0000_0000);
                        }
                        keys.push((*draw as usize, key.finish()));
                    }
                    // What it draws, for the draws after it: its key and how far it is drawn
                    // (its time too, unless every stroke it draws is whole and none moves).
                    prior.word(9);
                    prior.0 ^= own.finish();
                    prior.word(o.instances as u64);
                    let time = f32::from_bits(block[6]);
                    let moves = block[12] & (crate::strokes::stroke_gpu::FLAG_BOIL | crate::strokes::stroke_gpu::FLAG_FLOW | crate::strokes::stroke_gpu::FLAG_MORPH) != 0;
                    if moves || (o.set.whole_count(time) as u32) < o.instances {
                        prior.word(block[6] as u64);
                    }
                }
            }
        }
        for (d, key) in keys {
            if let Some(a) = self.stroke_ops[d].checkpoint.as_mut() {
                a.key = key;
            }
        }
    }

    /// A stroke draw's block words (its header and its anchors).
    fn stroke_block(&self, o: &StrokeOp) -> &[u32] {
        let start = (o.offset as usize / 4).min(self.stroke_blocks.len());
        let anchors = self.stroke_blocks.get(start + 13).copied().unwrap_or(0) as usize;
        let end = (start + crate::strokes::stroke_gpu::draw_block_bytes(anchors) / 4).min(self.stroke_blocks.len());
        &self.stroke_blocks[start..end]
    }

    fn hash_stencil(&self, h: &mut KeyHash, r: &Range<u32>) {
        h.words(as_words(&self.stencil[r.start as usize..r.end as usize]));
    }

    /// Cover vertices and the paints they name (with the upload of each paint's ramp slot).
    fn hash_cover(&self, h: &mut KeyHash, r: &Range<u32>) {
        let verts = &self.cover[r.start as usize..(r.end as usize).min(self.cover.len())];
        h.words(as_words(verts));
        let mut last = u32::MAX;
        for v in verts {
            if v.paint != last {
                last = v.paint;
                if let Some(p) = self.paints.get(v.paint as usize) {
                    h.words(as_words(std::slice::from_ref(p)));
                    h.word(self.ramp_serials.get(&p.info[2]).copied().unwrap_or(0));
                }
            }
        }
    }

    /// An effect pass's quad: its vertices (their paint field is the pass index), the pass's data
    /// and the kernel it reads.
    fn hash_fx_quad(&self, h: &mut KeyHash, r: &Range<u32>) {
        let verts = &self.cover[r.start as usize..(r.end as usize).min(self.cover.len())];
        h.words(as_words(verts));
        if let Some(pass) = verts.first().and_then(|v| self.fx_passes.get(v.paint as usize)) {
            h.words(as_words(std::slice::from_ref(pass)));
            let (offset, radius) = (pass.u0[0] as usize, pass.u0[1] as usize);
            if radius > 0 {
                let end = (offset + 2 * radius + 2).min(self.fx_kernels.len());
                h.words(as_words(&self.fx_kernels[offset.min(end)..end]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Circle, Shape};
    use peniko::color::{AlphaColor, Srgb};

    fn red() -> Brush {
        Brush::Solid(AlphaColor::<Srgb>::new([1.0, 0.0, 0.0, 1.0]))
    }

    #[test]
    fn separate_shapes_share_one_batch_and_overlapping_ones_do_not() {
        let mut scene = VectorScene::new();
        for i in 0..4 {
            scene.fill(Fill::NonZero, Affine::translate((i as f64 * 100.0, 0.0)), &red(), None, &Circle::new((40.0, 40.0), 30.0));
        }
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((50.0, 50.0), 30.0));
        let mut b = Builder::default();
        b.build(&scene, 800, 600);
        assert_eq!(b.stats.fill_batches, 2);
        assert_eq!(b.stats.draws, 5);
    }

    #[test]
    fn a_moving_shape_is_flattened_once() {
        let mut b = Builder::default();
        let mut scene = VectorScene::new();
        for frame in 0..5 {
            scene.reset();
            let t = Affine::translate((frame as f64 * 7.0, 3.0)) * Affine::rotate(frame as f64 * 0.2);
            scene.stroke(&kurbo::Stroke::new(4.0), t, &red(), None, &Circle::new((200.0, 200.0), 50.0));
            b.build(&scene, 800, 600);
        }
        assert_eq!(b.stats.cache_hits, 1);
        assert_eq!(b.cache.len(), 1);
    }

    #[test]
    fn geometry_that_changes_every_frame_stops_being_stored() {
        let mut b = Builder::default();
        let mut scene = VectorScene::new();
        for frame in 0..6 {
            scene.reset();
            scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((200.0, 200.0), 50.0 + frame as f64));
            b.build(&scene, 800, 600);
            assert_eq!(b.stats.cache_misses, 1);
            assert_eq!(b.ops.len(), 1);
        }
        assert_eq!(b.cache.len(), 2);
    }

    #[test]
    fn full_alpha_groups_and_canvas_clips_cost_nothing() {
        let mut scene = VectorScene::new();
        let canvas = kurbo::Rect::new(0.0, 0.0, 800.0, 600.0);
        scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &canvas);
        scene.push_clip_layer(Affine::IDENTITY, &canvas);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((50.0, 50.0), 30.0));
        scene.pop_layer();
        scene.pop_layer();
        let mut b = Builder::default();
        b.build(&scene, 800, 600);
        assert_eq!(b.ops.len(), 1);
        assert_eq!(b.stats.offscreen, 0);
    }

    #[test]
    fn a_translucent_group_of_two_draws_is_offscreen_and_a_mix_mode_reads_the_backdrop() {
        let mut scene = VectorScene::new();
        let canvas = kurbo::Rect::new(0.0, 0.0, 800.0, 600.0);
        scene.push_layer(Mix::Multiply, 0.5, Affine::IDENTITY, &canvas);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((50.0, 50.0), 30.0));
        scene.fill(Fill::EvenOdd, Affine::IDENTITY, &red(), None, &Circle::new((60.0, 50.0), 30.0));
        scene.pop_layer();
        let mut b = Builder::default();
        b.build(&scene, 800, 600);
        assert_eq!(b.stats.offscreen, 1);
        assert_eq!(b.stats.backdrop_copies, 1);
        assert_eq!(b.levels_needed, 2);
        assert!(matches!(b.ops.last(), Some(Op::LayerEnd { mode: CompositeMode::Mix, .. })));
    }

    #[test]
    fn a_full_alpha_group_holding_a_compose_group_stays_isolated() {
        let mut scene = VectorScene::new();
        let canvas = kurbo::Rect::new(0.0, 0.0, 800.0, 600.0);
        scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &canvas);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((50.0, 50.0), 30.0));
        scene.push_layer(Compose::SrcIn, 1.0, Affine::IDENTITY, &canvas);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((70.0, 50.0), 30.0));
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &Circle::new((90.0, 50.0), 30.0));
        scene.pop_layer();
        scene.pop_layer();
        let mut b = Builder::default();
        b.build(&scene, 800, 600);
        assert_eq!(b.stats.offscreen, 2);
        assert_eq!(b.levels_needed, 3);
    }

    #[test]
    fn thin_strokes_are_widened_with_less_alpha_and_off_canvas_draws_are_culled() {
        let mut scene = VectorScene::new();
        scene.stroke(&kurbo::Stroke::new(0.5), Affine::IDENTITY, &red(), None, &kurbo::Line::new((10.0, 10.0), (100.0, 10.0)));
        scene.fill(Fill::NonZero, Affine::translate((5000.0, 0.0)), &red(), None, &Circle::new((0.0, 0.0), 5.0));
        let mut b = Builder::default();
        b.build(&scene, 800, 600);
        assert_eq!(b.stats.culled, 1);
        assert_eq!(b.paints[1].color[3], 0.5);
    }

    #[test]
    fn a_group_with_effects_is_planned_as_a_resolve_blurs_and_a_composite_over_its_reach() {
        //Arrange
        let mut scene = VectorScene::new();
        let effects = crate::effects::DrawingEffects::glow(20.0, crate::Color::WHITE, 1.0).unwrap().with_shadow(crate::PointF::new(4.0, 4.0), 0.0, crate::Color::BLACK).unwrap();
        let params = EffectParams::new(&effects, Affine::IDENTITY, 0.0).unwrap();
        scene.push_effects(params, 0.5);
        scene.fill(Fill::NonZero, Affine::IDENTITY, &red(), None, &kurbo::Rect::new(100.0, 100.0, 200.0, 150.0));
        scene.pop_layer();
        let mut b = Builder::default();
        //Act
        b.build(&scene, 800, 600);
        //Assert
        assert_eq!(b.stats.effects, 1);
        let e = &b.effects[0];
        // glow sigma 10 px: level 8 (at most 8 taps a side), a down pass, two blur passes; the
        // hard shadow reads the resolved pixels themselves.
        assert_eq!(e.steps.len(), 4);
        assert!(matches!(e.steps[0], FxStep::Resolve { .. }));
        assert!(matches!(e.steps[1], FxStep::Down { src: (FxRole::Resolved, 1), dst: (FxRole::Down, 8), .. }));
        assert_eq!(e.reads, [(FxRole::Resolved, 1), (FxRole::Glow, 8), (FxRole::Resolved, 1)]);
        // reach: max(reach(10) = 41, 0 + 4) = 41 around the content.
        assert_eq!(e.region, kurbo::Rect::new(58.0, 58.0, 242.0, 192.0), "the content box (the draw grown by a pixel) grown by the reach");
        let composite = b.fx_passes.last().unwrap();
        assert_eq!(composite.v4, [4.0, 4.0, 1.0, 0.5]);
        assert_eq!(composite.v1[3], 3.0);
        assert_eq!(b.fx_kernels.len(), 11, "sigma 1.25 at level 8: radius 5");
        assert!(matches!(b.ops.last(), Some(Op::LayerEnd { mode: CompositeMode::Effect(0), .. })));
        assert_eq!(level_rect(e.region, 2, 800, 600), kurbo::Rect::new(28.0, 28.0, 122.0, 97.0));
        assert_eq!(level_rect(e.region, 8, 800, 600), kurbo::Rect::new(6.0, 6.0, 32.0, 25.0));
    }

    #[test]
    fn the_scale_step_is_never_coarser_than_the_scale() {
        for s in [0.3, 1.0, 1.1, 2.0, 7.5] {
            let (stepped, _) = scale_step(s);
            assert!(stepped >= s - 1e-12 && stepped < s * 1.19);
        }
        assert!(Circle::new((0.0, 0.0), 1.0).path_elements(0.1).count() > 1);
    }
}
