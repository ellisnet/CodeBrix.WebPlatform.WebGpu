//! "paint-override" (a feature of the second GO, Not in C#): ONE session - a rich drawing with
//! gradients, a picture, text, an SVG document, a glowing sun, a blurred cloud, a masked group and
//! a "Logo" layer - drawn three times side by side in one frame with three looks
//! (`SessionRenderOptions::paint`): as authored, as a LINE drawing (`SessionPaint::Line`, one
//! line style, optionally on paper) and FLAT (`SessionPaint::Flat`, each shape one colour from its
//! own fill). The effects switch (`effects`) and a layer kept as authored (`authored_layers`) are
//! controls. The session is never changed: only the render options differ.

use std::sync::Arc;

use codebrix_webgpu_animation::effects::{DrawingEffects, DrawingMask};
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeStyle, PathStrokeStyleOptions, StrokeWidthMode};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, DrawingShape, ImageShape, ImageShapeOptions, PathShape, RectangleShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::fills::RadialGradientFill;
use codebrix_webgpu_animation::svg::{DrawingSvg, DrawingSvgAspect, DrawingSvgOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, LinePaint, PointF, RectangleF, SessionPaint, SessionRenderOptions};

use crate::demo::kit::{disc, fill_rect, part};
use crate::demo::painted::picture;
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Paint override: as authored, line, flat";
const DESCRIPTION: &str = "One session drawn three ways in one frame with SessionRenderOptions::paint - the scene is never changed; effects off and an authored layer are switches";
/// The three panels (frame units).
pub const PANELS: [RectangleF; 3] = [RectangleF { x: 32.0, y: 140.0, width: 384.0, height: 216.0 }, RectangleF { x: 448.0, y: 140.0, width: 384.0, height: 216.0 }, RectangleF { x: 864.0, y: 140.0, width: 384.0, height: 216.0 }];
const LARGE: RectangleF = RectangleF { x: 256.0, y: 130.0, width: 768.0, height: 432.0 };
/// The logo's box in the drawing's own units (640 x 360).
pub const LOGO: RectangleF = RectangleF { x: 520.0, y: 290.0, width: 108.0, height: 58.0 };

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 60">
<path d="M5 55 L30 15 L45 35 L60 10 L95 55 Z" fill="#6d7fa8" stroke="#2b3550" stroke-width="2"/>
<path d="M52 22 L60 10 L68 22 Z" fill="#f4f6ff"/>
</svg>"##;

const CONTROLS: &[Control] = &[
    Control::Choice { key: "show", label: "Show", options: &[("three side by side", 0.0), ("as authored", 1.0), ("line", 2.0), ("line on paper", 3.0), ("flat", 4.0)], default: 0.0 },
    Control::Slider { key: "line-width", label: "Line width", min: 0.5, max: 6.0, step: 0.5, default: 1.5 },
    Control::Toggle { key: "fixed", label: "Fixed line width", default: true },
    Control::Toggle { key: "paper", label: "Paper under the lines (middle panel)", default: false },
    Control::Toggle { key: "effects", label: "Effects (glow, blur, shadow)", default: true },
    Control::Toggle { key: "keep-logo", label: "Keep the \"Logo\" layer as authored", default: true },
];

const LEGEND: &[&str] = &[
    "LINE looks like a pen drawing: every shape and letter an outline of one weight, the picture gone; with paper, nearer shapes hide the lines behind them.",
    "FLAT looks like a poster: each shape one colour close to its gradient's / picture's middle.",
    "The masked group (the star window, bottom left) keeps exactly its masked outline in every look.",
    "Effects off removes the sun's glow, the cloud's blur and the badge's shadow - but not the layer's vignette.",
    "The Logo layer (bottom right) stays as authored in all three while kept; untick it to see it repainted too.",
];

/// The rich drawing (640 x 360 units) the looks and the wipes are shown on; `night` makes the
/// second, contrasting session of the wipe scene.
pub fn rich_session(fonts: &FontRegistry, night: bool) -> DrawingSession {
    let mut session = create_session(640, 360, if night { Color::from_rgb(8, 10, 30) } else { Color::from_rgb(170, 200, 240) }, fonts);
    let (top, bottom) = if night { (Color::from_rgb(10, 14, 48), Color::from_rgb(60, 40, 110)) } else { (Color::from_rgb(70, 130, 220), Color::from_rgb(250, 210, 160)) };
    let sky = session.add_layer("Sky", Color::WHITE);
    sky.add_shape(RectangleShape::with_fill(0.0, 0.0, 640.0, 230.0, LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(0.0, 230.0), top, bottom).unwrap().into(), 0.0).unwrap());
    let sun = DrawingShape::from(disc(110.0, 70.0, 34.0, Some(if night { Color::from_rgb(236, 236, 250) } else { Color::from_rgb(255, 220, 90) })));
    sky.add_shape(sun.with_effects(DrawingEffects::glow(26.0, if night { Color::from_rgb(160, 170, 255) } else { Color::from_rgb(255, 170, 40) }, 1.4).unwrap()));
    let cloud = DrawingShape::from(codebrix_webgpu_animation::shapes::EllipseShape::with_fill(300.0, 60.0, 80.0, 22.0, RadialGradientFill::from_colors(PointF::new(300.0, 60.0), 80.0, Color::WHITE, Color::from_rgb(200, 210, 230)).unwrap().into()).unwrap());
    let cloud = sky.add_element(cloud.with_effects(DrawingEffects::blur(6.0).unwrap()));
    // The cloud drifts (the scene moves while the clock runs).
    let drift = crate::demo::kit::points(&[(0.0, -40.0, 0.0, crate::demo::kit::L), (5.0, 40.0, 0.0, codebrix_webgpu_animation::animation::EasingKind::SineInOut), (10.0, -40.0, 0.0, codebrix_webgpu_animation::animation::EasingKind::SineInOut)]);
    session.animations_mut().bind(cloud, codebrix_webgpu_animation::models::AnimatedProperty::Position, drift.with_loop_mode(codebrix_webgpu_animation::animation::TrackLoopMode::Loop)).unwrap();
    let svg = Arc::new(DrawingSvg::from_svg(SVG, &DrawingSvgOptions::default()).unwrap());
    session.add_layer("Mountains", Color::WHITE);
    session.draw_svg(&svg, RectangleF::new(330.0, 100.0, 300.0, 180.0), DrawingSvgAspect::Meet);
    let ground = session.add_layer("Ground", Color::WHITE);
    ground.add_shape(RectangleShape::with_fill(0.0, 230.0, 640.0, 130.0, LinearGradientFill::from_colors(PointF::new(0.0, 230.0), PointF::new(0.0, 360.0), Color::from_rgb(70, 140, 60), Color::from_rgb(20, 60, 30)).unwrap().into(), 0.0).unwrap());
    let image = ImageSource::from_plane_image(&picture()).unwrap();
    ground.add_shape(ImageShape::new(image, RectangleF::new(200.0, 250.0, 160.0, 90.0), ImageShapeOptions::default()).unwrap());
    ground.add_shape(PathShape::with_style(PathGeometry::parse("M 0 236 C 120 200 220 260 330 236 S 540 210 640 240").unwrap(), PathStrokeStyle::new(3.0, PathStrokeStyleOptions { dash_pattern: Some(vec![10.0, 6.0]), ..Default::default() }).unwrap(), Some(Color::from_rgb(230, 240, 200)), None, None));
    // A group masked inside a star: a striped window.
    let mut window = DrawingGroup::new(None);
    for k in 0..6 {
        window.add(fill_rect(20.0, 250.0 + k as f32 * 16.0, 160.0, 8.0, Some(if k % 2 == 0 { Color::from_rgb(240, 90, 80) } else { Color::from_rgb(250, 230, 120) }), 0.0));
    }
    window.set_mask(Some(DrawingMask::inside(PathGeometry::star(PointF::new(100.0, 295.0), 48.0, 22.0, 5, 0.0).unwrap())));
    session.add_layer("Masked", Color::WHITE).add_shape(window);
    let text = session.add_layer("Text", Color::WHITE);
    text.add_shape(TextShape::new(if night { "Night" } else { "Day" }, 200.0, 160.0, 54.0, fonts, TextShapeOptions { color: Some(if night { Color::from_rgb(240, 240, 255) } else { Color::from_rgb(30, 40, 80) }), align: TextAlign::Left, ..Default::default() }).unwrap());
    let badge = DrawingShape::from(fill_rect(20.0, 130.0, 150.0, 34.0, Some(Color::from_rgb(250, 250, 250)), 8.0));
    text.add_shape(badge.with_effects(DrawingEffects::drop_shadow(PointF::new(4.0, 5.0), 4.0, Color::from_rgba(0, 0, 0, 140)).unwrap()));
    text.add_shape(TextShape::new("a badge", 95.0, 154.0, 20.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(40, 40, 60)), align: TextAlign::Center, ..Default::default() }).unwrap());
    let logo = session.add_layer("Logo", Color::WHITE);
    logo.add_shape(fill_rect(LOGO.x, LOGO.y, LOGO.width, LOGO.height, Some(Color::from_rgb(250, 176, 72)), 10.0));
    logo.add_shape(disc(LOGO.x + 22.0, LOGO.y + 29.0, 14.0, Some(Color::from_rgb(40, 30, 70))));
    logo.add_shape(TextShape::new("LOGO", LOGO.x + 70.0, LOGO.y + 37.0, 20.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(40, 30, 70)), align: TextAlign::Center, ..Default::default() }).unwrap());
    // The frame's look: a vignette on the ground layer (kept with effects off).
    if let Some(layer) = session.get_layer_mut("Ground") {
        let vignette = codebrix_webgpu_animation::effects::FrameVignette { center_x: 320.0, center_y: 295.0, radius_x: 380.0, radius_y: 120.0, strength: 0.6, minimum: 0.3 };
        layer.set_frame_operations(Some(codebrix_webgpu_animation::effects::DrawingFrameOperations::NONE.with_vignette(vignette).unwrap()));
    }
    session
}

/// The line look of the scene's settings.
pub fn ink(width: f32, fixed: bool, paper: bool) -> LinePaint {
    let mode = if fixed { StrokeWidthMode::Fixed } else { StrokeWidthMode::ScalesWithElement };
    let paint = LinePaint::new(PathStrokeStyle::new(width, PathStrokeStyleOptions { width_mode: mode, ..Default::default() }).unwrap(), Color::from_rgb(28, 28, 40));
    if paper {
        paint.with_fill(Color::from_rgb(250, 247, 238))
    } else {
        paint
    }
}

pub struct PaintOverrideScene {
    frame: DrawingSession,
    drawing: DrawingSession,
    settings: Settings,
}

fn frame(fonts: &FontRegistry, show: usize) -> DrawingSession {
    let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
    let labels = session.add_layer("Labels", DIM);
    if show == 0 {
        for (i, name) in ["SessionPaint::AsAuthored", "SessionPaint::Line", "SessionPaint::Flat"].iter().enumerate() {
            label(labels, fonts, name, PANELS[i].x, PANELS[i].y + PANELS[i].height + 24.0, 15.0, INK, TextAlign::Left);
        }
        label(labels, fonts, "The same DrawingSession, three render_with calls with different options; nothing in the session changes between them.", 32.0, 430.0, 14.0, DIM, TextAlign::Left);
    }
    session
}

impl PaintOverrideScene {
    pub fn new(fonts: &FontRegistry) -> PaintOverrideScene {
        PaintOverrideScene { frame: frame(fonts, 0), drawing: rich_session(fonts, false), settings: Settings::from_controls(CONTROLS) }
    }

    fn options(&self, look: usize, target: RectangleF) -> SessionRenderOptions {
        let s = &self.settings;
        let paint = match look {
            1 => SessionPaint::Line(ink(s.get("line-width"), s.on("fixed"), false)),
            2 => SessionPaint::Line(ink(s.get("line-width"), s.on("fixed"), true)),
            3 => SessionPaint::Flat,
            _ => SessionPaint::AsAuthored,
        };
        let authored_layers = if s.on("keep-logo") { vec!["Logo".to_string()] } else { Vec::new() };
        SessionRenderOptions { target: Some(target), paint, effects: s.on("effects"), authored_layers, ..Default::default() }
    }
}

impl DemoScene for PaintOverrideScene {
    fn name(&self) -> &'static str {
        "paint-override"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        10.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.drawing.set_time(seconds);
        let show = self.settings.get("show") as usize;
        let paper = self.settings.on("paper");
        let renders: Vec<(usize, RectangleF)> = match show {
            0 => vec![(0, PANELS[0]), (if paper { 2 } else { 1 }, PANELS[1]), (3, PANELS[2])],
            1 => vec![(0, LARGE)],
            2 => vec![(1, LARGE)],
            3 => vec![(2, LARGE)],
            _ => vec![(3, LARGE)],
        };
        for (look, p) in renders {
            let target = part(&self.frame, p.x, p.y, p.width, p.height, width, height);
            let options = self.options(look, target);
            self.drawing.render_with(scene, width, height, &options);
        }
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        if settings.get("show") != self.settings.get("show") {
            let fonts = self.frame.fonts().clone();
            self.frame = frame(&fonts, settings.get("show") as usize);
        }
        self.settings = settings.clone();
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        let logo = |p: &RectangleF| {
            let s = p.width / 640.0;
            format!("{:.1},{:.1},{:.1},{:.1}", p.x + LOGO.x * s, p.y + LOGO.y * s, LOGO.width * s, LOGO.height * s)
        };
        vec![Readout::new("logo-boxes", "The Logo layer's box in each panel (frame units)", PANELS.iter().map(logo).collect::<Vec<_>>().join(" "))]
    }
}
