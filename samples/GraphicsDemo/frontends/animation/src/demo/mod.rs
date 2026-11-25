//! The demo's scene catalogue (AnimationDemoApp's Shared/Demo): the DemoScene contract, the
//! colours every scene uses, the header and label helpers (DemoScene's CreateSession / AddHeader /
//! Label), the catalogue with its timeline, and the scenes. First come the ports of the C# demo's
//! scenes, in its order, with the pixel-plane and painterly scenes, "shapes" (the renderer's
//! feature sheet) and "missing-characters" (the three missing-character modes) among them; then
//! the SVG scenes ("svg-parts" is the C# scene SvgPartsScene; "svg-smil", "svg-css" and
//! "svg-features" are the sample's own: documents that animate themselves, and what is drawn);
//! then the sample's own Lottie scenes (NoSkia has no Lottie); last, the feature scenes of the
//! second GO (render-time looks, sketching, colours-only uploads, painterly stroke sets, planning
//! and colour, live colour pickup, the surface finish, the checkpoint cache of a still painting,
//! stroke-level transitions, and water: ripples, a reflection, the painted pond and a simulated pond). REGISTRATIONS below is the one list. Scenes draw everything - their text included - through codebrix-webgpu-animation, in the
//! fonts of ONE FontRegistry that the page fills with the bytes the backend serves; their
//! pictures, caption text, SVG documents and Lottie files are resources of the backend too
//! (pictures.rs, captions.rs, svg_files.rs, lottie_files.rs, lottie_images.rs). (The page
//! `?scene=on-the-page` is on_page.rs, not part of the catalogue.)

pub mod additive;
pub mod brush_strokes;
pub mod brush_texture;
pub mod brushwork;
pub mod camera;
pub mod captions;
pub mod checkpoints;
pub mod clip_mask;
pub mod colour_pickup;
pub mod colour_rules;
pub mod controls;
pub mod direction_fields;
pub mod draw_on;
pub mod easing;
pub mod effects;
pub mod effects_follow;
pub mod fade;
pub mod fixed_width;
pub mod formation_heap;
pub mod formation_spiral;
pub mod gradients;
pub mod hard_edges;
pub mod images;
pub mod kerning;
pub mod keyframe_playback;
pub mod keyframes;
pub mod kit;
pub mod layer_visibility;
pub mod layout;
pub mod light;
pub mod live_pickup;
pub mod lottie_features;
pub mod lottie_files;
pub mod lottie_images;
pub mod missing;
pub mod morph;
pub mod motion_blur;
pub mod noise;
pub mod motion_path;
pub mod on_page;
pub mod outlined_text;
pub mod ownership;
pub mod ownership_colours;
pub mod page;
pub mod paint_override;
pub mod painting_timeline;
pub mod painted;
pub mod painted_pond;
pub mod paragraph;
pub mod particles;
pub mod path_combine;
pub mod path_queries;
pub mod pictures;
pub mod playback;
pub mod procedural;
pub mod ramps;
pub mod rolling_text;
pub mod safe_areas;
pub mod session_blend;
pub mod shapes;
pub mod simulated_pond;
pub mod sketch;
pub mod sprites;
pub mod surface_finish;
pub mod stroke_sets;
pub mod stroke_transition;
pub mod svg_css;
pub mod svg_features;
pub mod svg_files;
pub mod svg_parts;
pub mod svg_smil;
pub mod text;
pub mod time_keys;
pub mod timeline;
pub mod transforms;
pub mod transitions;
pub mod travelling_strokes;
pub mod typewriter;
pub mod water_reflection;
pub mod water_ripples;
pub mod wipe;

use codebrix_webgpu::CanvasInput;
use codebrix_webgpu_animation::animation::{AnimationTimeline, EasingKind};
use codebrix_webgpu_animation::models::DrawingLayer;
use codebrix_webgpu_animation::rendering::{RenderStats, VectorScene};
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, RectangleShape, RectangleShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};

pub use controls::{Control, Readout, Settings};

/// The frame the scenes are written for (AnimationDemoApp's DemoFrame), calibrated units.
pub const FRAME_WIDTH: u32 = 1280;
/// The frame's height.
pub const FRAME_HEIGHT: u32 = 720;
/// The frame's background.
pub const BACKDROP: Color = Color::from_rgb(18, 20, 30);
/// Bright text and lines.
pub const INK: Color = Color::from_rgb(236, 236, 244);
/// Quiet text and lines.
pub const DIM: Color = Color::from_rgb(132, 138, 164);
/// The accent used for moving parts.
pub const ACCENT: Color = Color::from_rgb(255, 176, 72);
/// The top of the area below the header.
pub const CONTENT_TOP: f32 = 96.0;

/// What a sample point of the "shapes" scene should show in the layer's own pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Expect {
    /// Drawn (alpha above 200 of 255).
    Drawn,
    /// Not drawn (alpha below 30; the scene's backdrop is left out under these points).
    Empty,
    /// Half covered (alpha between 100 and 160 of 255).
    Translucent,
    /// A colour different from the sample point `index` of the same list (some channel by more
    /// than 12 of 255).
    Differs(usize),
    /// Some pixel of the box (width, height in frame units, from the point) drawn (alpha above
    /// 200 of 255).
    InkInBox(f32, f32),
    /// No pixel of the box drawn (every alpha below 30 of 255).
    NoInkInBox(f32, f32),
}

/// A sample point: what, where (frame units), and what it should show.
#[derive(Clone, Debug, PartialEq)]
pub struct SamplePoint {
    pub name: &'static str,
    pub x: f32,
    pub y: f32,
    pub expect: Expect,
}

/// One scene of the demo (AnimationDemoApp's DemoScene).
pub trait DemoScene {
    /// The short name (the address's ?scene=).
    fn name(&self) -> &'static str;
    /// The title, shown in the header.
    fn title(&self) -> &'static str;
    /// The one-line description under the title.
    fn description(&self) -> &'static str;
    /// How long it plays, seconds.
    fn duration_seconds(&self) -> f64 {
        8.0
    }
    /// Records the scene at a time within it into the frame's scene, for a canvas of this size.
    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32);
    /// The sample points a run-check reads (the "shapes" scene).
    fn checks(&self) -> Vec<SamplePoint> {
        Vec::new()
    }
    /// Named points in time of the scene's own (name, seconds into the scene) - a Lottie
    /// animation's markers - which the page defines on the host's clock as "<scene>/<name>".
    fn time_points(&self) -> Vec<(String, f64)> {
        Vec::new()
    }
    /// The controls the page shows under the canvas while this scene is shown (the feature
    /// scenes of the second GO; most scenes have none).
    fn controls(&self) -> &'static [Control] {
        &[]
    }
    /// Takes the values of the scene's controls (called by the page after every change).
    fn apply_settings(&mut self, _settings: &Settings) {}
    /// The short legend the page shows beside the controls: what to look at.
    fn legend(&self) -> &'static [&'static str] {
        &[]
    }
    /// The scene's read-outs after the frame just recorded.
    fn readouts(&self) -> Vec<Readout> {
        Vec::new()
    }
    /// A primary-button press (a click, a finger, a pen's tip) on the canvas at a point of the
    /// frame (calibrated units, the frame being 1280 x 720), handed to the scene shown; true when
    /// the scene used it (the water scenes drop a stone there). Most scenes take no pointer input.
    fn pointer_down(&mut self, _x: f32, _y: f32) -> bool {
        false
    }
    /// Every pointer event on the canvas (down, move, up), with the map from the canvas's CSS
    /// pixels to the frame's units, handed to the scene shown (the simulated pond stirs its wave
    /// field with them). Most scenes take no pointer input.
    fn pointer_input(&mut self, _input: &CanvasInput, _to_frame: &dyn Fn(f64, f64) -> (f32, f32)) {}
    /// The renderer's statistics of the frame just drawn (the water scenes show the ripples it
    /// evaluated and skipped among their read-outs).
    fn see_stats(&mut self, _stats: &RenderStats) {}
}

/// A session for part of a frame (or a whole one), sharing the demo's fonts, with opaque layers
/// (DemoScene.CreateSession).
pub fn create_session(width: u32, height: u32, background: Color, fonts: &FontRegistry) -> DrawingSession {
    DrawingSession::new(DrawingSessionOptions { calibration_size: (width, height), layer_opacity: 255, background_fill_color: background, fonts: Some(fonts.clone()), ..DrawingSessionOptions::default() })
}

/// A whole-frame session with the scene's header on a first "Header" layer
/// (DemoScene.CreateFrameSession); its active layer is the header layer, so add a layer next.
pub fn create_frame_session(fonts: &FontRegistry, title: &str, description: &str) -> DrawingSession {
    let mut session = create_session(FRAME_WIDTH, FRAME_HEIGHT, BACKDROP, fonts);
    add_header(&mut session, title, description, false);
    session
}

/// The scene's header - title, description and rule - as a new "Header" layer
/// (DemoScene.AddHeader). `opaque_band` puts the backdrop behind it, for a header above layers
/// that draw there.
pub fn add_header(session: &mut DrawingSession, title: &str, description: &str, opaque_band: bool) {
    let fonts = session.fonts().clone();
    let header = session.add_layer("Header", INK);
    if opaque_band {
        header.add_shape(RectangleShape::new(0.0, 0.0, FRAME_WIDTH as f32, CONTENT_TOP, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(BACKDROP), is_filled: true, ..Default::default() }).unwrap());
    }
    header.add_shape(TextShape::new(title, 32.0, 46.0, 30.0, &fonts, TextShapeOptions { color: Some(INK), ..TextShapeOptions::default() }).unwrap());
    header.add_shape(TextShape::new(description, 33.0, 74.0, 15.0, &fonts, TextShapeOptions { color: Some(DIM), ..TextShapeOptions::default() }).unwrap());
    header.add_shape(LineShape::new(32.0, 86.0, FRAME_WIDTH as f32 - 32.0, 86.0, LineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 90)) }).unwrap());
}

/// A single-line label on a layer, its anchor on the baseline (DemoScene.Label).
#[allow(clippy::too_many_arguments)]
pub fn label(layer: &mut DrawingLayer, fonts: &FontRegistry, text: &str, x: f32, y: f32, size: f32, color: Color, align: TextAlign) {
    layer.add_shape(TextShape::new(text, x, y, size, fonts, TextShapeOptions { color: Some(color), align, ..TextShapeOptions::default() }).unwrap());
}

/// The scenes in order, each with its start on the catalogue's timeline.
pub struct SceneCatalog {
    pub scenes: Vec<Box<dyn DemoScene>>,
    starts: Vec<f64>,
    pub total_seconds: f64,
    pub timeline: AnimationTimeline,
}

/// Makes a scene with the demo's fonts.
pub type SceneFactory = fn(&FontRegistry) -> Box<dyn DemoScene>;

/// The registrations: name and constructor (SceneCatalog.Registrations in its order; not ported:
/// "masks" (readback mask meanings), "sequence" and "frame-cache" (rendering frames ahead on worker
/// threads) - the sample's README says why), plus "shapes", the renderer's feature sheet, and
/// "missing-characters", the three missing-character modes side by side; then the SVG scenes and
/// the Lottie scenes.
pub const REGISTRATIONS: [(&str, SceneFactory); 73] = [
    ("easing", |f| Box::new(easing::EasingScene::new(f))),
    ("ramps", |f| Box::new(ramps::ColorRampScene::new(f))),
    ("gradients", |f| Box::new(gradients::GradientScene::new(f))),
    ("text", |f| Box::new(text::TextScene::new(f))),
    ("safe-areas", |f| Box::new(safe_areas::SafeAreaScene::new(f))),
    ("fade", |f| Box::new(fade::FadeScene::new(f))),
    ("procedural", |f| Box::new(procedural::ProceduralScene::new(f))),
    ("light", |f| Box::new(light::LightScene::new(f))),
    ("noise", |f| Box::new(noise::NoiseScene::new(f))),
    ("transitions", |f| Box::new(transitions::TransitionScene::new(f))),
    ("particles", |f| Box::new(particles::ParticleScene::new(f))),
    ("motion-blur", |f| Box::new(motion_blur::MotionBlurScene::new(f))),
    ("timeline", |f| Box::new(timeline::TimelineScene::new(f))),
    ("keyframes", |f| Box::new(keyframes::KeyframeScene::new(f))),
    ("transforms", |f| Box::new(transforms::TransformScene::new(f))),
    ("time-keys", |f| Box::new(time_keys::TimeKeyScene::new(f))),
    ("layout", |f| Box::new(layout::LayoutScene::new(f))),
    ("camera", |f| Box::new(camera::CameraScene::new(f))),
    ("playback", |f| Box::new(playback::PlaybackScene::new(f))),
    ("draw-on", |f| Box::new(draw_on::DrawOnScene::new(f))),
    ("motion-path", |f| Box::new(motion_path::MotionPathScene::new(f))),
    ("morph", |f| Box::new(morph::MorphScene::new(f))),
    ("sprites", |f| Box::new(sprites::SpriteScene::new(f))),
    ("images", |f| Box::new(images::ImageScene::new(f))),
    ("clip-mask", |f| Box::new(clip_mask::ClipMaskScene::new(f))),
    ("effects", |f| Box::new(effects::EffectsScene::new(f))),
    ("effects-follow", |f| Box::new(effects_follow::EffectsFollowScene::new(f))),
    ("path-combine", |f| Box::new(path_combine::PathCombineScene::new(f))),
    ("kerning", |f| Box::new(kerning::KerningScene::new(f))),
    ("paragraph", |f| Box::new(paragraph::ParagraphScene::new(f))),
    ("typewriter", |f| Box::new(typewriter::TypewriterScene::new(f))),
    ("outlined-text", |f| Box::new(outlined_text::OutlinedTextScene::new(f))),
    ("captions", |f| Box::new(captions::CaptionScene::new(f))),
    ("hard-edges", |f| Box::new(hard_edges::HardEdgeScene::new(f))),
    ("ownership", |f| Box::new(ownership::OwnershipScene::new(f))),
    ("colour-pickup", |f| Box::new(colour_pickup::ColorPickupScene::new(f))),
    ("brush-texture", |f| Box::new(brush_texture::BrushTextureScene::new(f))),
    ("brush-strokes", |f| Box::new(brush_strokes::BrushStrokesScene::new(f))),
    ("keyframe-playback", |f| Box::new(keyframe_playback::KeyframePlaybackScene::new(f))),
    ("formation-spiral", |f| Box::new(formation_spiral::FormationSpiralScene::new(f))),
    ("formation-heap", |f| Box::new(formation_heap::FormationHeapScene::new(f))),
    ("rolling-text", |f| Box::new(rolling_text::RollingTextScene::new(f))),
    ("additive", |f| Box::new(additive::AdditiveScene::new(f))),
    ("path-queries", |f| Box::new(path_queries::PathQueriesScene::new(f))),
    ("layer-visibility", |f| Box::new(layer_visibility::LayerVisibilityScene::new(f))),
    ("shapes", |f| Box::new(shapes::ShapesScene::new(f))),
    ("missing-characters", |f| Box::new(missing::MissingCharactersScene::new(f))),
    ("svg-parts", |f| Box::new(svg_parts::SvgPartsScene::new(f))),
    ("svg-smil", |f| Box::new(svg_smil::SvgSmilScene::new(f))),
    ("svg-css", |f| Box::new(svg_css::SvgCssScene::new(f))),
    ("svg-features", |f| Box::new(svg_features::SvgFeaturesScene::new(f))),
    ("lottie-files", |f| Box::new(lottie_files::LottieFilesScene::new(f))),
    ("lottie-features", |f| Box::new(lottie_features::LottieFeaturesScene::new(f))),
    ("lottie-images", |f| Box::new(lottie_images::LottieImagesScene::new(f))),
    // The feature scenes of the second GO (render-time looks, sketching, colours-only uploads, the
    // painterly stroke sets, planning and colour), each with controls of its own on the page.
    ("fixed-width", |f| Box::new(fixed_width::FixedWidthScene::new(f))),
    ("session-blend", |f| Box::new(session_blend::SessionBlendScene::new(f))),
    ("paint-override", |f| Box::new(paint_override::PaintOverrideScene::new(f))),
    ("wipe", |f| Box::new(wipe::WipeScene::new(f))),
    ("sketch", |f| Box::new(sketch::SketchScene::new(f))),
    ("ownership-colours", |f| Box::new(ownership_colours::OwnershipColoursScene::new(f))),
    ("stroke-sets", |f| Box::new(stroke_sets::StrokeSetsScene::new(f))),
    ("travelling-strokes", |f| Box::new(travelling_strokes::TravellingStrokesScene::new(f))),
    ("direction-fields", |f| Box::new(direction_fields::DirectionFieldsScene::new(f))),
    ("painting-timeline", |f| Box::new(painting_timeline::PaintingTimelineScene::new(f))),
    ("colour-rules", |f| Box::new(colour_rules::ColourRulesScene::new(f))),
    // Painterly P3: strokes coloured every frame from a live source (a still picture, another
    // session through a picture target, a 3D scene of the graphics crate), and the surface finish.
    ("live-pickup", |f| Box::new(live_pickup::LivePickupScene::new(f))),
    ("surface-finish", |f| Box::new(surface_finish::SurfaceFinishScene::new(f))),
    // Painterly P4: a still painting drawn from its checkpoints (the cache on and off, its
    // counters), and a stroke-level transition between two paintings.
    ("checkpoints", |f| Box::new(checkpoints::CheckpointsScene::new(f))),
    ("stroke-transition", |f| Box::new(stroke_transition::StrokeTransitionScene::new(f))),
    // Water A1 + A2: a pool seen through a ripple field (clicks drop stones; exact under seeking),
    // a bank mirrored below a waterline, and the painted pond that brings the round together.
    ("water-ripples", |f| Box::new(water_ripples::WaterRipplesScene::new(f))),
    ("water-reflection", |f| Box::new(water_reflection::WaterReflectionScene::new(f))),
    ("painted-pond", |f| Box::new(painted_pond::PaintedPondScene::new(f))),
    // The graphics crate's simulated wave field as the water of a refraction (cannot be rewound).
    ("simulated-pond-2d", |f| Box::new(simulated_pond::SimulatedPondScene::new(f))),
];

impl SceneCatalog {
    pub fn new(fonts: &FontRegistry) -> SceneCatalog {
        let mut scenes = Vec::new();
        let mut starts = Vec::new();
        let mut start = 0.0;
        for (_, create) in REGISTRATIONS {
            let scene = create(fonts);
            starts.push(start);
            start += scene.duration_seconds();
            scenes.push(scene);
        }
        let mut timeline = AnimationTimeline::new(start, 30.0).unwrap();
        for (i, scene) in scenes.iter().enumerate() {
            timeline.add_cue(scene.name(), starts[i], starts[i] + scene.duration_seconds(), EasingKind::Linear).unwrap();
        }
        SceneCatalog { scenes, starts, total_seconds: start, timeline }
    }

    /// Where a scene starts.
    pub fn start_of(&self, index: usize) -> f64 {
        self.starts[index.min(self.starts.len() - 1)]
    }

    /// The scene at a catalogue time and the time within it.
    pub fn at(&self, seconds: f64) -> (usize, f64) {
        let seconds = seconds.clamp(0.0, self.total_seconds);
        let mut index = self.scenes.len() - 1;
        while index > 0 && self.starts[index] > seconds {
            index -= 1;
        }
        (index, (seconds - self.starts[index]).min(self.scenes[index].duration_seconds()))
    }

    /// The index of a scene by name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.scenes.iter().position(|s| s.name() == name)
    }
}
