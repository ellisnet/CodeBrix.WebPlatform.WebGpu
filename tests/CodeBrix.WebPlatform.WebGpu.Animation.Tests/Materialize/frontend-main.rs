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
//! from its backend or its own assets; a variable font's instance at chosen axis values), a line of text with a gradient fill and the
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
//! and the safe-area overlay; and the RENDER-TIME LOOKS: a fixed-width (non-scaling) stroke on a
//! path and a text block's outline and an SVG with vector-effect="non-scaling-stroke", a session
//! drawn with a blend mode and an opacity, as a line drawing (fixed-width line style, paper
//! fill), flat, without effects and with an authored layer, a soft wipe (straight edge and iris)
//! between one session's two looks and between two sessions, an ownership map re-coloured with
//! with_colors (and the upload counts of RenderStats), and a sketched path boiling with the
//! clock time; and THE PAINTERLY PLANNING AND COLOUR (the marked PAINTERLY P2 block): direction
//! fields, a detail map, a stroke layout and a painting timeline into a stroke set, colour rules,
//! perceptual mixing, Oklab / Oklch ramps and gradients and a colour bound as a function of time;
//! and LIVE COLOUR SOURCES AND THE SURFACE FINISH (the marked PAINTERLY P3 block): strokes picking
//! their colours from a picture, a picture target drawn by its own layer and an application
//! texture, and a finished layer; and CHECKPOINTS AND STROKE TRANSITIONS (the marked PAINTERLY
//! P4 block): a still painting painted on with its checkpoint cache, its counters and refusal, and
//! a stroke-level transition between two sets with a spread, its fraction bound to a track; and
//! WATER (the marked WATER A2 block): a layer seen through a ripple field with glints and caustic
//! light, a layer mirrored below a waterline, height pictures as water (decoded pixels, a picture
//! target, an application texture) and a session seen through water.

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
use codebrix_webgpu_animation::text::{FontRegistry, FontVariation, MeasureTextOptions, MissingCharacterMode, TextAlign};
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
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, LinePaint, PointF, Rectangle, RectangleF, SessionBlendMode, SessionPaint, SessionRenderOptions, SessionWipe, WipeEdge};
use codebrix_webgpu_animation::paths::{SketchOptions, StrokeWidthMode};
use dioxus::prelude::*;

/// A curve of the application's own as a constant (CubicBezierEasing::new is a const fn).
const SNAP: CubicBezierEasing = CubicBezierEasing::new(0.2, 0.0, 0.0, 1.0).unwrap();

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
    let _ = codebrix_webgpu_animation::images::ImageSource::from_jpeg(&[0xFF, 0xD8]).err();
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
    let family = fonts.register_font(font_bytes.clone(), Some("Body"))?;
    // A variable font at a chosen weight (refused here: the buffer is not a variable font).
    let _ = fonts.register_font_with_variations(font_bytes, Some("Body Bold"), &[FontVariation::weight(700.0), FontVariation::new(*b"wdth", 90.0)]).err();
    let _ = fonts.get_variation_axes(Some("Body")).map(|axes| axes.iter().map(|a| (a.tag_text(), a.min_value, a.default_value, a.max_value)).count());
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

/// The second GO's drawing side: fixed-width strokes (a path, a text block's outline, an SVG's
/// vector-effect), an ownership map made once and re-coloured with with_colors, a sketched path.
fn looks(session: &mut DrawingSession, time: f64) -> Result<(), String> {
    use codebrix_webgpu_animation::shapes::{OwnershipShape, OwnershipShapeOptions, TextBlockStrokeOptions};
    use codebrix_webgpu_animation::svg::{DrawingSvg, DrawingSvgAspect, DrawingSvgOptions};
    let fixed = PathStrokeStyle::new(3.0, PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed, dash_pattern: Some(vec![12.0, 8.0]), ..Default::default() })?;
    let _ = (fixed.is_fixed_width(), fixed.width_mode(), fixed.with_width_mode(StrokeWidthMode::ScalesWithElement));
    let star = PathGeometry::star(PointF::new(200.0, 200.0), 120.0, 50.0, 5, 0.0)?;
    let layer = session.add_layer("Looks", Color::WHITE);
    layer.add_shape(DrawingShape::from(PathShape::with_style(star.clone(), fixed.clone(), Some(Color::WHITE), None, None)).with_transform(DrawingTransform::IDENTITY.with_scale_xy(1.0, 3.0)));
    // The sketch helper: the same star roughened, boiling ten times a second of clock time.
    let sketch = SketchOptions { roughness: 2.5, wobble_length: 24.0, passes: 2, seed: 7, corner_angle_degrees: 30.0 }.boiled(time, 10.0);
    layer.add_shape(PathShape::with_style(star.sketch(&sketch)?, PathStrokeStyle::new(2.0, PathStrokeStyleOptions::default())?, Some(Color::from_rgb(30, 30, 40)), None, None));
    let _ = SketchOptions::default();
    let outline = session.layout_text("Outlined", 64.0, None, None)?;
    session.draw_shape(TextBlockShape::with_stroke(outline, 40.0, 500.0, Some(fixed), TextBlockStrokeOptions { stroke_color: Some(Color::WHITE), ..Default::default() })?);
    let svg = std::sync::Arc::new(DrawingSvg::from_svg(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><line x1="1" y1="5" x2="9" y2="5" stroke="#fff" stroke-width="1" vector-effect="non-scaling-stroke"/></svg>"##, &DrawingSvgOptions::default())?);
    session.draw_svg(&svg, RectangleF::new(600.0, 100.0, 400.0, 400.0), DrawingSvgAspect::Meet);
    // An ownership map made once and re-coloured each frame: only its colour table is uploaded.
    let map = session.export_layer_ownership_map("Looks", (64, 64))?;
    let base = OwnershipShape::new(&map, &vec![Color::WHITE; map.element_count()], RectangleF::new(0.0, 0.0, 64.0, 64.0), OwnershipShapeOptions::default())?;
    let shade = (128.0 + 127.0 * time.sin()) as u8;
    let recoloured = base.with_colors(&vec![Color::from_rgb(shade, 90, 200); map.element_count()])?.with_destination(RectangleF::new(1100.0, 600.0, 128.0, 128.0))?;
    let _ = base.with_float_colors(&vec![[0.5, 0.5, 0.5, 1.0]; map.element_count()])?;
    session.add_layer("Recoloured", Color::WHITE).add_shape(recoloured);
    Ok(())
}

/// A session drawn with the second GO's render options and wipes.
fn render_looks(session: &DrawingSession, other: &DrawingSession, scene: &mut codebrix_webgpu_animation::rendering::VectorScene, width: u32, height: u32, time: f64) {
    let ink = LinePaint::new(PathStrokeStyle::new(2.0, PathStrokeStyleOptions { width_mode: StrokeWidthMode::Fixed, ..Default::default() }).unwrap_or_else(|_| unreachable!()), Color::from_rgb(30, 30, 40)).with_fill(Color::from_rgb(250, 248, 240));
    let line = SessionRenderOptions { paint: SessionPaint::Line(ink), effects: false, authored_layers: vec!["Recoloured".to_string()], ..Default::default() };
    let flat = SessionRenderOptions { paint: SessionPaint::Flat, blend: SessionBlendMode::Multiply, opacity: 0.8, ..Default::default() };
    let progress = Easing::evaluate(EasingKind::CubicInOut, ((time % 4.0) / 3.0).min(1.0)) as f32;
    SessionWipe::new(progress, WipeEdge::Line { angle_degrees: 30.0 }, 0.06 * width as f32).render_sessions(scene, width, height, (session, &SessionRenderOptions::default()), (session, &line));
    let iris = SessionWipe { progress, edge: WipeEdge::Circle { center: PointF::new(width as f32 / 2.0, height as f32 / 2.0) }, ..SessionWipe::default() };
    iris.render(scene, RectangleF::new(0.0, 0.0, width as f32, height as f32), |scene| other.render_with(scene, width, height, &flat), |scene| other.render_with(scene, width, height, &SessionRenderOptions { blend: SessionBlendMode::Screen, ..Default::default() }));
    for mode in [SessionBlendMode::Normal, SessionBlendMode::Overlay, SessionBlendMode::Darken, SessionBlendMode::Lighten, SessionBlendMode::Add] {
        let _ = SessionRenderOptions { blend: mode, ..Default::default() };
    }
}

// ---- PAINTERLY P2 BEGIN: planning and colour (direction fields from a picture, noise and
// guides; a detail map; a stroke layout band by band; a painting timeline with stages and cues;
// colour rules; perceptual mixing; Oklab / Oklch ramps and gradients; a colour bound as a
// function of the time). The strokes are drawn through their CPU twins (BrushStrokeRecord::to_shape)
// until the GPU stroke sets draw them. ----
mod painterly_p2 {
    use codebrix_webgpu_animation::animation::{AnimationTimeline, ColorRamp};
    use codebrix_webgpu_animation::fills::{LinearGradientFill, RadialGradientFill};
    use codebrix_webgpu_animation::formations::{ArrivalRateProfile, CenterOutArrivalOrder, FormationArea};
    use codebrix_webgpu_animation::models::{AnimatedProperty, BindingTarget, BoundTrack};
    use codebrix_webgpu_animation::painting::{element_area, BrokenColor, ColorInterpolation, ColorMix, ColorRules, DetailMap, DirectionField, DirectionGuide, NoiseFieldOptions, PaintPalette, PaintingStage, PaintingTimeline, PaletteMode, PictureFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeOrder, StrokeSize};
    use codebrix_webgpu_animation::paths::PathGeometry;
    use codebrix_webgpu_animation::planes::{DrawingNoiseKind, DrawingPlane, DrawingPlaneImage};
    use codebrix_webgpu_animation::shapes::{CircleShape, RectangleShape};
    use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF};

    /// Plans a painting once: fields, a detail map, a layout, colour rules and a timeline.
    pub fn plan() -> Result<(BrushStrokeSet, AnimationTimeline), String> {
        let area_rect = RectangleF::new(40.0, 360.0, 400.0, 240.0);
        let mut picture = DrawingPlaneImage::new(80, 48)?;
        picture.fill_with(|x, y| {
            let d = ((x as f32 - 40.0).powi(2) + (y as f32 - 24.0).powi(2)).sqrt();
            let v = if d < 16.0 { 0.9 } else { 0.2 };
            [v, 0.5 * v, 1.0 - v, 1.0]
        });
        let structure = DirectionField::from_picture(&picture, area_rect, PictureFieldOptions { field_size: Some((40, 24)), ..PictureFieldOptions::default() })?;
        let noise = DirectionField::from_noise((20, 12), area_rect, DrawingNoiseKind::Gradient, 7, 0.01, NoiseFieldOptions { z: Some(0.5), ..NoiseFieldOptions::default() })?;
        let centre = PointF::new(240.0, 480.0);
        let guides = DirectionField::from_guides((20, 12), area_rect, &[
            DirectionGuide::Around { center: centre, reach: 150.0, inward: 15.0, weight: 1.0 },
            DirectionGuide::Radial { center: centre, reach: 40.0, weight: 0.5 },
            DirectionGuide::Fixed { angle: 0.0, center: centre, reach: f32::INFINITY, weight: 0.2 },
            DirectionGuide::Along { path: PathGeometry::parse("M 40 590 L 440 580")?, reach: 30.0, weight: 1.0 },
        ])?;
        let half = DrawingPlane::with_value(40, 24, 0.5)?;
        let field = structure.blend(&noise, &half)?.blend(&guides, &half)?.rotated(5.0);
        let _ = (field.angle_at(100.0, 400.0), field.strength_at(100.0, 400.0), field.trace(PointF::new(100.0, 400.0), 4.0, 10), field.size(), field.destination(), field.stroke_pose(centre, 20.0));
        let detail = DetailMap::from_picture_edges(&picture, 1.5)?;
        let _ = DetailMap::sample(&detail, area_rect, 100.0, 400.0);
        let area = FormationArea::from_geometry(&PathGeometry::rectangle(area_rect), None)?;
        let rules = ColorRules {
            palette: Some(PaintPalette { pigments: vec![Color::from_rgb(30, 40, 120), Color::from_rgb(230, 200, 90), Color::from_rgb(180, 60, 50)], snap: 0.8, mode: PaletteMode::MixOfTwo }),
            broken: Some(BrokenColor { hue: 10.0, ..BrokenColor::default() }),
        };
        rules.validate()?;
        let _ = (rules.apply_float([0.5, 0.4, 0.3, 1.0], 3), PaintPalette::new(vec![Color::WHITE]));
        let colour_at = |x: f32, y: f32| rules.apply(Color::from_rgb((x / 2.0) as u8, (y / 3.0) as u8, 128), (x as u32) ^ ((y as u32) << 8));
        let options = StrokeLayoutOptions { sizes: vec![StrokeSize::new(18.0, 5.0, 12.0), StrokeSize { min_detail: 0.4, variation: 0.2, ..StrokeSize::new(7.0, 2.5, 5.0) }], ..StrokeLayoutOptions::default() };
        let bands = StrokeLayout::fill(&area, &field, Some((&detail, area_rect)), &colour_at, &options)?;
        let mut painting = PaintingTimeline::new();
        painting.add_stage(PaintingStage::new("wash", 0.0, 6.0, (0.5, 1.0)).with_order(StrokeOrder::Positional(Box::new(CenterOutArrivalOrder::new(false)))))?;
        painting.add_stage(PaintingStage::new("detail", 4.0, 10.0, (0.2, 0.4)).with_order(StrokeOrder::DarkToLight).with_profile(ArrivalRateProfile::default()))?;
        let set = painting.schedule(bands, 11)?;
        let _ = (set.len(), set.is_empty(), set.bounds(), set.anchor_bounds().len(), set.first_start(), set.started_count(3.0), set.id(), set.records().first().map(|r| r.progress_at(3.0)));
        let mut cues = AnimationTimeline::new(12.0, 60.0)?;
        painting.add_cues(&mut cues)?;
        Ok((set, cues))
    }

    /// A session showing the planned painting at a time, an Oklch ramp, Oklab and linear
    /// gradients, and a layer colour bound to an Oklab mix as a function of the time.
    pub fn draw(set: &BrushStrokeSet, time: f64) -> Result<DrawingSession, String> {
        let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, background_fill_color: Color::TRANSPARENT, ..Default::default() });
        session.add_layer("Sky", Color::BLACK);
        session.animations_mut().bind(BindingTarget::layer("Sky"), AnimatedProperty::Color, BoundTrack::color_fn(|t| ColorMix::mix(Color::from_rgb(40, 60, 200), Color::from_rgb(250, 220, 40), ((t / 10.0) % 1.0) as f32, ColorInterpolation::Oklab)))?;
        let bar = session.draw_rectangle(480.0, 360.0, 300.0, 40.0, None, None, true, 0.0)?;
        let _ = element_area(&session, bar, time)?;
        let ramp = ColorRamp::from_colors(&[Color::from_rgb(0, 0, 255), Color::from_rgb(255, 255, 0)])?.with_interpolation(ColorInterpolation::Oklch);
        let _ = ramp.interpolation();
        let linear = LinearGradientFill::from_colors(PointF::new(480.0, 420.0), PointF::new(780.0, 420.0), ramp.at(0.0), ramp.at(1.0))?.with_interpolation(ColorInterpolation::Oklab);
        let mut radial = RadialGradientFill::from_colors(PointF::new(630.0, 560.0), 60.0, Color::WHITE, Color::from_rgb(200, 30, 60))?;
        radial.set_interpolation(ColorInterpolation::LinearRgb);
        let _ = (linear.interpolation(), radial.interpolation());
        session.add_layer("Painted", Color::WHITE);
        session.draw_shape(RectangleShape::with_fill(480.0, 420.0, 300.0, 40.0, linear.into(), 0.0)?);
        session.draw_shape(CircleShape::with_fill(630.0, 560.0, 60.0, radial.into())?);
        for record in set.records().iter().filter(|r| r.has_started(time)) {
            session.draw_shape(record.to_shape(time)?);
        }
        let _ = BrushStrokeRecord::from_shape(&BrushStrokeRecord::new(PointF::new(0.0, 0.0), 10.0, 2.0, 0.0, Color::WHITE).to_shape(0.0)?);
        session.set_time(time);
        Ok(session)
    }
}
// ---- PAINTERLY P2 END ----

// ---- PAINTERLY P1 BEGIN: stroke sets drawn on the GPU (BrushStrokesShape: one instanced draw per
// set, records uploaded once), brush tips made in code with mip chains, anchors that follow a
// moving element and a fixed one with a blend between them, boiling with the shared boil timing,
// an animated flow, a soft brush head, a replay offset, the set's coverage in an ownership map, and
// the stroke counters of RenderStats. ----
mod painterly_p1 {
    use codebrix_webgpu_animation::animation::{EasingKind, PointTrack};
    use codebrix_webgpu_animation::models::{AnimatedProperty, DrawingTransform, DrawingTransformOptions};
    use codebrix_webgpu_animation::planes::DrawingPlane;
    use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, CircleShape, ShapeOptions};
    use codebrix_webgpu_animation::strokes::{boil_epoch, BrushStrokeRecord, BrushStrokeSet, BrushTip, BrushTips, StrokeAnchor, StrokeBoil, StrokeFlow};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF};

    /// Bristle ridges across the stroke: a soft edge times a ripple (an application's own tip).
    fn bristles() -> Result<BrushTips, String> {
        let (w, h) = (32u32, 16u32);
        let mut coverage = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let across = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
                let along = (x as f32 + 0.5) / w as f32;
                coverage.push((1.0 - across * across) * (0.7 + 0.3 * (across * 9.0 + along).sin()));
            }
        }
        let tip = BrushTip::new(DrawingPlane::from_data(w, h, coverage)?, None);
        let picture = codebrix_webgpu_animation::images::ImageSource::from_rgba(&[255, 255, 255, 255, 128, 128, 128, 64], 2, 1)?;
        BrushTips::new((w, h), &[tip, BrushTip::from_image(&picture, true)?])
    }

    /// A session with a moving element and a painting of a few hundred strokes painted on over
    /// ten seconds, half of them following the element.
    pub(super) fn session() -> Result<DrawingSession, String> {
        let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
        let layer = s.add_layer("Painting", Color::WHITE);
        let figure = layer.add_element(CircleShape::new(0.0, 0.0, 4.0, ShapeOptions::default())?);
        let track = PointTrack::from_keys(&[(0.0, PointF::new(0.0, 0.0), EasingKind::Linear), (10.0, PointF::new(200.0, 50.0), EasingKind::Linear)])?;
        s.animations_mut().bind(figure, AnimatedProperty::Position, track)?;
        let mut records = Vec::new();
        for i in 0..400u32 {
            let (x, y) = (100.0 + (i % 40) as f32 * 27.0, 120.0 + (i / 40) as f32 * 45.0);
            let mut r = BrushStrokeRecord::new(PointF::new(x, y), 34.0, 9.0, (i % 7) as f32 * 12.0, Color::from_rgb((i * 7 % 255) as u8, 120, 200));
            r.bend = 0.08;
            r.tip = (i % 3) as u16;
            r.anchors = if i % 2 == 0 { [0, 0] } else { [1, 0] };
            r.anchor_blend = if i % 5 == 0 { 0.5 } else { 0.0 };
            r.start = x / 128.0;
            r.end = r.start + 0.8;
            r.seed = i;
            r.load = 0.9;
            records.push(r);
        }
        let set = BrushStrokeSet::new(records)?;
        let options = BrushStrokesOptions { time_offset: 0.5, head_softness: 0.15, tip_mirroring: true, boil: Some(StrokeBoil { rate: 10.0, ..StrokeBoil::default() }), flow: Some(StrokeFlow { turn: 20.0, ..StrokeFlow::default() }) };
        let fixed = DrawingTransform::new(PointF::new(0.0, 20.0), DrawingTransformOptions::default())?;
        let shape = BrushStrokesShape::new(set.clone(), options)?.with_tips(bristles()?).with_anchors(vec![StrokeAnchor::Fixed(fixed), StrokeAnchor::FollowElement(figure)])?;
        let _ = (shape.get_bounds(), shape.set().started_count(2.0), shape.coverage_areas_at(2.0).len(), boil_epoch(2.0, 10.0));
        s.draw_brush_strokes(shape);
        // The same set again, plain (shares the one upload).
        s.draw_brush_strokes(BrushStrokesShape::new(set, BrushStrokesOptions::default())?);
        let _ = s.export_ownership_map((64, 36))?;
        Ok(s)
    }
}
// ---- PAINTERLY P1 END ----

// ---- WATER A1 BEGIN: a ripple field as a function of time (drops, a click's drop, seeded rain,
// a gentle swell, walls), read on the CPU (height, slope, focus, live drops, ring radius) to float
// a marker; and the caustics noise kind drawn on the GPU (NoiseShape, fractal, z = time) and
// computed on the CPU (DrawingNoise::caustics, a plane fill). ----
mod water_a1 {
    use codebrix_webgpu_animation::fills::ColorStop;
    use codebrix_webgpu_animation::planes::{DrawingNoise, DrawingNoiseFillOptions, DrawingNoiseKind, DrawingPlane};
    use codebrix_webgpu_animation::shapes::{NoiseShape, NoiseShapeOptions};
    use codebrix_webgpu_animation::water::{RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, SwellWave};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF};

    /// The pool's water: a drop, a "click" at 3 s, rain at 2 drops per second, a gentle swell
    /// plus one wave of its own, walls on.
    pub(super) fn field() -> Result<RippleField, String> {
        let options = RippleOptions { walls: true, ..RippleOptions::default() };
        let mut swell = RippleSwell::gentle(4, 160.0, 0.6);
        swell.waves.push(SwellWave { angle: 120.0, ..SwellWave::default() });
        RippleField::new(RectangleF::new(100.0, 100.0, 1080.0, 520.0), options)?
            .with_drops(vec![RippleDrop { center: PointF::new(400.0, 300.0), time: 0.5, strength: 0.8, size: 1.2 }])?
            .with_drop(RippleDrop::new(PointF::new(700.0, 400.0), 3.0))?
            .with_rain(RippleRain { rate: 2.0, end: Some(600.0), ..RippleRain::new(2.0, 1) })?
            .with_swell(swell)
    }

    /// The frame at a time: a caustics floor drawn on the GPU and a marker floating on the water.
    pub(super) fn draw(field: &RippleField, time: f64) -> Result<DrawingSession, String> {
        let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
        let place = field.place();
        let stops = vec![ColorStop::new(0.0, Color::from_rgb(10, 40, 90)).ok_or("stop")?, ColorStop::new(0.75, Color::from_rgb(110, 180, 220)).ok_or("stop")?, ColorStop::new(1.0, Color::WHITE).ok_or("stop")?];
        let fill = DrawingNoiseFillOptions { z: Some(time * 0.3), octaves: 2, lacunarity: 1.7, gain: 0.6, ..DrawingNoiseFillOptions::default() };
        s.add_layer("Pool", Color::WHITE).add_shape(NoiseShape::new(place.x, place.y, place.width, place.height, DrawingNoiseKind::Caustics, 7, 0.02, NoiseShapeOptions { fill, stops })?);
        // The marker rides the water: lifted by the height, pushed by the slope, brighter where the
        // light focuses.
        let (x, y) = (640.0, 360.0);
        let (sx, sy) = field.slope_at(x, y, time);
        let lift = field.height_at(x, y, time);
        let light = (128.0 - 400.0 * field.focus_at(x, y, time)).clamp(0.0, 255.0) as u8;
        let _ = s.draw_circle(x + 20.0 * sx, y + 20.0 * sy - 4.0 * lift, 10.0, None, Some(Color::from_rgb(light, light, 255)), true);
        // Read-outs a scene would show; the CPU pattern of the same kind.
        let live = field.live_drops(time);
        let rings = live.iter().filter_map(|d| field.ring_radius(d, time)).count();
        let _ = (rings, field.drop_life(), field.options().speed, field.drops().len(), field.rain().map(|r| r.rate), field.swell().map(|w| w.waves.len()));
        let _ = DrawingNoise::caustics(1.0, 2.0, time, 7) + DrawingNoise::sample_3d(DrawingNoiseKind::Caustics, 1.0, 2.0, time, 7);
        let mut plane = DrawingPlane::new(16, 16)?;
        DrawingNoise::fill(&mut plane, DrawingNoiseKind::Caustics, 7, 0.1, DrawingNoiseFillOptions { z: Some(time), ..DrawingNoiseFillOptions::default() })?;
        Ok(s)
    }
}
// ---- WATER A1 END ----

// ---- PAINTERLY P3 BEGIN: live colour sources and the surface finish (strokes that PICK their
// colours on the GPU every frame from a picture with colour rules - palette and broken colour -,
// from a PictureTarget another session draws in the same frame through its own
// AnimationLayer::to_picture layer listed below its reader, and from a texture the application
// made on the canvas host's device; the target shown by ImageShape::from_picture_target; a layer
// with a SurfaceFinish whose strokes' tips have height, over the canvas weave). ----
mod painterly_p3 {
    use std::cell::RefCell;
    use std::rc::Rc;

    use codebrix_webgpu::{wgpu, GpuContext, RendererHandle};
    use codebrix_webgpu_animation::effects::{CanvasWeave, SurfaceFinish};
    use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
    use codebrix_webgpu_animation::painting::{BrokenColor, ColorRules, PaintPalette, PaletteMode};
    use codebrix_webgpu_animation::planes::DrawingPlane;
    use codebrix_webgpu_animation::rendering::{AnimationLayer, PictureTarget};
    use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape};
    use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTip, BrushTips, ExternalPicture, PictureEncoding, StrokeColor, StrokeColorSource, StrokePickup};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF};

    fn options() -> DrawingSessionOptions {
        DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, background_fill_color: Color::from_rgb(120, 170, 220), ..Default::default() }
    }

    /// A tip whose height rises in ridges across the stroke (the finish lights it).
    fn ridged() -> Result<BrushTips, String> {
        let (w, h) = (32u32, 16u32);
        let (mut coverage, mut height) = (Vec::new(), Vec::new());
        for y in 0..h {
            for _ in 0..w {
                let across = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
                coverage.push(1.0 - across * across);
                height.push((1.0 - across * across) * (0.6 + 0.4 * (across * 8.0).sin().abs()));
            }
        }
        BrushTips::new((w, h), &[BrushTip::new(DrawingPlane::from_data(w, h, coverage)?, Some(DrawingPlane::from_data(w, h, height)?))])
    }

    fn strokes(area: RectangleF) -> Result<BrushStrokeSet, String> {
        let records = (0..600u32).map(|i| {
            let (x, y) = (area.x + (i % 30) as f32 / 30.0 * area.width, area.y + (i / 30) as f32 / 20.0 * area.height);
            let mut r = BrushStrokeRecord::new(PointF::new(x, y), 28.0, 8.0, (i * 37 % 180) as f32, Color::WHITE);
            r.tip = 1;
            r.seed = i;
            r
        });
        BrushStrokeSet::new(records.collect())
    }

    fn picked(set: BrushStrokeSet, tips: &BrushTips, source: StrokeColorSource, rules: Option<ColorRules>) -> Result<BrushStrokesShape, String> {
        let pickup = StrokePickup { taps: 5, own_weight: 0.1, rules, ..StrokePickup::new(source, RectangleF::new(0.0, 0.0, 1280.0, 720.0)) };
        BrushStrokesShape::new(set, BrushStrokesOptions::default())?.with_tips(tips.clone()).with_color(StrokeColor::Picked(pickup))
    }

    /// The picture layer (FIRST: below its reader) and the reader.
    pub(super) fn layers(context: &GpuContext) -> Result<Vec<RendererHandle>, String> {
        // Another session, drawn into a picture every frame.
        let target = PictureTarget::new(640, 360)?;
        let mut sky = DrawingSession::new(options());
        sky.add_layer("Sky", Color::WHITE);
        sky.draw_circle(900.0, 200.0, 120.0, None, Some(Color::from_rgb(250, 210, 90)), true)?;
        sky.draw_rectangle(0.0, 480.0, 1280.0, 240.0, None, Some(Color::from_rgb(60, 120, 70)), true, 0.0)?;
        let picture = AnimationLayer::to_picture(target.clone(), move |frame, scene| {
            sky.set_time(frame.info.time);
            sky.render_with_background(scene, frame.info.width, frame.info.height);
        });
        let _ = picture.picture().map(|p| p.size());
        // A texture of the application's (a video frame it copied in, a 3D render it drew).
        let texture = context.device().create_texture(&wgpu::TextureDescriptor { label: Some("application picture"), size: wgpu::Extent3d { width: 2, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        context.queue().write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &[200, 60, 40, 255, 40, 60, 200, 255], wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(1) }, wgpu::Extent3d { width: 2, height: 1, depth_or_array_layers: 1 });
        let external = ExternalPicture { view: texture.create_view(&wgpu::TextureViewDescriptor::default()), size: (2, 1), encoding: PictureEncoding::Linear };
        // The reader: a finished layer of three picked sets and the target shown.
        let tips = ridged()?;
        let rules = ColorRules { palette: Some(PaintPalette { pigments: vec![Color::from_rgb(30, 40, 110), Color::from_rgb(230, 200, 70), Color::from_rgb(170, 50, 40), Color::from_rgb(240, 235, 220)], snap: 0.7, mode: PaletteMode::MixOfTwo }), broken: Some(BrokenColor { hue: 10.0, chroma: 0.12, lightness: 0.02, seed: 3 }) };
        let gradient = ImageSource::from_rgba(&[220, 80, 60, 255, 60, 80, 220, 255, 240, 220, 90, 255, 40, 140, 70, 255], 2, 2)?;
        let mut s = DrawingSession::new(options());
        let layer = s.add_layer("Live", Color::WHITE);
        layer.set_surface_finish(Some(SurfaceFinish { light_angle: 225.0, light_elevation: 35.0, relief: 0.6, impasto: 1.0, gloss: 0.15, weave: Some(CanvasWeave { spacing: 4.0, ..CanvasWeave::default() }) }))?;
        let _ = layer.surface_finish();
        layer.add_shape(picked(strokes(RectangleF::new(40.0, 40.0, 380.0, 640.0))?, &tips, StrokeColorSource::Picture(gradient), Some(rules))?);
        layer.add_shape(picked(strokes(RectangleF::new(450.0, 40.0, 380.0, 640.0))?, &tips, StrokeColorSource::Target(target.clone()), None)?);
        layer.add_shape(picked(strokes(RectangleF::new(860.0, 40.0, 380.0, 640.0))?, &tips, StrokeColorSource::Texture(external), None)?);
        layer.add_shape(ImageShape::from_picture_target(&target, RectangleF::new(1000.0, 560.0, 240.0, 135.0), ImageSampling::Linear)?);
        let session = Rc::new(RefCell::new(s));
        let reader = AnimationLayer::new(move |frame, scene| {
            let mut s = session.borrow_mut();
            s.set_time(frame.info.time);
            s.render(scene, frame.info.width, frame.info.height);
            let _ = s.take_reports();
        });
        Ok(vec![RendererHandle::new(picture), RendererHandle::new(reader)])
    }
}
// ---- PAINTERLY P3 END ----

// ---- PAINTERLY P4 BEGIN: the checkpoint cache of a still painting (StrokeCheckpoints with its
// spacing and capacity, the refusal it explains, its RenderStats counters) and a stroke-level
// transition between two stroke sets (BrushStrokeMorph::pair with its options, the slots, the CPU
// stroke of a slot, BrushStrokesShape::morph with a left-to-right spread, MorphFraction bound to a
// track - scrubbable). ----
mod painterly_p4 {
    use codebrix_webgpu_animation::animation::{EasingKind, FloatTrack};
    use codebrix_webgpu_animation::formations::DirectionalArrivalOrder;
    use codebrix_webgpu_animation::models::AnimatedProperty;
    use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape};
    use codebrix_webgpu_animation::strokes::{BrushStrokeMorph, BrushStrokeRecord, BrushStrokeSet, StrokeCheckpoints, StrokePairingOptions};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF};

    /// A grid of strokes painted on over `span` seconds (record order is paint order).
    fn strokes(n: u32, offset: f32, color: Color, span: f32) -> Result<BrushStrokeSet, String> {
        let records = (0..n).map(|i| {
            let mut r = BrushStrokeRecord::new(PointF::new(40.0 + (i % 48) as f32 * 25.0 + offset, 40.0 + (i / 48) as f32 * 25.0), 26.0, 8.0, (i * 53 % 180) as f32, color);
            r.start = i as f32 / n as f32 * span;
            r.end = r.start + 0.8;
            r.seed = i;
            r
        });
        BrushStrokeSet::new(records.collect())
    }

    /// A still painting with its checkpoint cache, and a transition between two paintings whose
    /// fraction follows a track from 0 at 2 s to 1 at 8 s.
    pub(super) fn session() -> Result<DrawingSession, String> {
        let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, background_fill_color: Color::from_rgb(245, 240, 228), ..Default::default() });
        s.add_layer("Easel", Color::WHITE);
        let cache = StrokeCheckpoints { spacing: 200, capacity: 4 };
        let _ = (StrokeCheckpoints::bytes_at(2560, 1440), StrokeCheckpoints::BYTES_PER_PIXEL, cache.validate());
        let easel = BrushStrokesShape::new(strokes(1200, 0.0, Color::from_rgb(70, 90, 160), 20.0)?, BrushStrokesOptions::default())?.with_checkpoints(Some(cache))?;
        let _ = (easel.checkpoints(), easel.checkpoint_refusal());
        s.draw_brush_strokes(easel);
        let noon = strokes(600, 0.0, Color::from_rgb(230, 200, 90), 0.0)?;
        let dusk = strokes(500, 9.0, Color::from_rgb(90, 50, 120), 0.0)?;
        let morph = BrushStrokeMorph::pair(&noon, &dusk, StrokePairingOptions { search_radius: 40.0, color_weight: 0.5, seed: 3 })?;
        let _ = (morph.len(), morph.paired_count(), morph.pairs().first().map(|p| (p.from, p.to)), morph.record_at(0, 0.5), morph.from().len(), morph.to().len(), morph.seed(), morph.is_empty());
        s.add_layer("Transition", Color::WHITE);
        let spread = Box::new(DirectionalArrivalOrder::new(0.0, 0.0)?);
        let shape = BrushStrokesShape::morph(morph, BrushStrokesOptions::default(), Some((spread, 0.4)))?.with_morph_fraction(0.0);
        let _ = (shape.transition().map(|m| m.len()), shape.morph_fraction());
        let id = s.draw_brush_strokes(shape);
        let track = FloatTrack::from_keys(&[(2.0, 0.0, EasingKind::Linear), (8.0, 1.0, EasingKind::CubicInOut)])?;
        s.animations_mut().bind(id, AnimatedProperty::MorphFraction, track)?;
        Ok(s)
    }
}
// ---- PAINTERLY P4 END ----

// ---- WATER A2 BEGIN: refraction and reflection (a pool floor seen THROUGH a ripple field - drops,
// rain, swell, walls - with glints and a caustic light; a bank mirrored below a waterline, broken
// up by the same field; height pictures as the water: decoded pixels, a picture target drawn this
// frame below its reader, an application's Rgba16Float texture; a whole session seen through water
// inside a multiply blend; the RenderStats ripple counters). ----
mod water_a2 {
    use std::cell::RefCell;
    use std::rc::Rc;

    use codebrix_webgpu::{wgpu, GpuContext, RendererHandle};
    use codebrix_webgpu_animation::images::ImageSource;
    use codebrix_webgpu_animation::rendering::{AnimationLayer, PictureTarget};
    use codebrix_webgpu_animation::strokes::{ExternalPicture, PictureEncoding};
    use codebrix_webgpu_animation::water::{CausticLight, RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, WaterGlints, WaterReflection, WaterRefraction, WaterSource};
    use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions, PointF, RectangleF, SessionBlendMode, SessionRenderOptions};

    fn options() -> DrawingSessionOptions {
        DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() }
    }

    fn field() -> Result<RippleField, String> {
        RippleField::new(RectangleF::new(0.0, 0.0, 1280.0, 720.0), RippleOptions { walls: true, ..RippleOptions::default() })?
            .with_drop(RippleDrop::new(PointF::new(640.0, 360.0), 1.0))?
            .with_rain(RippleRain::new(2.0, 1))?
            .with_swell(RippleSwell::gentle(4, 160.0, 0.6))
    }

    /// The layers: the picture target's layer FIRST (below its reader), then the water scene.
    pub(super) fn layers(context: &GpuContext) -> Result<Vec<RendererHandle>, String> {
        let water = field()?;
        let mut s = DrawingSession::new(options());
        // A pool floor made in code, seen through the ripples.
        let floor = s.add_layer("Floor", Color::WHITE);
        let refraction = WaterRefraction { strength: 12.0, shading: 0.3, light_angle: 225.0, light_elevation: 45.0, glints: Some(WaterGlints { amount: 0.8, sharpness: 80.0, color: Color::from_rgb(255, 245, 225) }), caustics: Some(CausticLight { seed: 3, ..CausticLight::default() }), ..WaterRefraction::new(WaterSource::Ripples(water.clone())) };
        refraction.validate()?;
        floor.set_refraction(Some(refraction))?;
        let _ = (floor.refraction().map(|r| r.source.is_exact_in_time()), WaterGlints::default(), CausticLight::default());
        s.draw_rectangle(0.0, 0.0, 1280.0, 720.0, None, Some(Color::from_rgb(70, 140, 200)), true, 0.0)?;
        s.draw_circle(640.0, 360.0, 200.0, Some(8.0), Some(Color::from_rgb(240, 230, 190)), false)?;
        // A bank mirrored below the waterline, broken up by the same field.
        let bank = s.add_layer("Bank", Color::WHITE);
        bank.set_reflection(Some(WaterReflection { gap: 4.0, opacity: 0.6, fade_distance: 300.0, tint: Color::from_rgba(40, 70, 90, 90), streak: 3.0, water: Some(WaterSource::Ripples(water)), water_strength: 10.0, ..WaterReflection::new(300.0) }))?;
        let _ = bank.reflection().map(|r| r.validate());
        s.draw_rectangle(100.0, 120.0, 400.0, 180.0, None, Some(Color::from_rgb(60, 130, 70)), true, 20.0)?;
        // Height pictures: decoded pixels, a picture target drawn this frame, an application's texture.
        let heights = ImageSource::from_rgba(&[128, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 255, 128, 0, 0, 255], 2, 2)?;
        s.add_layer("Pictured", Color::WHITE).set_refraction(Some(WaterRefraction::new(WaterSource::Picture { picture: heights, place: RectangleF::new(700.0, 100.0, 400.0, 200.0), height: 2.0 })))?;
        s.draw_rectangle(700.0, 100.0, 400.0, 200.0, None, Some(Color::from_rgb(200, 120, 60)), true, 0.0)?;
        let target = PictureTarget::new(320, 180)?;
        let mut waves = DrawingSession::new(DrawingSessionOptions { background_fill_color: Color::from_rgb(128, 128, 128), ..options() });
        waves.add_layer("Waves", Color::WHITE);
        waves.draw_circle(640.0, 360.0, 160.0, None, Some(Color::WHITE), true)?;
        let picture = AnimationLayer::to_picture(target.clone(), move |frame, scene| {
            waves.set_time(frame.info.time);
            waves.render_with_background(scene, frame.info.width, frame.info.height);
        });
        s.add_layer("Targeted", Color::WHITE).set_refraction(Some(WaterRefraction::new(WaterSource::Target { picture: target, place: RectangleF::new(100.0, 420.0, 400.0, 200.0), height: 3.0 })))?;
        s.draw_rectangle(100.0, 420.0, 400.0, 200.0, None, Some(Color::from_rgb(90, 60, 160)), true, 0.0)?;
        // The application's own texture (as a graphics wave field's surface would be): R = height.
        let texture = context.device().create_texture(&wgpu::TextureDescriptor { label: Some("application heights"), size: wgpu::Extent3d { width: 2, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba16Float, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        context.queue().write_texture(wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &[0, 0, 0, 0, 0, 0, 0, 60, 0, 60, 0, 0, 0, 0, 0, 60], wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(16), rows_per_image: Some(1) }, wgpu::Extent3d { width: 2, height: 1, depth_or_array_layers: 1 });
        let external = ExternalPicture { view: texture.create_view(&wgpu::TextureViewDescriptor::default()), size: (2, 1), encoding: PictureEncoding::Linear };
        s.add_layer("Simulated", Color::WHITE).set_refraction(Some(WaterRefraction::new(WaterSource::Texture { picture: external, place: RectangleF::new(700.0, 420.0, 400.0, 200.0), height: 4.0 })))?;
        s.draw_rectangle(700.0, 420.0, 400.0, 200.0, None, Some(Color::from_rgb(40, 160, 150)), true, 0.0)?;
        let session = Rc::new(RefCell::new(s));
        let whole = WaterRefraction { strength: 6.0, ..WaterRefraction::new(WaterSource::Ripples(field()?)) };
        let reader = AnimationLayer::new(move |frame, scene| {
            let mut s = session.borrow_mut();
            s.set_time(frame.info.time);
            s.render(scene, frame.info.width, frame.info.height);
            // The whole session again, seen through water inside a multiply blend.
            s.render_with(scene, frame.info.width, frame.info.height, &SessionRenderOptions { blend: SessionBlendMode::Multiply, opacity: 0.5, refraction: Some(whole.clone()), ..SessionRenderOptions::default() });
            let _ = s.take_reports();
        });
        let stats = reader.control().stats();
        let _ = (stats.ripples_evaluated, stats.ripples_skipped, RippleField::MAX_LIVE_DROPS);
        Ok(vec![RendererHandle::new(picture), RendererHandle::new(reader)])
    }
}
// ---- WATER A2 END ----

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
            let eased = CubicBezierEasing::EASE.evaluate(steps).max(SNAP.evaluate(0.0));
            scene.fill(Fill::NonZero, to_view, &Brush::Solid(PenikoColor::from_rgb8(250, 200, 80)), None, &Rect::new(560.0 + 80.0 * eased, 100.0, 720.0, 220.0));
            scene.pop_layer();
            let mut lettering = lettering.clone();
            lettering.set_time(frame.info.time);
            let part = RectangleF::new(0.0, 0.0, frame.info.width as f32 / 2.0, frame.info.height as f32 / 2.0);
            lettering.render_with(scene, frame.info.width, frame.info.height, &SessionRenderOptions { include_background: false, target: Some(part), ..Default::default() });
        });
        let looked = AnimationLayer::new(move |frame, scene| {
            let mut first = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, ..Default::default() });
            let _ = looks(&mut first, frame.info.time);
            let mut second = DrawingSession::new(DrawingSessionOptions { calibration_size: (1280, 720), layer_opacity: 255, background_fill_color: Color::from_rgb(20, 24, 40), ..Default::default() });
            let _ = second.draw_circle(640.0, 360.0, 200.0, None, Some(Color::from_rgb(240, 180, 60)), true);
            render_looks(&first, &second, scene, frame.info.width, frame.info.height, frame.info.time);
        });
        // ---- PAINTERLY P2 BEGIN: the planned painting drawn at the frame's time ----
        let planned = painterly_p2::plan().ok();
        let painted = AnimationLayer::new(move |frame, scene| {
            if let Some(Ok(session)) = planned.as_ref().map(|(set, _cues)| painterly_p2::draw(set, frame.info.time % 12.0)) {
                session.render(scene, frame.info.width, frame.info.height);
            }
        });
        // ---- PAINTERLY P2 END ----
        // ---- PAINTERLY P1 BEGIN: a stroke set drawn on the GPU at the frame's time ----
        let painting = Rc::new(RefCell::new(painterly_p1::session().ok()));
        let stroked = AnimationLayer::new(move |frame, scene| {
            if let Some(session) = painting.borrow_mut().as_mut() {
                session.set_time(frame.info.time % 12.0);
                session.render(scene, frame.info.width, frame.info.height);
            }
        });
        let strokes = stroked.control().stats();
        let _ = (strokes.stroke_draws, strokes.strokes, strokes.stroke_uploads, strokes.stroke_upload_bytes, strokes.tip_uploads, strokes.tip_upload_bytes);
        // ---- PAINTERLY P1 END ----
        // ---- WATER A1 BEGIN: the pool at the frame's time (exact under seeking) ----
        let water = water_a1::field().ok();
        let pooled = AnimationLayer::new(move |frame, scene| {
            if let Some(Ok(session)) = water.as_ref().map(|field| water_a1::draw(field, frame.info.time)) {
                session.render(scene, frame.info.width, frame.info.height);
            }
        });
        // ---- WATER A1 END ----
        // ---- PAINTERLY P3 BEGIN: a picture layer and its reader, after the other layers ----
        let live = painterly_p3::layers(&context).unwrap_or_default();
        // ---- PAINTERLY P3 END ----
        // ---- PAINTERLY P4 BEGIN: a still painting from its checkpoints and a stroke transition ----
        let easel = Rc::new(RefCell::new(painterly_p4::session().ok()));
        let painted_on = AnimationLayer::new(move |frame, scene| {
            if let Some(session) = easel.borrow_mut().as_mut() {
                session.set_time(frame.info.time % 24.0);
                session.render_with_background(scene, frame.info.width, frame.info.height);
            }
        });
        let cached = painted_on.control().stats();
        let _ = (cached.checkpoint_hits, cached.checkpoint_bakes, cached.checkpoint_misses, cached.strokes_restored, cached.checkpoints, cached.checkpoint_bytes, cached.strokes);
        let live = live.into_iter().chain(std::iter::once(RendererHandle::new(painted_on))).collect::<Vec<_>>();
        // ---- PAINTERLY P4 END ----
        // ---- WATER A2 BEGIN: the water layers (picture target first), after the other layers ----
        let live = live.into_iter().chain(water_a2::layers(&context).unwrap_or_default()).collect::<Vec<_>>();
        // ---- WATER A2 END ----
        let control = direct.control();
        let stats = control.stats();
        let _ = (stats.record_ms, stats.build_ms, stats.encode_ms, stats.cpu_ms());
        let uploads = looked.control().stats();
        let _ = (uploads.image_uploads, uploads.image_upload_bytes, uploads.color_table_uploads, uploads.color_table_upload_bytes, uploads.unsupported);
        CanvasLayers::new(vec![RendererHandle::new(shapes), RendererHandle::new(direct), RendererHandle::new(looked), RendererHandle::new(painted), RendererHandle::new(stroked), RendererHandle::new(pooled)].into_iter().chain(live).collect())
    });
    rsx! {
        WebGpuCanvas { context: context.clone(), layers, clock, class: "canvas" }
    }
}
