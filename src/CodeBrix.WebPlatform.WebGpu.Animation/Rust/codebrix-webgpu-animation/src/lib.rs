//! # codebrix-webgpu-animation
//!
//! Real-time 2D animation for CodeBrix.WebPlatform frontends, with the names and concepts of
//! the C# library CodeBrix.Imaging.Drawing.NoSkia: a [`DrawingSession`] of calibrated units with
//! [`DrawingLayer`](models::DrawingLayer)s of shapes (rectangles with rounded corners, circles,
//! ellipses, lines, arrows, polylines, paths, [groups](shapes::DrawingGroup) and
//! [animated shapes](shapes::AnimatedShape); outlined or filled with colours or
//! [gradient fills](fills)), single lines of [`text`] ([`TextShape`](shapes::TextShape)) in
//! fonts registered from their BYTES (never a system font), session and layer fades, THE
//! ANIMATION MODEL - [keyframe tracks](animation::KeyframeTrack) bound to element, layer and
//! [camera](models::DrawingCamera) properties, [transforms](models::DrawingTransform), time
//! windows, [`layout`] - and PATHS (measuring, trims for draw-on, motion along a path,
//! [morphs](paths::PathMorph)); PICTURES and SPRITES from decoded pixels or PNG and JPEG files
//! ([`ImageSource`](images::ImageSource), [`ImageShape`](shapes::ImageShape),
//! [`SpriteSheet`](images::SpriteSheet), [`SpriteShape`](shapes::SpriteShape) - the frame a
//! function of the session time); MASKS ([`DrawingMask`](effects::DrawingMask): inside or outside
//! a path, a layer matte) and EFFECTS ([`DrawingEffects`](effects::DrawingEffects): blur, glow,
//! drop shadow, made on the GPU, their parameters bindable to tracks); [`Easing`](animation::Easing) curves,
//! [`ColorRamp`](animation::ColorRamp)s, [`AnimationTimeline`](animation::AnimationTimeline)s,
//! the [`PlaybackController`](animation::PlaybackController) and the
//! [`DebugOverlay`](rendering::DebugOverlay).
//!
//! TEXT BLOCKS laid out by NoSkia's line breaker ([`TextLayoutEngine`](text::TextLayoutEngine),
//! [`TextBlockShape`](shapes::TextBlockShape)) with reveals, staggers and text along a path;
//! [`captions`] from SRT and WebVTT text, against the frame by default; PATH COMBINING and PATH
//! QUERIES ([`PathGeometry`](paths::PathGeometry)); combined bindings, rolling and trains along
//! paths, [`formations`], layer visibility and [`safe_areas`] guides with their overlay.
//!
//! SVG DOCUMENTS ([`svg`]): a document's text or bytes turned into a scene of parts (through
//! usvg, its text laid out into paths with fonts the application registers from their bytes),
//! moved, turned, faded and trimmed per part under program control, and the document's own SMIL
//! and CSS animations played for the time passed in; drawn as a session element
//! ([`DrawingSession::draw_svg`]).
//!
//! LOTTIE animations ([`lottie`]): a Lottie document's JSON read through velato into a
//! [`DrawingLottie`](lottie::DrawingLottie), played as a function of the session time (loop,
//! ping-pong, hold, speed, a range or a marker) as an element of a session
//! ([`DrawingSession::draw_lottie`]), with its markers as the clock's named points and a report of
//! what is not drawn.
//!
//! RENDER-TIME LOOKS (Not in C#), set on [`SessionRenderOptions`] and never changing the scene:
//! a whole session's [blend mode](SessionBlendMode) and opacity, its [paint](SessionPaint) - as
//! authored, a [line drawing](LinePaint) or flat colours - with effects on or off and layers kept
//! as authored; a [`SessionWipe`] with a soft edge (a straight line at any angle, or an iris)
//! between any two renders. Strokes whose width is
//! [fixed in the drawing's units](paths::StrokeWidthMode) (SVG's
//! `vector-effect="non-scaling-stroke"` too), and [sketched paths](paths::SketchOptions) that
//! can "boil" with the clock time.
//!
//! A frame is a FUNCTION OF THE TIME: set the session's time from the frame
//! ([`DrawingSession::set_time`]) and its bindings, animated shapes and time windows follow.
//!
//! It draws on the GPU with its own renderer (stencil-then-cover with 4x MSAA, on kurbo, with
//! blend, gradient and style pieces taken from vello) as a LAYER of the codebrix-webgpu canvas
//! host: [`AnimationLayer`](rendering::AnimationLayer) draws over what the layers below drew -
//! a 3D scene of the Graphics package, for example - into the frame it is handed.
//!
//! PIXEL PLANES ([`planes`]: float planes and images with filters, blends, transitions, noise,
//! particles and motion blur, computed on the CPU - one thread - with NoSkia's values), with GPU
//! versions of what a real-time scene needs per frame: whole-frame operations over a layer
//! ([`DrawingFrameOperations`](effects::DrawingFrameOperations)), transitions between layers
//! ([`DrawingSession::set_layer_transition`]), noise drawn per pixel
//! ([`NoiseShape`](shapes::NoiseShape)) and particles in one instanced draw
//! ([`ParticlesShape`](shapes::ParticlesShape)). ANIMATED BRUSH STROKES and the painterly playback
//! ([`painting`]: brushstrokes that can be painted on, ownership maps by the hard-edged pixel rule
//! with colour pickup, brush textures and seams, keyframe players; hard-edged masks), computed on
//! the CPU - one thread - with NoSkia's values, ownership maps coloured on the GPU
//! ([`OwnershipShape`](shapes::OwnershipShape)).
//!
//! STROKE SETS (Not in C#, [`strokes`]): tens of thousands of brushstrokes as records drawn by ONE
//! instanced GPU draw ([`BrushStrokesShape`](shapes::BrushStrokesShape)), painted on, anchored to
//! moving elements, boiling and flowing with the time, with brush tips; planned from pictures by
//! direction fields, stroke layouts and painting timelines with colour rules and perceptual
//! mixing ([`painting`]); a still painting drawn from its checkpoints (pixel for pixel the same
//! frame), and stroke-level transitions between two sets. LIVE COLOUR SOURCES AND THE SURFACE
//! FINISH: stroke sets coloured every frame on the GPU from a picture, another session drawn into
//! a [`PictureTarget`](rendering::PictureTarget), or an application's texture (a 3D render); a
//! layer's canvas weave and paint thickness, lit
//! ([`SurfaceFinish`](effects::SurfaceFinish)).
//!
//! WATER (Not in C#, [`water`]): a [`RippleField`](water::RippleField) - rings from drops and
//! seeded rain spreading and fading over a swell, inside a rectangle whose edges can reflect
//! them - as a FUNCTION OF THE TIME (seeking shows exactly the water playing shows), with its
//! height, slope and focus readable on the CPU; a layer's or a whole session's picture seen
//! THROUGH water ([`WaterRefraction`](water::WaterRefraction): bent by the slope, shaded, with
//! glints and a caustic light; the ripples evaluated per pixel on the GPU) and a layer MIRRORED
//! below a waterline, broken up by the water and fading
//! ([`WaterReflection`](water::WaterReflection)) - both drawn from the layer's own picture, the
//! water a ripple field or any height picture (decoded pixels, a picture target, an application's
//! texture such as the Graphics package's simulated wave field); and the
//! [caustics](planes::DrawingNoiseKind::Caustics) noise kind, the light net of a pool floor, the
//! same pattern on the CPU and drawn on the GPU.
//!
//! Nothing in it talks to a backend or reads a file: fonts are bytes, and pictures are DECODED
//! pixels the application hands over (from its backend in the picture-pixels form, or made in the
//! frontend) or the bytes of a PNG or JPEG FILE, which the crate decodes through codebrix-webgpu
//! (PNG and JPEG are the only file formats it reads; a JPEG's EXIF orientation is not applied);
//! caption text, SVG documents and Lottie JSON are text or bytes the application hands over too.
//!
//! THE RULE OF THIS CRATE: nothing it DRAWS depends on a clock. Time comes from the frame
//! (`frame.info.time`, `frame.info.delta`) - the host's ONE clock, which every layer shares, can
//! be paused, and carries named points in time. The one clock reading in the crate is a
//! MEASUREMENT: the CPU milliseconds of a frame in [`RenderStats`](rendering::RenderStats), which
//! nothing drawn ever reads.
//!
//! C# names are kept for types; members are snake_case with the same argument order; C# default
//! arguments are option structs with `Default`; where C# throws, Rust returns a `Result` or an
//! `Option` whose error says what to do.

#![warn(missing_docs)]

pub mod animation;
pub mod captions;
mod color;
mod drawing_session;
mod drawing_session_blend;
mod drawing_session_paint;
mod drawing_session_wipe;
#[cfg(test)]
mod drawing_session_looks_tests;
mod drawing_session_options;
mod drawing_session_painting;
mod drawing_session_transitions;
pub mod effects;
pub mod fills;
pub mod formations;
mod geometry;
pub mod images;
pub mod layout;
pub mod lottie;
mod mask_meaning;
pub mod models;
pub mod painting;
pub mod paths;
pub mod planes;
pub mod rendering;
pub mod safe_areas;
pub mod shapes;
pub mod strokes;
pub mod svg;
pub mod text;
pub mod water;

pub use color::Color;
pub use drawing_session::{DrawingSession, SessionRenderOptions};
pub use drawing_session_blend::SessionBlendMode;
pub use drawing_session_paint::{LinePaint, SessionPaint};
pub use drawing_session_wipe::{SessionWipe, WipeEdge};
pub use drawing_session_options::DrawingSessionOptions;
pub use geometry::{PointF, Rectangle, RectangleF};
pub use mask_meaning::MaskMeaning;

/// kurbo (curves, shapes, transforms) as this crate was built with it.
pub use kurbo;
/// peniko (brushes, gradients, images, blend modes, fill rules) as this crate was built with it.
pub use peniko;
