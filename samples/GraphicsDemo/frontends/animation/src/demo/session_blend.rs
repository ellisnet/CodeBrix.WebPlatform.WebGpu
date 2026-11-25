//! "session-blend" (a feature of the second GO, Not in C#): one session drawn over another in
//! each of the seven blend modes (`SessionRenderOptions::blend`) at a chosen opacity
//! (`SessionRenderOptions::opacity`). The motivating case: a TEXTURE (paper grain, drawn on the GPU
//! as noise, with bold ink shapes and lettering on it) multiplied over a PICTURE (the painterly
//! scenes' iris field, made in code). Both sessions are rendered into the SAME scene (the same
//! animation layer): the picture first, the texture over it. The eighth panel is the top session
//! alone.

use codebrix_webgpu_animation::fills::{ColorStop, LinearGradientFill};
use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::planes::DrawingNoiseKind;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{ImageShape, ImageShapeOptions, NoiseShape, NoiseShapeOptions, RectangleShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::shapes::PathShape;
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF, SessionBlendMode, SessionRenderOptions};

use crate::demo::kit::{fill_rect, part};
use crate::demo::painted::picture;
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Session blend modes and opacity";
const DESCRIPTION: &str = "A paper texture with ink shapes drawn over a picture in each SessionBlendMode, at the chosen opacity - one render_with call per session into the same scene";
/// The seven modes in the order of the panels, with their names.
pub const MODES: [(SessionBlendMode, &str); 7] = [
    (SessionBlendMode::Normal, "normal"),
    (SessionBlendMode::Multiply, "multiply"),
    (SessionBlendMode::Screen, "screen"),
    (SessionBlendMode::Overlay, "overlay"),
    (SessionBlendMode::Darken, "darken"),
    (SessionBlendMode::Lighten, "lighten"),
    (SessionBlendMode::Add, "add"),
];
/// The panels (frame units): 4 x 2, 288 x 162 each.
pub fn panel(index: usize) -> RectangleF {
    RectangleF::new(40.0 + (index % 4) as f32 * 302.0, 124.0 + (index / 4) as f32 * 262.0, 288.0, 162.0)
}
const LARGE: RectangleF = RectangleF { x: 224.0, y: 130.0, width: 832.0, height: 468.0 };

const CONTROLS: &[Control] = &[
    Control::Slider { key: "opacity", label: "Opacity of the top session", min: 0.0, max: 1.0, step: 0.05, default: 1.0 },
    Control::Toggle { key: "top-background", label: "Top session's paper background", default: true },
    Control::Choice {
        key: "show",
        label: "Show",
        options: &[("all seven", 0.0), ("normal", 1.0), ("multiply", 2.0), ("screen", 3.0), ("overlay", 4.0), ("darken", 5.0), ("lighten", 6.0), ("add", 7.0)],
        default: 0.0,
    },
];

const LEGEND: &[&str] = &[
    "Multiply darkens: black ink stays black, the white paper vanishes and the grain shades the picture (the texture-over-a-picture case).",
    "Screen lightens: white stays, black vanishes. Overlay raises contrast. Darken / lighten pick per channel. Add brightens towards white.",
    "Normal at opacity 1 is the plain render. Dragging the opacity fades the WHOLE top picture as one (overlapping ink shapes do not show through each other).",
    "Switching the paper background off: the render opacity then fades only the ink and grain.",
];

pub struct SessionBlendScene {
    frame: DrawingSession,
    picture: DrawingSession,
    texture: DrawingSession,
    settings: Settings,
}

fn picture_session(fonts: &FontRegistry) -> DrawingSession {
    let mut session = create_session(640, 360, Color::BLACK, fonts);
    let image = ImageSource::from_plane_image(&picture()).unwrap();
    session.add_layer("Picture", Color::WHITE).add_shape(ImageShape::new(image, RectangleF::new(0.0, 0.0, 640.0, 360.0), ImageShapeOptions::default()).unwrap());
    session
}

fn texture_session(fonts: &FontRegistry) -> DrawingSession {
    let mut session = create_session(640, 360, Color::from_rgb(244, 238, 226), fonts);
    let grain = NoiseShapeOptions { stops: vec![ColorStop::new(0.0, Color::from_rgba(150, 140, 125, 190)).unwrap(), ColorStop::new(0.55, Color::from_rgba(235, 228, 214, 60)).unwrap(), ColorStop::new(1.0, Color::from_rgba(255, 255, 255, 0)).unwrap()], ..Default::default() };
    session.add_layer("Grain", Color::WHITE).add_shape(NoiseShape::new(0.0, 0.0, 640.0, 360.0, DrawingNoiseKind::Gradient, 11, 0.06, grain).unwrap());
    let fonts = session.fonts().clone();
    let ink = session.add_layer("Ink", Color::BLACK);
    ink.add_shape(crate::demo::kit::disc(170.0, 180.0, 110.0, Some(Color::from_rgb(20, 20, 26))));
    ink.add_shape(fill_rect(250.0, 70.0, 180.0, 140.0, Some(Color::from_rgb(210, 50, 40)), 12.0));
    ink.add_shape(RectangleShape::with_fill(60.0, 300.0, 520.0, 34.0, LinearGradientFill::from_colors(PointF::new(60.0, 0.0), PointF::new(580.0, 0.0), Color::from_rgb(30, 60, 200), Color::from_rgb(250, 250, 250)).unwrap().into(), 6.0).unwrap());
    ink.add_shape(PathShape::with_fill_color(PathGeometry::star(PointF::new(520.0, 150.0), 80.0, 34.0, 5, 0.0).unwrap(), Color::WHITE));
    // A small ink disc travelling along the bottom (the scene moves while the clock runs).
    let traveller = ink.add_element(crate::demo::kit::disc(0.0, 0.0, 16.0, Some(Color::from_rgb(20, 20, 26))));
    ink.add_shape(TextShape::new("ink on paper", 450.0, 270.0, 38.0, &fonts, TextShapeOptions { color: Some(Color::from_rgb(16, 16, 20)), align: TextAlign::Center, ..Default::default() }).unwrap());
    let path = crate::demo::kit::points(&[(0.0, 40.0, 345.0, crate::demo::kit::L), (5.0, 600.0, 345.0, codebrix_webgpu_animation::animation::EasingKind::SineInOut), (10.0, 40.0, 345.0, codebrix_webgpu_animation::animation::EasingKind::SineInOut)]);
    session.animations_mut().bind(traveller, codebrix_webgpu_animation::models::AnimatedProperty::Position, path.with_loop_mode(codebrix_webgpu_animation::animation::TrackLoopMode::Loop)).unwrap();
    session
}

fn frame(fonts: &FontRegistry, show: usize) -> DrawingSession {
    let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
    let labels = session.add_layer("Labels", DIM);
    if show == 0 {
        for (i, (_, name)) in MODES.iter().enumerate() {
            let p = panel(i);
            label(labels, fonts, &format!("SessionBlendMode::{}", name[..1].to_uppercase() + &name[1..]), p.x, p.y + p.height + 22.0, 14.0, INK, TextAlign::Left);
        }
        let p = panel(7);
        label(labels, fonts, "the top session alone (no picture under it)", p.x, p.y + p.height + 22.0, 14.0, DIM, TextAlign::Left);
    } else {
        label(labels, fonts, &format!("SessionBlendMode::{}", MODES[show - 1].1), LARGE.x, LARGE.y + LARGE.height + 26.0, 16.0, INK, TextAlign::Left);
    }
    session
}

impl SessionBlendScene {
    pub fn new(fonts: &FontRegistry) -> SessionBlendScene {
        SessionBlendScene { frame: frame(fonts, 0), picture: picture_session(fonts), texture: texture_session(fonts), settings: Settings::from_controls(CONTROLS) }
    }
}

impl DemoScene for SessionBlendScene {
    fn name(&self) -> &'static str {
        "session-blend"
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
        self.texture.set_time(seconds);
        let opacity = self.settings.get("opacity");
        let top_background = self.settings.on("top-background");
        let show = self.settings.get("show") as usize;
        let panels: Vec<(RectangleF, Option<SessionBlendMode>)> = if show == 0 {
            let mut list: Vec<(RectangleF, Option<SessionBlendMode>)> = MODES.iter().enumerate().map(|(i, (mode, _))| (panel(i), Some(*mode))).collect();
            list.push((panel(7), None));
            list
        } else {
            vec![(LARGE, Some(MODES[show - 1].0))]
        };
        for (p, mode) in panels {
            let target = part(&self.frame, p.x, p.y, p.width, p.height, width, height);
            match mode {
                Some(blend) => {
                    self.picture.render_with(scene, width, height, &SessionRenderOptions { target: Some(target), ..Default::default() });
                    self.texture.render_with(scene, width, height, &SessionRenderOptions { include_background: top_background, target: Some(target), blend, opacity, ..Default::default() });
                }
                None => self.texture.render_with(scene, width, height, &SessionRenderOptions { include_background: top_background, target: Some(target), opacity, ..Default::default() }),
            }
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
        vec![Readout::new("panels", "Panels (frame units, left to right, top to bottom)", (0..8).map(|i| { let p = panel(i); format!("{},{},{},{}", p.x, p.y, p.width, p.height) }).collect::<Vec<_>>().join(" "))]
    }
}
