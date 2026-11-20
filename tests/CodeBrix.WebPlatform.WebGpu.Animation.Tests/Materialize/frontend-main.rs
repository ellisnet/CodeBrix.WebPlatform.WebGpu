//! The test frontend of the Animation package's real-materialize tests: it names the crate
//! codebrix-webgpu-animation (and codebrix-webgpu, which it builds on), so that
//! `cargo check --offline --target wasm32-unknown-unknown` proves a frontend can build with it
//! from the materialized vendor folder alone. It is compiled, never run. It uses the crate's
//! public items the way an application does: a drawing session of calibrated units with layers of
//! shapes (a rounded rectangle with a gradient fill, a circle, an ellipse, a line, a polygon, a
//! dashed path), easing curves, a colour ramp, a timeline with cues and a marker at one of the
//! clock's named points, a playback controller that follows the frame's time, the session drawn
//! by an AnimationLayer, and a second AnimationLayer that records into the VectorScene directly
//! with peniko brushes and kurbo shapes (a sweep gradient, a group with a mix mode, a clip); and
//! TEXT: a font registry fed with font bytes (here an empty buffer the application would fill
//! from its backend or its own assets), a line of text with a gradient fill and the
//! missing-character choice, a measured width, the missing-character report, the debug overlay,
//! and a session drawn without its background into a part of the canvas at half opacity; and
//! THE ANIMATION MODEL AND PATHS: keyframe tracks bound to an element, a layer and the camera, a
//! group, an animated shape, a time window, a layer's parallax, a layout, a trimmed path drawn on
//! by a track, an element moved along a path and a morph whose fraction is bound; and PICTURES,
//! SPRITES, MASKS AND EFFECTS: a picture from RGBA pixels with a source rectangle and a tint
//! bound to a track, a sprite sheet cut from a grid played by time and by a bound frame index, a
//! group masked inside a path, a layer masked outside one, a layer matte, and a glow, a drop
//! shadow and a blur with bound parameters on elements and a layer; and TEXT BLOCKS, CAPTIONS,
//! PATHS AND FORMATIONS: a laid-out block with a bound reveal, a stagger of its pieces, the
//! pieces sent along a path, captions from SRT (against the frame, and the safe-area opt-in), path
//! combining and queries, a combined binding, a roll along a path, a formation, a hidden layer
//! and the safe-area overlay.

use std::cell::RefCell;
use std::rc::Rc;

use codebrix_webgpu::{CanvasClock, CanvasLayers, GpuContext, RendererHandle, WebGpuCanvas, WebGpuUnavailable};
use codebrix_webgpu_animation::animation::{AnimationTimeline, ColorRamp, CubicBezierEasing, Easing, EasingKind, FloatTrack, PlaybackController, PointTrack, StepPosition, TrackLoopMode};
use codebrix_webgpu_animation::layout::{DrawingLayout, LayoutAnchor, LayoutReference};
use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, DrawingCamera, DrawingTransform, PathTravelOptions};
use codebrix_webgpu_animation::fills::{ColorStop, GradientSpread, LinearGradientFill};
use codebrix_webgpu_animation::kurbo::{Affine, Circle, Point, Rect};
use codebrix_webgpu_animation::paths::{PathGeometry, PathMorph, PathStrokeCap, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::peniko::{BlendMode, Brush, Color as PenikoColor, Compose, Fill, Gradient, Mix};
use codebrix_webgpu_animation::rendering::{AnimationLayer, CanvasCalibration, DebugOverlay, DebugOverlayCorner};
use codebrix_webgpu_animation::shapes::{AnimatedShape, CircleShape, DrawingGroup, DrawingShape, EllipseShape, LineShape, LineShapeOptions, PathShape, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, MeasureTextOptions, MissingCharacterMode, TextAlign};
use codebrix_webgpu_animation::effects::{DrawingEffects, DrawingMask, ShadowDirection};
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource, SpritePlayback, SpriteSheet};
use codebrix_webgpu_animation::shapes::{ImageShape, ImageShapeOptions, SpriteShapeOptions};
use codebrix_webgpu_animation::captions::{CaptionPlacement, CaptionPresenter, CaptionStyle, CaptionTrack};
use codebrix_webgpu_animation::formations::{ArrivalSchedule, CenterOutArrivalOrder, FormationArea, FormationMember, FormationPose, FormationTargetOptions, FormationTargets, IArrivalPath, SpiralArrival, SpiralArrivalOptions};
use codebrix_webgpu_animation::layout::DrawingLayoutGuideOptions;
use codebrix_webgpu_animation::models::{BindingMode, PathTrainOptions, StaggerOptions};
use codebrix_webgpu_animation::paths::{PathBooleanOperation, PathInflateOptions};
use codebrix_webgpu_animation::safe_areas::{SafeAreaCheckOptions, SafeAreaGuides, SafeAreaOverlay};
use codebrix_webgpu_animation::shapes::TextBlockShape;
use codebrix_webgpu_animation::text::{TextLayoutOptions, TextUnit};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, Rectangle, RectangleF, SessionRenderOptions};
use dioxus::prelude::*;

/// Pictures, sprites, masks and effects on a session (the pixels would come from the backend).
fn pictures(session: &mut DrawingSession) -> Result<(), String> {
    let pixels = ImageSource::from_rgba(&[255; 16 * 8 * 4], 16, 8)?;
    let picture = session.draw_shape(ImageShape::new(pixels.clone(), RectangleF::new(10.0, 10.0, 160.0, 80.0), ImageShapeOptions { source_rectangle: Some(Rectangle::new(0, 0, 8, 8)), sampling: ImageSampling::Nearest, tint: Some(Color::from_rgb(255, 200, 200)) })?);
    session.animations_mut().bind(picture, AnimatedProperty::Tint, codebrix_webgpu_animation::animation::ColorTrack::from_keys(&[(0.0, Color::WHITE, EasingKind::Linear)])?)?;
    let sheet = SpriteSheet::from_grid(&pixels, 2, 1, 0)?;
    let sprite = session.draw_sprite(&sheet, RectangleF::new(200.0, 10.0, 80.0, 80.0), SpriteShapeOptions { frames_per_second: 12.0, playback: SpritePlayback::PingPong, sampling: ImageSampling::Linear })?;
    session.animations_mut().bind(sprite, AnimatedProperty::FrameIndex, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (1.0, 1.0, EasingKind::Linear)])?)?;
    let mut group = DrawingGroup::new(None);
    group.add(CircleShape::new(50.0, 50.0, 40.0, ShapeOptions { stroke_thickness: 1.0, color: None, is_filled: true })?);
    group.set_mask(Some(DrawingMask::inside(PathGeometry::star(PointF::new(50.0, 50.0), 40.0, 16.0, 5, 0.0)?)));
    let glowing = session.draw_shape(DrawingShape::from(group).with_effects(DrawingEffects::glow(12.0, Color::from_rgb(255, 140, 40), 1.5)?.with_shadow(PointF::new(4.0, 6.0), 6.0, Color::from_rgba(0, 0, 0, 120))?.with_shadow_direction(ShadowDirection::RotatesWithElement)));
    let mut a = session.animations_mut();
    a.bind(glowing, AnimatedProperty::GlowRadius, FloatTrack::from_keys(&[(0.0, 4.0, EasingKind::Linear), (2.0, 20.0, EasingKind::SineInOut)])?)?;
    a.bind(glowing, AnimatedProperty::ShadowOffset, PointTrack::from_keys(&[(0.0, PointF::new(2.0, 2.0), EasingKind::Linear)])?)?;
    a.bind(glowing, AnimatedProperty::BlurRadius, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear)])?)?;
    session.add_layer("Matte", Color::WHITE).set_is_visible(false);
    let layer = session.add_layer("Masked", Color::WHITE);
    layer.set_mask(Some(DrawingMask::outside(PathGeometry::circle(PointF::new(300.0, 300.0), 50.0))));
    layer.set_effects(Some(DrawingEffects::drop_shadow(PointF::new(6.0, 6.0), 4.0, Color::BLACK)?.with_blur_radius(1.0)?));
    session.add_layer("Matted", Color::WHITE).set_mask(Some(DrawingMask::from_layer("Matte", true)));
    session.animations_mut().bind(BindingTarget::layer("Masked"), AnimatedProperty::ShadowColor, codebrix_webgpu_animation::animation::ColorTrack::from_keys(&[(0.0, Color::BLACK, EasingKind::Linear)])?)?;
    Ok(())
}

/// The animation model and paths on a session: tracks bound to an element, a layer and the
/// camera; a group; an animated shape; a time window; parallax; a layout; a draw-on; a motion
/// along a path; a morph.
fn animated(session: &mut DrawingSession) -> Result<(), String> {
    let layout = DrawingLayout::new((1280, 720))?;
    let centre = layout.place_at(LayoutAnchor::CENTER, LayoutReference::Frame, 0.0, 0.0);
    session.add_layer("Moving", Color::WHITE).set_parallax(0.5)?;
    let mut group = DrawingGroup::new(Some(Color::from_rgb(90, 200, 255)));
    group.add(CircleShape::new(0.0, 0.0, 20.0, ShapeOptions { is_filled: true, ..Default::default() })?);
    let group = session.draw_shape(DrawingShape::from(group).with_transform(DrawingTransform::translate(centre.x, centre.y)).with_time_window(Some(0.5), None));
    let mut position = PointTrack::new();
    position.add(0.0, PointF::new(100.0, 600.0), EasingKind::Linear)?.add(2.0, PointF::new(1100.0, 600.0), EasingKind::CubicInOut)?;
    let spin = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 360.0, EasingKind::Linear)])?.with_loop_mode(TrackLoopMode::Loop);
    session.set_camera(Some(DrawingCamera::new()));
    let path = PathGeometry::circle(centre, 200.0);
    let mut ring = PathShape::with_style(path.clone(), PathStrokeStyle::new(6.0, PathStrokeStyleOptions::default())?, Some(Color::WHITE), None, None);
    ring.set_trim_end(0.0)?;
    let ring = session.draw_path(ring);
    let morph = PathMorph::new(PathGeometry::star(centre, 120.0, 50.0, 5, 0.0)?, path.clone(), 0)?;
    let morphing = session.draw_path(PathShape::from_morph(morph, 0.0, ShapeOptions { is_filled: true, ..Default::default() })?);
    let dot = session.draw_shape(AnimatedShape::new(|t| CircleShape::new(10.0 + (t as f32 % 1.0) * 5.0, 10.0, 4.0, ShapeOptions { is_filled: true, ..Default::default() }).ok().map(DrawingShape::from), 0.0));
    let progress = FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (4.0, 1.0, EasingKind::SineInOut)])?;
    let mut animations = session.animations_mut();
    animations.bind(group, AnimatedProperty::Position, position)?;
    animations.bind(group, AnimatedProperty::Rotation, spin)?;
    animations.bind(BindingTarget::layer("Moving"), AnimatedProperty::Opacity, FloatTrack::from_keys(&[(0.0, 0.2, EasingKind::Linear), (1.0, 1.0, EasingKind::Linear)])?)?;
    animations.bind(BindingTarget::Camera, AnimatedProperty::Scale, FloatTrack::from_keys(&[(0.0, 1.0, EasingKind::Linear), (4.0, 1.3, EasingKind::SineInOut)])?)?;
    animations.bind(ring, AnimatedProperty::TrimEnd, progress.clone())?;
    animations.bind(morphing, AnimatedProperty::MorphFraction, progress.clone())?;
    animations.bind_along_path(dot, &path, progress, PathTravelOptions { orient_to_path: true, rotation_offset: 0.0, ..PathTravelOptions::default() })?;
    let _ = path.get_point_at(0.5).angle();
    Ok(())
}

/// Steps 9 and 10 and the pictures from files compiled in a frontend: planes, noise drawn on the
/// GPU, particles, frame operations, layer transitions, plane masks, PNG and picture-pixels
/// reading, brushstrokes painting on, ownership maps coloured on the GPU, keyframe players.
fn planes_and_painting(session: &mut DrawingSession) -> Result<(), String> {
    use codebrix_webgpu_animation::effects::{DrawingFrameOperations, DrawingMask};
    use codebrix_webgpu_animation::painting::{KeyframePlayer, KeyframePlayerOptions};
    use codebrix_webgpu_animation::planes::{DrawingNoiseKind, DrawingParticleEmitter, DrawingParticleSystem, DrawingPlane, DrawingTransition};
    use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, NoiseShape, NoiseShapeOptions, OwnershipShape, OwnershipShapeOptions, ParticlesShape};
    use codebrix_webgpu_animation::MaskMeaning;
    let plane = DrawingPlane::with_value(4, 4, 0.5)?;
    let picture = codebrix_webgpu_animation::images::ImageSource::from_picture_pixels(&[b'C', b'B', b'P', b'X', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 9, 9, 9, 255])?;
    let _ = codebrix_webgpu_animation::images::ImageSource::from_png(&[0x89, b'P', b'N', b'G']).err();
    let layer = session.add_layer("Planes", Color::WHITE);
    layer.add_shape(NoiseShape::new(0.0, 0.0, 100.0, 100.0, DrawingNoiseKind::Gradient, 7, 0.02, NoiseShapeOptions::default())?);
    layer.set_frame_operations(Some(DrawingFrameOperations::NONE.with_tone_curve(0.15, 1.08).with_bloom(4.0, 0.5)?));
    layer.set_mask(Some(DrawingMask::from_plane(&plane, RectangleF::new(0.0, 0.0, 100.0, 100.0), false)?));
    let mut system = DrawingParticleSystem::new(1);
    system.add_emitter(DrawingParticleEmitter::new()).set_burst(10);
    layer.add_shape(ParticlesShape::new(&system, 0.5, None)?);
    layer.add_shape(BrushStrokeShape::new(50.0, 50.0, 80.0, 20.0, 10.0, BrushStrokeShapeOptions::default())?.with_progress(0.5));
    session.add_layer("Next", Color::WHITE).add_shape(codebrix_webgpu_animation::shapes::ImageShape::new(picture, RectangleF::new(0.0, 0.0, 10.0, 10.0), Default::default())?);
    session.set_layer_transition("Planes", "Next", DrawingTransition::dissolve(), 0.5)?;
    let map = session.export_layer_ownership_map("Planes", (64, 64))?;
    let colors = vec![Color::WHITE; map.element_count()];
    session.add_layer("Map", Color::WHITE).add_shape(OwnershipShape::new(&map, &colors, RectangleF::new(0.0, 0.0, 64.0, 64.0), OwnershipShapeOptions::default())?);
    let _ = session.export_mask((64, 64), MaskMeaning::HardEdged)?;
    let mut player = KeyframePlayer::new(8, 8, 1.0, |_, _, rgba| { rgba.fill(255); Ok(()) }, KeyframePlayerOptions::default())?;
    let mut frame = vec![0u8; 8 * 8 * 4];
    player.render_frame(0.5, &mut frame)?;
    let _ = session.take_reports();
    Ok(())
}

/// Steps 6, 11 and 12 compiled in a frontend: text blocks and their bindings, captions, path
/// combining and queries, combined and rolling bindings, formations, safe areas.
fn blocks_and_formations(session: &mut DrawingSession) -> Result<(), String> {
    let layout = session.layout_text("Hello, blocks", 48.0, None, Some(&TextLayoutOptions { max_width: Some(600.0), align: TextAlign::Center, ..Default::default() }))?;
    let block = session.draw_text_block(layout.clone(), 40.0, 40.0, None)?;
    session.animations_mut().bind(block, AnimatedProperty::RevealFraction, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (2.0, 1.0, EasingKind::CubicOut)])?)?;
    let shape = TextBlockShape::new(layout, 40.0, 200.0, Some(Color::WHITE))?;
    let pieces: Vec<_> = shape.create_pieces(TextUnit::Word).into_iter().map(|p| session.draw_shape(p)).collect();
    session.animations_mut().bind_stagger(&pieces, Some(&StaggerOptions { interval: 0.1, ..Default::default() }))?;
    let route = PathGeometry::parse("M -200 600 C 200 500 400 700 640 400")?;
    session.animations_mut().bind_text_along_path(&shape, TextUnit::Character, "Shapes", &route, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (3.0, 1.0, EasingKind::SineOut)])?, PathTrainOptions { roll_on_baseline: true, ..Default::default() })?;
    let captions = CaptionTrack::parse("WEBVTT\n\n00:01.000 --> 00:03.000\nA caption\n")?;
    session.draw_captions(captions.clone(), None, None)?;
    let placement = CaptionPlacement::from_guide((1280, 720), &SafeAreaGuides::broadcast_title_safe(), DrawingLayoutGuideOptions::default())?;
    let presenter = CaptionPresenter::new(captions, session.fonts(), DrawingLayout::new((1280, 720))?, Some(CaptionStyle::default()))?.with_placement(placement);
    session.draw_captions_with(presenter);
    let a = PathGeometry::circle(PointF::new(300.0, 300.0), 80.0);
    let b = PathGeometry::rectangle(RectangleF::new(300.0, 260.0, 160.0, 80.0));
    let combined = PathGeometry::combine_all(&[a.clone(), b.clone()], PathBooleanOperation::Union, None)?;
    let _ = (a.subtract(&b, None)?, a.intersect(&b, Some(0.1))?, a.exclusive_or(&b, None)?, combined.simplify(None)?, a.inflate(4.0, PathInflateOptions::default())?);
    let _ = (combined.contains(PointF::new(300.0, 300.0), None)?, combined.distance_to_edge(PointF::new(0.0, 0.0), None)?, combined.get_nearest_point(PointF::new(0.0, 0.0), None)?);
    let outline = route.to_stroke_outline(&PathStrokeStyle::new(6.0, PathStrokeStyleOptions::default())?, None)?;
    let dot = session.draw_path(PathShape::with_style(outline, PathStrokeStyle::new(1.0, PathStrokeStyleOptions::default())?, None, None, None));
    session.animations_mut().bind_with_mode(dot, AnimatedProperty::Rotation, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (4.0, 30.0, EasingKind::Linear)])?, BindingMode::Add)?;
    session.animations_mut().bind_roll_along_path(dot, &route, FloatTrack::from_keys(&[(0.0, 0.0, EasingKind::Linear), (4.0, 1.0, EasingKind::Linear)])?, 20.0, None, true)?;
    let targets = FormationTargets::fill(&FormationArea::from_geometry(&a, None)?, FormationTargetOptions { spacing: 20.0, order: Some(Box::new(CenterOutArrivalOrder::new(false))), ..Default::default() })?;
    let times = ArrivalSchedule::new(0.0, 2.0, None)?.get_times(targets.len());
    let members = FormationMember::from_targets(&targets, &times, |target, arrival, _| {
        let path: Rc<dyn IArrivalPath> = Rc::new(SpiralArrival::new(FormationPose::at(target.position.x, target.position.y), arrival - 1.0, arrival, 200.0, SpiralArrivalOptions::default()).unwrap());
        path
    })?;
    session.add_layer("Rest", Color::WHITE);
    session.add_layer("Travel", Color::WHITE);
    session.animations_mut().bind_formation(&members, "Rest", "Travel", |_| DrawingShape::from(CircleShape::new(0.0, 0.0, 4.0, ShapeOptions { is_filled: true, ..Default::default() }).unwrap()), None)?;
    session.get_layer_mut("Travel").ok_or("layer")?.set_is_visible(false);
    session.set_safe_area_overlay(Some(SafeAreaOverlay::new(SafeAreaGuides::instagram_reels_ad())));
    let _ = session.check_safe_area(&SafeAreaGuides::tiktok(), None, SafeAreaCheckOptions::default());
    Ok(())
}

/// The session of the first layer: shapes in calibrated units.
fn session() -> Result<DrawingSession, String> {
    let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
    let stops = [ColorStop::new(0.0, Color::from_rgb(214, 96, 120)).ok_or("stop")?, ColorStop::new(1.0, Color::from_rgb(120, 90, 210)).ok_or("stop")?];
    let gradient = LinearGradientFill::new(PointF::new(40.0, 40.0), PointF::new(440.0, 40.0), &stops, GradientSpread::Pad)?;
    let shapes = session.add_layer("Shapes", Color::from_rgb(255, 176, 72));
    shapes.add_shape(RectangleShape::with_fill(40.0, 40.0, 400.0, 120.0, gradient.into(), 12.0)?);
    shapes.add_shape(RectangleShape::new(40.0, 200.0, 200.0, 80.0, RectangleShapeOptions { stroke_thickness: 3.0, color: Some(Color::WHITE), is_filled: false, corner_radius: 6.0 })?);
    shapes.add_shape(CircleShape::new(640.0, 360.0, 40.0, ShapeOptions { is_filled: true, ..Default::default() })?);
    shapes.add_shape(EllipseShape::new(900.0, 360.0, 90.0, 40.0, ShapeOptions { stroke_thickness: 2.0, color: None, is_filled: false })?);
    shapes.add_shape(LineShape::new(40.0, 600.0, 1240.0, 600.0, LineShapeOptions { stroke_thickness: 2.0, color: Some(Color::from_rgba(255, 255, 255, 160)) })?);
    let star = [PointF::new(1000.0, 100.0), PointF::new(1060.0, 260.0), PointF::new(920.0, 160.0), PointF::new(1080.0, 160.0), PointF::new(940.0, 260.0)];
    shapes.add_shape(PolylineShape::new(&star, PolylineShapeOptions { stroke_thickness: 2.0, color: Some(Color::from_rgb(90, 200, 160)), is_closed: true, is_filled: true })?);
    let style = PathStrokeStyle::new(4.0, PathStrokeStyleOptions { cap: PathStrokeCap::Round, dash_pattern: Some(vec![12.0, 8.0]), ..Default::default() })?;
    let wave = PathGeometry::parse("M 100 500 C 300 400 500 600 700 500 S 1100 400 1200 500")?;
    shapes.add_shape(PathShape::with_style(wave, style, Some(Color::from_rgb(240, 240, 255)), None, None));
    Ok(session)
}

/// Text: fonts from bytes (an application hands over what its backend served, or bytes it packed
/// itself), a line with a gradient fill, the missing-character choice and report, a measured
/// width, the debug overlay, and a session drawn into part of the canvas without its background.
fn text(font_bytes: Vec<u8>) -> Result<DrawingSession, String> {
    let fonts = FontRegistry::new();
    let family = fonts.register_font(font_bytes, Some("Body"))?;
    let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, fonts: Some(fonts.clone()), ..Default::default() });
    session.add_layer("Text", Color::WHITE);
    let fill = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(400.0, 0.0), Color::from_rgb(120, 170, 255), Color::WHITE)?;
    let title = TextShape::with_fill("Title", 640.0, 100.0, 48.0, &fonts, fill.into(), TextShapeOptions { align: TextAlign::Center, tracking: 0.02, family_name: Some(family.clone()), ..Default::default() })?;
    let width = fonts.measure_text("Title", 48.0, &MeasureTextOptions { tracking: 0.02, family_name: Some(family), kerning: true })?;
    let _ = (title.width() - width, title.get_bounds(), title.clusters().len(), title.has_missing_characters());
    session.draw_shape(title);
    let line = session.draw_text("Tea \u{65E5} time", 40.0, 200.0, 24.0, TextShapeOptions { color: Some(Color::from_rgb(236, 236, 244)), missing_characters: MissingCharacterMode::Blank, ..Default::default() })?;
    for missing in line.missing_characters() {
        let _ = (missing.text_index, missing.character);
    }
    let _ = fonts.get_missing_characters("\u{05E9}", None)?;
    let mut overlay = DebugOverlay::new(Some(fonts), Some(AnimationTimeline::new(8.0, 30.0)?));
    overlay.show_cues = true;
    overlay.corner = DebugOverlayCorner::BottomRight;
    overlay.set_text_size(14.0)?;
    session.set_debug_overlay(Some(overlay));
    session.set_opacity(0.5);
    Ok(session)
}

/// The timeline of the first layer, its playback following the frame's time.
fn timeline() -> Result<PlaybackController, String> {
    let mut timeline = AnimationTimeline::new(8.0, 30.0)?;
    timeline.add_cue("move", 1.0, 3.0, EasingKind::CubicInOut)?;
    Ok(PlaybackController::new(timeline))
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let gpu = use_resource(GpuContext::request);
    let state = gpu.read();
    match &*state {
        None => rsx! { p { "Asking the browser for a WebGPU adapter..." } },
        Some(Err(reason)) => rsx! { WebGpuUnavailable { reason: reason.clone(), class: "notice" } },
        Some(Ok(context)) => rsx! { Scene { context: context.clone() } },
    }
}

#[component]
fn Scene(context: GpuContext) -> Element {
    let clock = use_hook(|| {
        let clock = CanvasClock::new();
        clock.define_point("title in", 2.0);
        clock
    });
    let layers = use_hook(|| {
        let session = Rc::new(RefCell::new(session().and_then(|mut s| animated(&mut s).and_then(|_| pictures(&mut s)).and_then(|_| blocks_and_formations(&mut s)).and_then(|_| planes_and_painting(&mut s)).map(|_| s)).unwrap_or_else(|_| DrawingSession::new(DrawingSessionOptions::default()))));
        let lettering = text(Vec::new()).unwrap_or_else(|_| DrawingSession::new(DrawingSessionOptions::default()));
        let playback = Rc::new(RefCell::new(timeline().ok()));
        let ramp = ColorRamp::from_colors(&[Color::from_rgb(214, 96, 120), Color::from_rgb(200, 140, 90), Color::from_rgb(120, 90, 210)]).ok();
        let shapes = AnimationLayer::new({
            let session = session.clone();
            move |frame, scene| {
                let mut playback = playback.borrow_mut();
                let mut s = session.borrow_mut();
                if let Some(playback) = playback.as_mut() {
                    playback.seek(frame.info.time % playback.duration());
                    let mut marked = playback.timeline().clone();
                    let _ = marked.add_marker_at_point("title", frame.points, "title in");
                    let moved = marked.progress("move", playback.time()).unwrap_or(0.0);
                    s.set_opacity(Easing::lerp_f32(0.4, 1.0, Easing::smooth_step(moved)));
                }
                if let (Some(ramp), Some(layer)) = (ramp.as_ref(), s.get_layer_mut("Shapes")) {
                    layer.set_color(ramp.at(Easing::evaluate(EasingKind::SineInOut, (frame.info.time / 4.0).fract())));
                }
                s.set_time(frame.info.time);
                let _ = s.draw_circle(640.0, 360.0, 20.0 + 10.0 * Easing::back_out_with((frame.info.time % 1.0).min(1.0), 2.0) as f32, Some(2.0), Some(Color::WHITE), false);
                s.render(scene, frame.info.width, frame.info.height);
            }
        });
        let direct = AnimationLayer::new(move |frame, scene| {
            let fit = CanvasCalibration::get_drawing_rect(frame.info.width as f32, frame.info.height as f32, (1280, 720));
            let to_view = Affine::translate((fit.x as f64, fit.y as f64)) * Affine::scale(fit.width as f64 / 1280.0);
            let turn = Affine::rotate_about(frame.info.time, Point::new(200.0, 400.0));
            let sweep = Gradient::new_sweep((200.0, 400.0), 0.0, std::f32::consts::TAU).with_stops([PenikoColor::from_rgb8(255, 0, 0), PenikoColor::from_rgb8(0, 0, 255), PenikoColor::from_rgb8(255, 0, 0)]);
            scene.fill(Fill::NonZero, to_view, &Brush::Gradient(sweep), Some(turn), &Circle::new((200.0, 400.0), 60.0));
            scene.push_layer(BlendMode::new(Mix::Multiply, Compose::SrcOver), 0.8, to_view, &Rect::new(300.0, 300.0, 500.0, 500.0));
            scene.fill(Fill::EvenOdd, to_view, &Brush::Solid(PenikoColor::from_rgb8(40, 160, 220)), None, &Circle::new((400.0, 400.0), 80.0));
            scene.pop_layer();
            scene.push_clip_layer(to_view, &Circle::new((640.0, 160.0), 50.0));
            let steps = Easing::steps((frame.info.time % 2.0) / 2.0, 4, StepPosition::End).unwrap_or(0.0);
            let eased = CubicBezierEasing::EASE.evaluate(steps);
            scene.fill(Fill::NonZero, to_view, &Brush::Solid(PenikoColor::from_rgb8(250, 200, 80)), None, &Rect::new(560.0 + 80.0 * eased, 100.0, 720.0, 220.0));
            scene.pop_layer();
            let mut lettering = lettering.clone();
            lettering.set_time(frame.info.time);
            let part = RectangleF::new(0.0, 0.0, frame.info.width as f32 / 2.0, frame.info.height as f32 / 2.0);
            lettering.render_with(scene, frame.info.width, frame.info.height, &SessionRenderOptions { include_background: false, target: Some(part) });
        });
        let control = direct.control();
        let _ = control.stats();
        CanvasLayers::new(vec![RendererHandle::new(shapes), RendererHandle::new(direct)])
    });
    rsx! {
        WebGpuCanvas { context: context.clone(), layers, clock, class: "canvas" }
    }
}
