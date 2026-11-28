//! Host tests of the TARGET WINDOW through the renderer's CPU half (target_window.rs; Not in C#):
//! what a frame's internal targets must cover, the window the builder picks for it, the effect
//! passes' texture sizes in it, and that nothing else the GPU half is handed changes with it (the
//! vertices, paints, operations and pass data are in canvas pixels either way). BOTH MODES: the
//! default (canvas-sized targets, today's frame) and the opt-in content-sized one, each switched
//! explicitly where a test is about one of them.

use kurbo::{Affine, Rect};
use peniko::color::{AlphaColor, Srgb};
use peniko::{Brush, Fill};

use crate::effects::effect_sizes::EffectParams;
use crate::rendering::core::build::{level_rect, Builder, FxStep};
use crate::rendering::core::target_window::{TargetWindow, WINDOW_ALIGN};
use crate::rendering::VectorScene;

fn solid(r: f32, g: f32, b: f32) -> Brush {
    Brush::Solid(AlphaColor::<Srgb>::new([r, g, b, 1.0]))
}

/// A small drawing at `at`: a square, and a square with a glow and a soft shadow beside it.
fn flourish(scene: &mut VectorScene, at: (f64, f64)) {
    let (x, y) = at;
    scene.fill(Fill::NonZero, Affine::IDENTITY, &solid(0.2, 0.5, 0.8), None, &Rect::new(x, y, x + 40.0, y + 40.0));
    let effects = crate::effects::DrawingEffects::glow(12.0, crate::Color::WHITE, 1.0).unwrap().with_shadow(crate::PointF::new(6.0, 6.0), 8.0, crate::Color::BLACK).unwrap();
    scene.push_effects(EffectParams::new(&effects, Affine::IDENTITY, 0.0).unwrap(), 0.8);
    scene.fill(Fill::NonZero, Affine::IDENTITY, &solid(0.9, 0.3, 0.1), None, &Rect::new(x + 44.0, y + 10.0, x + 64.0, y + 30.0));
    scene.pop_layer();
}

fn build(scene: &VectorScene, size: (u32, u32), sized: bool) -> Builder {
    let mut b = Builder::default();
    b.set_content_sized_targets(sized);
    b.build(scene, size.0, size.1);
    b
}

fn contains(w: &TargetWindow, r: Rect) -> bool {
    r.x0 >= w.x as f64 && r.y0 >= w.y as f64 && r.x1 <= (w.x + w.width) as f64 && r.y1 <= (w.y + w.height) as f64
}

/// The two builds hand the GPU half the same frame but for the window and the sized passes' v4.
fn same_frame_but_the_window(on: &Builder, off: &Builder) {
    assert_eq!(on.stencil, off.stencil);
    assert_eq!(on.cover, off.cover);
    assert_eq!(on.paints, off.paints);
    assert_eq!(on.ops, off.ops);
    assert_eq!(on.fx_kernels, off.fx_kernels);
    assert_eq!(on.fx_passes.len(), off.fx_passes.len());
    for (a, b) in on.fx_passes.iter().zip(&off.fx_passes) {
        let (mut a, mut b) = (*a, *b);
        if a.v4 != b.v4 && a.v1[0] >= 1.0 {
            // A resolve, down or blur pass: its target texture's size, the window's at its level.
            let level = a.v1[1] as u32;
            let (tw, th) = on.window.level_size(level);
            assert_eq!([a.v4[0], a.v4[1]], [tw as f32, th as f32]);
            let (cw, ch) = off.window.level_size(level);
            assert_eq!([b.v4[0], b.v4[1]], [cw as f32, ch as f32]);
            a.v4 = [0.0; 4];
            b.v4 = [0.0; 4];
        }
        assert_eq!(a, b);
    }
}

#[test]
fn a_drawing_that_fills_the_canvas_keeps_canvas_sized_targets_and_the_very_same_frame() {
    //Arrange: a background over the whole canvas and the flourish on it.
    let mut scene = VectorScene::new();
    scene.fill(Fill::NonZero, Affine::IDENTITY, &solid(0.9, 0.9, 0.85), None, &Rect::new(0.0, 0.0, 1283.0, 721.0));
    flourish(&mut scene, (600.0, 300.0));
    //Act
    let on = build(&scene, (1283, 721), true);
    let off = build(&scene, (1283, 721), false);
    //Assert: the canvas's window either way, and every value handed to the GPU half equal.
    assert_eq!(on.window, TargetWindow::canvas(1283, 721));
    assert_eq!(off.window, on.window);
    same_frame_but_the_window(&on, &off);
    assert_eq!(on.fx_passes, off.fx_passes, "with the canvas's window the sizes are the canvas's too");
}

#[test]
fn a_small_drawing_on_a_large_canvas_gets_a_window_holding_its_effects_reach() {
    //Arrange
    let mut scene = VectorScene::new();
    flourish(&mut scene, (3000.0, 1800.0));
    //Act
    let on = build(&scene, (3840, 2160), true);
    let off = build(&scene, (3840, 2160), false);
    //Assert: a small window, aligned, inside the canvas.
    let w = on.window;
    assert_eq!(off.window, TargetWindow::canvas(3840, 2160));
    assert_eq!((w.x % WINDOW_ALIGN, w.y % WINDOW_ALIGN), (0, 0));
    assert!(w.width < 400 && w.height < 400, "{w:?}");
    // It holds the root content, each effect's region grown by a pixel and every effect level's
    // texels the passes write and read.
    assert!(contains(&w, on.root_content));
    let e = &on.effects[0];
    assert!(contains(&w, e.region.inflate(1.0, 1.0)));
    for step in &e.steps {
        let levels = match step {
            FxStep::Resolve { .. } => vec![1],
            FxStep::Down { src, dst, .. } | FxStep::Blur { src, dst, .. } => vec![src.1, dst.1],
        };
        for level in levels {
            let t = level_rect(e.region, level, 3840, 2160);
            let k = level as f64;
            assert!(contains(&w, Rect::new(t.x0 * k, t.y0 * k, t.x1 * k, t.y1 * k)), "level {level}");
        }
    }
    // The frame itself is the canvas-sized one's: only the window and the passes' sizes differ.
    same_frame_but_the_window(&on, &off);
    // The targets: 20 bytes a pixel a level (colour and stencil, 4 samples) - in proportion to
    // the drawing, not the canvas.
    assert!(w.level_bytes(on.levels_needed) * 100 < off.window.level_bytes(off.levels_needed), "{} vs {}", w.level_bytes(on.levels_needed), off.window.level_bytes(off.levels_needed));
}

#[test]
fn content_partly_off_the_canvas_is_clipped_and_an_empty_frame_gets_the_smallest_window() {
    //Arrange: the flourish over the canvas's bottom right corner.
    let mut scene = VectorScene::new();
    flourish(&mut scene, (1250.0, 700.0));
    //Act
    let on = build(&scene, (1283, 721), true);
    //Assert: the window ends at the canvas's edges.
    let w = on.window;
    assert_eq!((w.x + w.width, w.y + w.height), (1283, 721));
    assert!(contains(&w, on.root_content.intersect(Rect::new(0.0, 0.0, 1283.0, 721.0))));
    same_frame_but_the_window(&on, &build(&scene, (1283, 721), false));
    // Nothing on the canvas (wholly off it, or nothing at all): the smallest window, no final quad.
    for scene in [VectorScene::new(), {
        let mut s = VectorScene::new();
        flourish(&mut s, (5000.0, 5000.0));
        s
    }] {
        let b = build(&scene, (3840, 2160), true);
        assert_eq!(b.window, TargetWindow { x: 0, y: 0, width: 16, height: 16 });
        assert!(b.final_quad.is_empty());
    }
}

#[test]
fn the_window_is_kept_from_frame_to_frame_while_the_drawing_moves_a_little() {
    //Arrange: the content-sized mode.
    let mut b = Builder::default();
    b.set_content_sized_targets(true);
    let mut windows = Vec::new();
    //Act: the flourish wobbling by up to 12 pixels over 200 frames.
    for frame in 0..200 {
        let mut scene = VectorScene::new();
        let d = (frame % 7) as f64 * 2.0;
        flourish(&mut scene, (2000.0 + d, 1000.0 - d));
        b.build(&scene, 3840, 2160);
        if windows.last() != Some(&b.window) {
            windows.push(b.window);
        }
    }
    //Assert: one window, made once.
    assert_eq!(windows.len(), 1, "{windows:?}");
}

// ---- stroke checkpoints: a checkpoint is a copy of a target at its place ----

fn painting() -> crate::shapes::BrushStrokesShape {
    use crate::strokes::{BrushStrokeRecord, BrushStrokeSet, StrokeCheckpoints};
    let records = (0..60)
        .map(|i| {
            let mut r = BrushStrokeRecord::new(crate::PointF::new(40.0 + (i % 10) as f32 * 12.0, 40.0 + (i / 10) as f32 * 12.0), 20.0, 6.0, 10.0, crate::Color::from_rgb(200, 90, 40));
            r.start = i as f32 * 0.1;
            r.end = i as f32 * 0.1 + 1.0;
            r
        })
        .collect();
    let set = BrushStrokeSet::new(records).unwrap();
    crate::shapes::BrushStrokesShape::new(set, crate::shapes::BrushStrokesOptions::default()).unwrap().with_checkpoints(Some(StrokeCheckpoints::default())).unwrap()
}

/// The checkpoint key of the session's stroke draw, built by `b` at `time` (no background).
fn key_of(b: &mut Builder, s: &mut crate::DrawingSession, time: f64) -> u64 {
    s.set_time(time);
    let mut scene = VectorScene::new();
    s.render(&mut scene, 3840, 2160);
    b.build(&scene, 3840, 2160);
    b.stroke_ops[0].checkpoint.unwrap().key
}

#[test]
fn a_checkpoint_key_names_the_target_window_so_a_copy_is_never_restored_at_another_place() {
    //Arrange: a painting in the top left of a large canvas, no background.
    let options = crate::DrawingSessionOptions { calibration_size: (3840, 2160), layer_opacity: 255, ..crate::DrawingSessionOptions::default() };
    let shape = painting();
    let mut s = crate::DrawingSession::new(options.clone());
    s.add_layer("Paint", crate::Color::WHITE);
    s.draw_brush_strokes(shape.clone());
    let mut b = Builder::default();
    b.set_content_sized_targets(true);
    //Act + Assert: while the window stays, the key stays (the painting is restored frame to frame).
    let first = key_of(&mut b, &mut s, 3.0);
    let window = b.window;
    assert!(window.width < 400, "{window:?}");
    assert_eq!(key_of(&mut b, &mut s, 4.0), first);
    assert_eq!(b.window, window);
    // A shape drawn AFTER the set, far away, moves the targets: the key changes (a copy made at
    // the old place is never restored at the new one) ...
    let mut far = crate::DrawingSession::new(options.clone());
    far.add_layer("Paint", crate::Color::WHITE);
    far.draw_brush_strokes(shape);
    far.draw_circle(3500.0, 2000.0, 20.0, None, Some(crate::Color::from_rgb(250, 0, 0)), true).unwrap();
    let moved = key_of(&mut b, &mut far, 4.0);
    assert_ne!(b.window, window);
    assert_ne!(moved, first);
    // ... and stays while the targets stay there.
    assert_eq!(key_of(&mut b, &mut far, 5.0), moved);
    // In the default mode (canvas-sized targets) the window is the canvas, frame after frame (the
    // root clear, which the key has always named, already changes it when a shape after the set
    // changes the root box).
    let mut c = Builder::default();
    assert!(!c.content_sized_targets());
    let plain = key_of(&mut c, &mut s, 4.0);
    assert_eq!(key_of(&mut c, &mut s, 5.0), plain);
    assert_eq!(c.window, TargetWindow::canvas(3840, 2160));
    assert_ne!(plain, first, "a key is made for the place its copy was taken at");
}

#[test]
fn the_default_mode_hands_the_gpu_half_todays_frame() {
    //Arrange: a small drawing with effects on a large canvas - the case the opt-in mode shrinks.
    let mut scene = VectorScene::new();
    flourish(&mut scene, (3000.0, 1800.0));
    //Act: the builder as an application gets it (no switch called), and switched off explicitly.
    let mut default = Builder::default();
    default.build(&scene, 3840, 2160);
    let off = build(&scene, (3840, 2160), false);
    //Assert: the default is off; the window is the canvas.
    assert!(!default.content_sized_targets());
    assert_eq!(default.window, TargetWindow::canvas(3840, 2160));
    // The target sizes are the canvas's: every resolve, down and blur pass's texture size (v4.xy)
    // is today's formula, the canvas divided by the level rounded up.
    assert!(!default.fx_sized.is_empty());
    for &(pass, level) in &default.fx_sized {
        let p = &default.fx_passes[pass as usize];
        assert_eq!([p.v4[0], p.v4[1]], [3840u32.div_ceil(level) as f32, 2160u32.div_ceil(level) as f32], "level {level}");
    }
    // The view uniform: the first vec4 is today's (canvas width, height, 0, 0); the new second
    // vec4 is the output's size (what vs_final maps to: the same canvas).
    assert_eq!(default.window.view_words((3840, 2160)), [3840.0, 2160.0, 0.0, 0.0, 3840.0, 2160.0, 0.0, 0.0]);
    // The very frame of the explicit off switch, pass data and all.
    same_frame_but_the_window(&default, &off);
    assert_eq!(default.fx_passes, off.fx_passes);
    assert_eq!(default.window, off.window);
}

#[test]
fn in_the_default_mode_a_checkpoint_key_never_depends_on_where_the_drawing_is() {
    //Arrange: the painting, alone and with a shape drawn after it far away.
    let options = crate::DrawingSessionOptions { calibration_size: (3840, 2160), layer_opacity: 255, ..crate::DrawingSessionOptions::default() };
    let shape = painting();
    let mut s = crate::DrawingSession::new(options.clone());
    s.add_layer("Paint", crate::Color::WHITE);
    s.draw_brush_strokes(shape.clone());
    let mut b = Builder::default();
    //Act
    let keys: Vec<u64> = (0..5).map(|k| key_of(&mut b, &mut s, 3.0 + k as f64)).collect();
    let mut fresh = Builder::default();
    let again = key_of(&mut fresh, &mut s, 9.0);
    //Assert: one key whatever the builder saw before (the window is the canvas every frame, so
    // the key's window words are a constant of the canvas size, which it named already).
    assert!(keys.iter().all(|k| *k == keys[0]));
    assert_eq!(again, keys[0]);
    assert_eq!(b.window, TargetWindow::canvas(3840, 2160));
}
