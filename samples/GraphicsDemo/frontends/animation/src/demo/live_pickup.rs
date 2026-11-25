//! "live-pickup" (painterly P3, Not in C#): a LARGE stroke set (5,000 to 40,000 records, bristle
//! tips made in code) whose colours are PICKED EVERY FRAME ON THE GPU from a live source
//! (`BrushStrokesShape::with_color(StrokeColor::Picked(StrokePickup { .. }))`), the source laid
//! over the painting area (`place`):
//!
//! - a STILL PICTURE: the iris picture made in code (`StrokeColorSource::Picture`), with the colour
//!   rules on the GPU (a palette of six pigments, broken colour) on controls;
//! - ANOTHER SESSION, animating underneath: a sky, a sun crossing it, a drifting cloud, swaying
//!   flower stems and a winding river - a session drawn every frame by a picture layer
//!   (`AnimationLayer::to_picture`) into a 640 x 360 `PictureTarget` (`StrokeColorSource::Target`):
//!   the picture moves and the strokes recolour themselves, with no stroke uploaded again;
//! - a 3D SCENE drawn by the GRAPHICS crate (codebrix-webgpu-graphics, this application's one use
//!   of it): the archer walking among trees under the procedural sky, the camera circling, drawn on
//!   the HDR path into a 640 x 360 target and tone mapped into an 8-bit sRGB texture, handed over
//!   as an `ExternalPicture` (`StrokeColorSource::Texture`, `PictureEncoding::Linear`: its sRGB
//!   view decodes to linear values).
//!
//! THE LAYER ORDER (demo/page.rs): the picture layer and the 3D layer are BELOW the catalogue's
//! layer in `CanvasLayers` (all layers share one encoder, so the strokes read THIS frame's
//! pictures), and a small layer ABOVE it shows the 3D picture itself when "the source" is chosen
//! (an image shape can show a picture target, not an application's texture). The source layers
//! draw only while this scene asks for them (the flags below), so the other scenes pay nothing.
//!
//! Controls: the source, the view (the strokes, or the source itself), the taps, the mix with the
//! stroke's own colour (the iris picture's colour under it), the colour rules and the count.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use codebrix_webgpu::glam::{Mat4, Quat, Vec3};
use codebrix_webgpu::{wgpu, CanvasFrame, CanvasRenderer, CanvasTarget, GpuContext, RendererHandle};
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::painting::{BrokenColor, ColorRules, PaintPalette, PaletteMode};
use codebrix_webgpu_animation::rendering::{AnimationLayer, CanvasCalibration, PictureTarget, VectorScene};
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape, ImageShapeOptions, RectangleShape};
use codebrix_webgpu_animation::strokes::{BrushStrokeSet, BrushTips, ExternalPicture, PictureEncoding, StrokeColor, StrokeColorSource, StrokePickup};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF};
use codebrix_webgpu_graphics::{Camera, Color4, EnvironmentMap, GltfDocument, GltfLoadOptions, GltfModelSet, GraphicsContext, GraphicsException, HeadlessRenderTarget, HeadlessTargetOptions, ModelInstance, ModelLoadOptions, ModelRenderer, PlayOptions, SceneLighting, SkyboxRenderer, SpriteBatch, SpriteBatchBeginOptions, ToneMapOperator, ToneMapper};

use crate::demo::colour_rules::PIGMENTS;
use crate::demo::kit::{disc, fill_rect, line, polygon};
use crate::demo::painted::{bristle_tips, jittered_strokes, picture};
use crate::demo::{create_frame_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH};
use crate::generated::api::models;

const TITLE: &str = "Live colour pickup";
const DESCRIPTION: &str = "A stroke set coloured every frame on the GPU from a live source: a still picture, another session moving underneath, or a 3D scene";
/// Where the painting (and the source over it) is laid, frame units.
const AREA: RectangleF = RectangleF { x: 107.0, y: 104.0, width: 1066.0, height: 600.0 };
/// The size of the moving session's picture and of the 3D scene's picture: half the canvas.
const SOURCE_SIZE: (u32, u32) = (640, 360);

const CONTROLS: &[Control] = &[
    Control::Choice { key: "source", label: "Source", options: &[("still picture", 0.0), ("moving session", 1.0), ("3D scene", 2.0)], default: 1.0 },
    Control::Choice { key: "view", label: "Show", options: &[("the strokes", 0.0), ("the source itself", 1.0)], default: 0.0 },
    Control::Choice { key: "taps", label: "Taps per stroke", options: &[("1", 1.0), ("3", 3.0), ("5", 5.0)], default: 5.0 },
    Control::Slider { key: "own", label: "Own colour weight", min: 0.0, max: 1.0, step: 0.05, default: 0.0 },
    Control::Choice { key: "rules", label: "Colour rules", options: &[("none", 0.0), ("palette", 1.0), ("broken colour", 2.0), ("both", 3.0)], default: 0.0 },
    Control::Choice { key: "mode", label: "Palette mode", options: &[("MixOfTwo", 1.0), ("Nearest", 0.0)], default: 1.0 },
    Control::Slider { key: "snap", label: "Palette snap", min: 0.0, max: 1.0, step: 0.05, default: 0.7 },
    Control::Slider { key: "hue", label: "Broken hue (degrees)", min: 0.0, max: 30.0, step: 1.0, default: 12.0 },
    Control::Slider { key: "chroma", label: "Broken chroma", min: 0.0, max: 0.3, step: 0.01, default: 0.15 },
    Control::Choice { key: "count", label: "Strokes", options: &[("5,000", 5000.0), ("20,000", 20000.0), ("40,000", 40000.0)], default: 20000.0 },
];

const LEGEND: &[&str] = &[
    "The painting follows its source every frame: with the moving session the sun's strokes turn yellow as it passes, the stems' strokes sway; with the 3D scene the painted forest turns with the camera.",
    "No stroke is uploaded again while the source moves: stroke_uploads stays 0 in the second stats line.",
    "Paused, nothing changes; a seek shows the same frame. \"The source itself\" shows what the strokes read.",
    "Own colour weight leans the strokes towards their own colour (the iris picture's); taps 1 / 3 / 5 trade cost against a steadier colour; the rules paint with six pigments and break the colour stroke by stroke.",
];

/// What the source layers share with the scene: the picture target, the 3D scene's picture, and
/// the flags the scene sets every frame it is drawn (the layers below read them one frame later,
/// the layer above in the same frame).
pub struct LiveSources {
    target: PictureTarget,
    want_session: Cell<bool>,
    want_scene3d: Cell<bool>,
    show_scene3d: Cell<bool>,
    scene3d: RefCell<Scene3d>,
    external: RefCell<Option<ExternalPicture>>,
    models: RefCell<Option<SceneModelBytes>>,
}

thread_local! {
    static SOURCES: Rc<LiveSources> = Rc::new(LiveSources {
        target: PictureTarget::new(SOURCE_SIZE.0, SOURCE_SIZE.1).expect("640 x 360 is a valid picture size"),
        want_session: Cell::new(false),
        want_scene3d: Cell::new(false),
        show_scene3d: Cell::new(false),
        scene3d: RefCell::new(Scene3d::default()),
        external: RefCell::new(None),
        models: RefCell::new(None),
    });
}

fn sources() -> Rc<LiveSources> {
    SOURCES.with(|s| s.clone())
}

/// The bytes of the 3D scene's models, fetched once by the page's loading step.
#[derive(Clone)]
pub struct SceneModelBytes {
    models: Vec<(String, String, GltfDocument)>,
    files: Rc<HashMap<String, Vec<u8>>>,
    ground: Rc<Vec<u8>>,
}

/// The models of the 3D source: (name, file below the models folder, pack).
const SCENE_MODELS: [(&str, &str, &str); 5] = [
    ("archer", "kenney-mini-forest/character-archer.glb", "kenney-mini-forest"),
    ("tree", "survival-kit/tree.glb", "survival-kit"),
    ("tree-autumn", "survival-kit/tree-autumn.glb", "survival-kit"),
    ("rock-a", "survival-kit/rock-a.glb", "survival-kit"),
    ("barrel", "survival-kit/barrel.glb", "survival-kit"),
];

async fn fetch_address(address: &str) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = address.split('/').collect();
    let message = |e: codebrix_webplatform::api::ApiError| format!("The model file '{address}' could not be read from the backend: {}", e.message());
    match parts.as_slice() {
        [pack, file] => models::model_file_bytes(pack, file).await.map_err(message),
        [pack, folder, file] => models::model_folder_file_bytes(pack, folder, file).await.map_err(message),
        _ => Err(format!("The model file '{address}' is not one the backend serves.")),
    }
}

/// Fetches the 3D source's model files (and the files they name, each once) and the ground's
/// blob, for the page's loading step (about 0.4 MB).
pub async fn fetch_scene_models() -> Result<(), String> {
    let mut list = Vec::new();
    let mut files = HashMap::new();
    for (name, file, pack) in SCENE_MODELS {
        let bytes = fetch_address(file).await?;
        let document = GltfDocument::parse(&bytes).map_err(|e| format!("The model file '{file}' could not be read: {e}"))?;
        for external in document.external_files(Some(pack), true) {
            if !files.contains_key(&external.address) {
                let bytes = fetch_address(&external.address).await?;
                files.insert(external.address, bytes);
            }
        }
        list.push((name.to_string(), pack.to_string(), document));
    }
    let ground = models::model_data_bytes("ground").await.map_err(|e| format!("The ground could not be read from the backend: {}", e.message()))?;
    *sources().models.borrow_mut() = Some(SceneModelBytes { models: list, files: Rc::new(files), ground: Rc::new(ground) });
    Ok(())
}

/// The layers this scene needs beside the catalogue's: (below it, above it).
pub fn source_layers() -> (Vec<RendererHandle>, Vec<RendererHandle>) {
    let shared = sources();
    // THE MOVING SESSION, drawn into the picture target every frame this scene asks for it.
    let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (FRAME_WIDTH, FRAME_HEIGHT), layer_opacity: 255, background_fill_color: Color::from_rgb(20, 30, 60), ..DrawingSessionOptions::default() });
    session.add_layer("Landscape", Color::WHITE);
    let flags = shared.clone();
    let picture_layer = AnimationLayer::to_picture(shared.target.clone(), move |frame, scene| {
        if !flags.want_session.replace(false) {
            return;
        }
        moving_painting(&mut session, frame.info.time);
        session.render_with_background(scene, frame.info.width, frame.info.height);
    });
    let below = vec![RendererHandle::new(picture_layer), RendererHandle::new(Scene3dLayer { shared: shared.clone() })];
    let above = vec![RendererHandle::new(Scene3dDisplay { shared, sprites: None, graphics: None })];
    (below, above)
}

/// The moving painting at `time` (catalogue seconds: a function of the frame's time only, so a
/// pause holds it and a seek repeats it): a sky, a sun crossing it, a cloud, a river and stems.
fn moving_painting(session: &mut DrawingSession, time: f64) {
    let t = time as f32;
    let layer = session.get_layer_mut("Landscape").expect("made above");
    layer.clear();
    let sky = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(0.0, 430.0), Color::from_rgb(24, 52, 140), Color::from_rgb(250, 196, 150)).expect("two colours");
    layer.add_shape(RectangleShape::with_fill(0.0, 0.0, 1280.0, 430.0, sky.into(), 0.0).expect("a rectangle"));
    // The sun on an arc across the sky, every 24 s.
    let a = (t / 24.0 * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU);
    let (sx, sy) = (640.0 - 520.0 * a.cos(), 360.0 - 260.0 * a.sin().abs());
    layer.add_shape(disc(sx, sy, 95.0, Some(Color::from_rgb(255, 214, 90))));
    layer.add_shape(disc(sx, sy, 62.0, Some(Color::from_rgb(255, 246, 200))));
    // A cloud drifting right.
    let cx = (t * 40.0).rem_euclid(1600.0) - 160.0;
    for (dx, dy, r) in [(0.0, 0.0, 46.0), (52.0, 10.0, 38.0), (-50.0, 12.0, 34.0), (20.0, -22.0, 34.0)] {
        layer.add_shape(disc(cx + dx, 150.0 + dy, r, Some(Color::from_rgb(244, 244, 250))));
    }
    // Hills, then a meadow.
    let hills: Vec<PointF> = (0..=32).map(|i| PointF::new(i as f32 * 40.0, 430.0 - 40.0 * ((i as f32 * 0.45).sin() + 1.0))).chain([PointF::new(1280.0, 720.0), PointF::new(0.0, 720.0)]).collect();
    layer.add_shape(polygon(&hills, Some(Color::from_rgb(70, 120, 60))));
    layer.add_shape(fill_rect(0.0, 470.0, 1280.0, 250.0, Some(Color::from_rgb(52, 104, 44)), 0.0));
    // A winding river whose bends travel slowly.
    let river: Vec<PointF> = (0..=26).map(|i| { let y = 440.0 + i as f32 * 11.0; PointF::new(560.0 + 90.0 * (y / 60.0 + t * 0.6).sin() - 40.0 - (y - 440.0) * 0.6, y) })
        .chain((0..=26).rev().map(|i| { let y = 440.0 + i as f32 * 11.0; PointF::new(560.0 + 90.0 * (y / 60.0 + t * 0.6).sin() + 40.0 + (y - 440.0) * 0.6, y) }))
        .collect();
    layer.add_shape(polygon(&river, Some(Color::from_rgb(70, 140, 220))));
    // Stems swaying, with purple heads.
    for i in 0..18 {
        let x = 60.0 + i as f32 * 68.0 + if i % 2 == 0 { 0.0 } else { 20.0 };
        let base = 700.0 - (i % 3) as f32 * 30.0;
        let length = 170.0 + (i % 4) as f32 * 30.0;
        let sway = (t * 1.3 + i as f32 * 0.7).sin() * 0.28;
        let (tx, ty) = (x + length * sway.sin(), base - length * sway.cos());
        layer.add_shape(line(x, base, tx, ty, 7.0, Some(Color::from_rgb(40, 150, 60))));
        layer.add_shape(disc(tx, ty, 24.0, Some(if i % 3 == 0 { Color::from_rgb(150, 70, 210) } else { Color::from_rgb(96, 84, 220) })));
        layer.add_shape(disc(tx, ty + 4.0, 8.0, Some(Color::from_rgb(250, 214, 70))));
    }
}

/// The 3D source: the graphics crate's renderers and targets, made at the first frame it is
/// wanted (on the page's device), and its instances.
#[derive(Default)]
struct Scene3d {
    graphics: Option<GraphicsContext>,
    gpu: Option<Scene3dGpu>,
    failed: Option<String>,
    errors: Vec<String>,
    frames: u64,
}

struct Scene3dGpu {
    renderer: ModelRenderer,
    skybox: SkyboxRenderer,
    tone_mapper: ToneMapper,
    environment: EnvironmentMap,
    hdr: HeadlessRenderTarget,
    ldr: HeadlessRenderTarget,
    instances: Vec<ModelInstance>,
    archer: usize,
}

impl Scene3d {
    fn build(graphics: &GraphicsContext, bytes: &SceneModelBytes) -> Result<Scene3dGpu, String> {
        let error = |e: GraphicsException| e.to_string();
        let mut set = GltfModelSet::new();
        let lookup = |address: &str| bytes.files.get(address).map(|b| b.as_slice());
        for (name, pack, document) in &bytes.models {
            set.add(document, name, Some(pack), &lookup, &GltfLoadOptions::default()).map_err(|e| format!("The model '{name}' could not be built: {e}"))?;
        }
        let made = set.create_models(graphics, &ModelLoadOptions::default()).map_err(error)?;
        let ground = graphics.load_model(&bytes.ground, &ModelLoadOptions::default()).map_err(error)?;
        let place = |index: usize, scale: f32, yaw: f32, at: Vec3| made[index].create_instance_at(Mat4::from_scale_rotation_translation(Vec3::splat(scale), Quat::from_rotation_y(yaw), at));
        let mut instances = vec![ground.create_instance()];
        for (k, (index, scale, at)) in [(1usize, 2.6f32, Vec3::new(-3.5, 0.0, -2.0)), (2, 2.8, Vec3::new(3.0, 0.0, -3.0)), (1, 2.4, Vec3::new(1.0, 0.0, -6.0)), (2, 2.6, Vec3::new(-5.5, 0.0, 3.0)), (3, 1.8, Vec3::new(2.5, 0.0, 2.5)), (4, 1.9, Vec3::new(-2.0, 0.0, 3.5)), (1, 2.6, Vec3::new(5.5, 0.0, 2.0))].into_iter().enumerate() {
            instances.push(place(index, scale, k as f32 * 1.3, at));
        }
        let mut archer = made[0].create_instance_at(Mat4::from_scale(Vec3::splat(2.0)));
        archer.animation().play_named("walk", PlayOptions::default()).map_err(|e| e.to_string())?;
        let archer_index = instances.len();
        instances.push(archer);
        let hdr = graphics.create_headless_target(SOURCE_SIZE.0, SOURCE_SIZE.1, HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba16Float, with_depth: true, depth_format: wgpu::TextureFormat::Depth24Plus }).map_err(error)?;
        // The 8-bit picture the strokes read: an Rgba8UnormSrgb target, so its view DECODES to
        // linear values on sampling (PictureEncoding::Linear) - right for the strokes and for the
        // sprite batch that shows it; the scene is opaque (the sky), so straight = premultiplied.
        let ldr = graphics.create_headless_target(SOURCE_SIZE.0, SOURCE_SIZE.1, HeadlessTargetOptions { format: wgpu::TextureFormat::Rgba8UnormSrgb, with_depth: false, depth_format: wgpu::TextureFormat::Depth24Plus }).map_err(error)?;
        let mut tone_mapper = graphics.create_tone_mapper().map_err(error)?;
        tone_mapper.operator = ToneMapOperator::KhronosPbrNeutral;
        Ok(Scene3dGpu { renderer: graphics.create_model_renderer().map_err(error)?, skybox: graphics.create_skybox_renderer().map_err(error)?, tone_mapper, environment: graphics.default_environment().map_err(error)?, hdr, ldr, instances, archer: archer_index })
    }
}

/// The 3D source's drawing: below the catalogue's layer, into its own targets only (it never
/// draws on the canvas).
struct Scene3dLayer {
    shared: Rc<LiveSources>,
}

impl CanvasRenderer for Scene3dLayer {
    fn prepare(&mut self, context: &GpuContext, _target: &CanvasTarget) {
        self.shared.scene3d.borrow_mut().graphics = Some(GraphicsContext::wrap_gpu_context(context));
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        if !self.shared.want_scene3d.replace(false) {
            return;
        }
        let mut state = self.shared.scene3d.borrow_mut();
        let Some(graphics) = state.graphics.clone() else { return };
        if state.gpu.is_none() && state.failed.is_none() {
            match self.shared.models.borrow().as_ref() {
                Some(bytes) => match Scene3d::build(&graphics, bytes) {
                    Ok(gpu) => {
                        *self.shared.external.borrow_mut() = Some(ExternalPicture { view: gpu.ldr.color().view().clone(), size: SOURCE_SIZE, encoding: PictureEncoding::Linear });
                        state.gpu = Some(gpu);
                    }
                    Err(error) => state.failed = Some(error),
                },
                None => state.failed = Some("The 3D scene's models were not fetched.".to_string()),
            }
        }
        let time = frame.info.time;
        let Some(gpu) = state.gpu.as_mut() else { return };
        // The camera circles the clearing; the archer walks a small circle; both on the frame's time.
        let t = time as f32;
        let mut camera = Camera::new();
        camera.field_of_view = 50f32.to_radians();
        camera.position = Vec3::new((0.25 * t).cos() * 11.0, 4.5, (0.25 * t).sin() * 11.0);
        camera.target = Vec3::new(0.0, 1.5, 0.0);
        camera.near_plane = 0.1;
        camera.far_plane = 400.0;
        let walk = 0.5 * t;
        let archer = &mut gpu.instances[gpu.archer];
        archer.world = Mat4::from_scale_rotation_translation(Vec3::splat(2.0), Quat::from_rotation_y(-walk), Vec3::new(walk.cos() * 1.5, 0.0, walk.sin() * 1.5));
        archer.animation().update_to(time);
        let lighting = SceneLighting { environment: Some(gpu.environment.clone()), environment_intensity: 1.0, light_intensity: 2.8, light_direction: Vec3::new(-0.5, -1.0, -0.4).normalize(), ..SceneLighting::default() };
        let result: Result<(), GraphicsException> = (|| {
            let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
            {
                let mut pass = graphics_frame.begin_render_pass_to(&gpu.hdr.view(), Some(Color4::BLACK), Some(1.0))?;
                gpu.renderer.begin(&pass, &camera, Some(&lighting))?;
                for instance in &gpu.instances {
                    gpu.renderer.draw(instance)?;
                }
                gpu.renderer.end_opaque(&mut pass)?;
                gpu.skybox.draw(&mut pass, &camera, Some(&lighting))?;
                gpu.renderer.end_blended(&mut pass)?;
            }
            gpu.tone_mapper.apply(&mut graphics_frame, gpu.hdr.color(), &gpu.ldr.view())
        })();
        state.frames += 1;
        if let Err(error) = result {
            let text = error.to_string();
            if state.errors.len() < 10 && !state.errors.contains(&text) {
                state.errors.push(text);
            }
        }
        for error in graphics.take_errors() {
            let text = error.to_string();
            if state.errors.len() < 10 && !state.errors.contains(&text) {
                state.errors.push(text);
            }
        }
    }
}

/// Shows the 3D source's picture over the painting area when the scene asks for "the source
/// itself" (a layer ABOVE the catalogue's: an image shape cannot show an application's texture).
struct Scene3dDisplay {
    shared: Rc<LiveSources>,
    graphics: Option<GraphicsContext>,
    sprites: Option<SpriteBatch>,
}

impl CanvasRenderer for Scene3dDisplay {
    fn prepare(&mut self, context: &GpuContext, _target: &CanvasTarget) {
        let graphics = GraphicsContext::wrap_gpu_context(context);
        self.sprites = graphics.create_sprite_batch().ok();
        self.graphics = Some(graphics);
    }

    fn render(&mut self, frame: &mut CanvasFrame<'_>) {
        if !self.shared.show_scene3d.replace(false) {
            return;
        }
        let (Some(graphics), Some(sprites)) = (self.graphics.as_ref(), self.sprites.as_mut()) else { return };
        let state = self.shared.scene3d.borrow();
        let Some(gpu) = state.gpu.as_ref() else { return };
        let (width, height) = (frame.info.width as f32, frame.info.height as f32);
        let rect = CanvasCalibration::get_drawing_rect(width, height, (FRAME_WIDTH, FRAME_HEIGHT));
        let scale = rect.width / FRAME_WIDTH as f32;
        let place = codebrix_webgpu_graphics::RectangleF::new(rect.x + AREA.x * scale, rect.y + AREA.y * scale, AREA.width * scale, AREA.height * scale);
        let mut graphics_frame = graphics.begin_canvas_frame(frame, None);
        let result: Result<(), GraphicsException> = (|| {
            let mut pass = graphics_frame.begin_render_pass(None, None)?;
            sprites.begin(&pass, SpriteBatchBeginOptions::default())?;
            sprites.draw(gpu.ldr.color(), place, Color4::WHITE);
            sprites.end(&mut pass)
        })();
        let _ = result;
    }
}

pub struct LivePickupScene {
    frame: DrawingSession,
    painting: DrawingSession,
    set: Option<(usize, BrushStrokeSet)>,
    tips: BrushTips,
    still: ImageSource,
    settings: Settings,
    reports: Vec<String>,
    source_name: &'static str,
    waiting: bool,
}

/// The stroke records: the iris picture's colours as their OWN colours, horizontal in the sky and
/// upright in the meadow, all shown (no painting on).
fn records(count: usize) -> BrushStrokeSet {
    let image = picture();
    let spacing = (AREA.width * AREA.height / count as f32).sqrt();
    let horizon = AREA.y + AREA.height * 0.42;
    let list = jittered_strokes(&image, AREA, count, 11, spacing * 3.4, spacing * 1.5, &|x, y| if y < horizon { 6.0 * ((x - AREA.x) / 150.0).sin() } else { -78.0 + 18.0 * ((x - AREA.x) / 60.0).sin() });
    BrushStrokeSet::new(list).expect("the records are valid")
}

impl LivePickupScene {
    pub fn new(fonts: &FontRegistry) -> LivePickupScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Note", DIM);
        label(notes, fonts, "source laid over the whole painting area (StrokePickup::place)", AREA.x, AREA.y + AREA.height + 13.0, 12.0, DIM, TextAlign::Left);
        let mut painting = crate::demo::create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        painting.add_layer("Source", Color::WHITE);
        painting.add_layer("Paint", Color::WHITE);
        let still = ImageSource::from_plane_image(&picture()).expect("the iris picture");
        LivePickupScene { frame, painting, set: None, tips: bristle_tips(), still, settings: Settings::from_controls(CONTROLS), reports: Vec::new(), source_name: "", waiting: false }
    }

    fn rules(&self) -> Option<ColorRules> {
        let which = self.settings.get("rules") as usize;
        let palette = PaintPalette { pigments: PIGMENTS.to_vec(), snap: self.settings.get("snap"), mode: if self.settings.get("mode") >= 0.5 { PaletteMode::MixOfTwo } else { PaletteMode::Nearest } };
        let broken = BrokenColor { hue: self.settings.get("hue"), chroma: self.settings.get("chroma"), lightness: 0.05, seed: 1 };
        match which {
            1 => Some(ColorRules { palette: Some(palette), broken: None }),
            2 => Some(ColorRules { palette: None, broken: Some(broken) }),
            3 => Some(ColorRules { palette: Some(palette), broken: Some(broken) }),
            _ => None,
        }
    }
}

impl DemoScene for LivePickupScene {
    fn name(&self) -> &'static str {
        "live-pickup"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        24.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let shared = sources();
        let source = self.settings.get("source") as usize;
        let show_source = self.settings.get("view") >= 0.5;
        // Ask the source layers for their pictures (the layers below draw them from the next
        // frame on, the layer above in this one).
        shared.want_session.set(source == 1);
        shared.want_scene3d.set(source == 2);
        shared.show_scene3d.set(source == 2 && show_source);
        let count = self.settings.get("count") as usize;
        if self.set.as_ref().map(|(n, _)| *n) != Some(count) {
            self.set = Some((count, records(count)));
        }
        let set = self.set.as_ref().expect("made above").1.clone();
        let (color_source, name) = match source {
            1 => (Some(StrokeColorSource::Target(shared.target.clone())), "moving session (PictureTarget)"),
            2 => (shared.external.borrow().clone().map(StrokeColorSource::Texture), "3D scene (ExternalPicture)"),
            _ => (Some(StrokeColorSource::Picture(self.still.clone())), "still picture (ImageSource)"),
        };
        self.source_name = name;
        self.waiting = color_source.is_none();
        let source_layer = self.painting.get_layer_mut("Source").expect("made in new");
        source_layer.clear();
        // The 3D picture is shown by the layer above (Scene3dDisplay).
        if show_source && source == 1 {
            source_layer.add_shape(ImageShape::from_picture_target(&shared.target, AREA, ImageSampling::Linear).expect("a valid area"));
        } else if show_source && source == 0 {
            source_layer.add_shape(ImageShape::new(self.still.clone(), AREA, ImageShapeOptions::default()).expect("a valid area"));
        }
        let paint = self.painting.get_layer_mut("Paint").expect("made in new");
        paint.clear();
        if !show_source {
            if let Some(color_source) = color_source {
                let pickup = StrokePickup { taps: self.settings.get("taps") as u32, own_weight: self.settings.get("own"), rules: self.rules(), ..StrokePickup::new(color_source, AREA) };
                let shape = BrushStrokesShape::new(set, BrushStrokesOptions::default()).expect("a set").with_tips(self.tips.clone()).with_color(StrokeColor::Picked(pickup));
                match shape {
                    Ok(shape) => {
                        self.painting.set_active_layer("Paint");
                        self.painting.draw_brush_strokes(shape);
                    }
                    Err(error) => self.reports = vec![error],
                }
            }
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render(scene, width, height);
        let reports = self.painting.take_reports();
        if !reports.is_empty() {
            self.reports = reports;
        }
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.reports.clear();
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        let shared = sources();
        let state = shared.scene3d.borrow();
        let scene3d = match (&state.failed, state.gpu.is_some()) {
            (Some(error), _) => format!("failed: {error}"),
            (None, true) => format!("drawing ({} frames drawn)", state.frames),
            (None, false) => "not made yet (made the first time it is chosen)".to_string(),
        };
        vec![
            Readout::new("source-now", "Colours picked from", if self.waiting { "the 3D picture is not made yet (next frame)" } else { self.source_name }),
            Readout::new("strokes", "Strokes in the set", self.set.as_ref().map_or(0, |(_, s)| s.len())),
            Readout::new("source-size", "Source picture size", format!("{} x {}", SOURCE_SIZE.0, SOURCE_SIZE.1)),
            Readout::new("scene3d", "3D source", scene3d),
            Readout::new("scene3d-errors", "3D source errors", if state.errors.is_empty() { "none".to_string() } else { state.errors.join(" | ") }),
            Readout::new("reports", "Session reports", if self.reports.is_empty() { "none".to_string() } else { self.reports.join(" | ") }),
        ]
    }
}
