//! "water-reflection" (water A2, Not in C#): a BANK AT DUSK made in code (painted.rs
//! `dusk_bank`: a violet-to-orange sky, a low sun, far hills, trees and houses with lit windows)
//! above a waterline, MIRRORED below it by a `WaterReflection` - broken up by the water, darker and
//! bluer (the tint), smeared a little downwards (the streak) and fading to nothing at the fade
//! distance. The SAME layer also carries a `WaterRefraction`: below the line it holds the pond's
//! bed (pebbles made in code) whose alpha grows towards the viewer, so far off the reflection shows
//! and near by the bed shows through - bent by the ripples. Refraction and reflection ride the
//! layer's ONE effect group. Under it all, a plain "Deep" layer of dark water.
//!
//! The water source of the reflection is a control: the same ripples as the refraction (rain; the
//! renderer then evaluates the water ONCE per pixel), those ripples with a swell, a still height
//! picture made in code, or none (a still mirror). A click on the water drops a stone there.

use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::{RenderStats, VectorScene};
use codebrix_webgpu_animation::shapes::{ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::water::{RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, WaterReflection, WaterRefraction, WaterSource};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::painted::bank_picture;
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH};

const TITLE: &str = "Water reflection";
const DESCRIPTION: &str = "A bank at dusk mirrored below a waterline, broken up by ripples and fading with distance; the bed refracted on the same layer";
/// The bank, above the waterline (frame units).
const BANK: RectangleF = RectangleF { x: 32.0, y: 104.0, width: 1216.0, height: 300.0 };
/// The water, below it.
const WATER: RectangleF = RectangleF { x: 32.0, y: 404.0, width: 1216.0, height: 260.0 };
/// The bed picture's size.
const BED: (u32, u32) = (304, 130);

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "reflection", label: "Reflection", default: true },
    Control::Slider { key: "line", label: "Waterline (y; the shore is at 404)", min: 340.0, max: 480.0, step: 2.0, default: 404.0 },
    Control::Slider { key: "fade", label: "Fade distance", min: 50.0, max: 600.0, step: 10.0, default: 200.0 },
    Control::Slider { key: "break-up", label: "Break-up strength (the water's strength)", min: 0.0, max: 30.0, step: 0.5, default: 10.0 },
    Control::Choice {
        key: "source",
        label: "Water of the reflection",
        options: &[("the refraction's ripples (evaluated once)", 0.0), ("ripples and a swell", 1.0), ("a still height picture", 2.0), ("none: a still mirror", 3.0)],
        default: 1.0,
    },
    Control::Slider { key: "opacity", label: "Opacity at the line", min: 0.0, max: 1.0, step: 0.05, default: 0.6 },
    Control::Slider { key: "streak", label: "Streak", min: 0.0, max: 12.0, step: 0.5, default: 3.0 },
    Control::Slider { key: "tint", label: "Tint amount (alpha of 40, 70, 90)", min: 0.0, max: 255.0, step: 5.0, default: 90.0 },
    Control::Slider { key: "gap", label: "Gap below the line", min: 0.0, max: 40.0, step: 1.0, default: 0.0 },
    Control::Toggle { key: "refraction", label: "Refraction of the bed (same layer)", default: true },
    Control::Slider { key: "refraction-strength", label: "Refraction strength", min: 0.0, max: 30.0, step: 0.5, default: 8.0 },
    Control::Toggle { key: "bed", label: "The bed through the shallows", default: true },
    Control::Slider { key: "rain", label: "Rain (drops / s)", min: 0.0, max: 10.0, step: 0.25, default: 1.0 },
];

const LEGEND: &[&str] = &[
    "The bank is mirrored upside down directly below the waterline, darker and bluer (the tint), fading to nothing at the fade distance; nothing of it ever shows ABOVE the line.",
    "The swell makes it sway; raindrops (and your clicks on the water) break it up in rings; the streak smears it downwards. \"none\" is a still mirror.",
    "Near the viewer the pebbly bed shows through the water, bent by the same rings: refraction and reflection on ONE layer (one effect group). Its alpha grows towards the bottom, so far off the reflection shows and near by the bed.",
    "For the owner's eye (library defaults): opacity 0.6, fade 300 (200 here, so the fade ends inside the picture), tint (40, 70, 90, 90), streak 3, water strength 10.",
];

/// The pond's bed (made in code): sand and pebbles whose alpha grows from clear at the top (far
/// off) to almost opaque at the bottom (near by).
fn bed_picture() -> ImageSource {
    let (w, h) = BED;
    let mut image = DrawingPlaneImage::new(w, h).unwrap();
    image.fill_with(|x, y| {
        let (u, t) = (x as f32 / w as f32, y as f32 / h as f32);
        let pebbles = ((u * 61.0 + (t * 23.0).sin() * 2.0).sin() * (t * 47.0 + (u * 31.0).cos() * 2.0).sin()).abs();
        let dark = (pebbles > 0.75) as i32 as f32;
        let sand = [0.52 + 0.08 * (u * 13.0).sin(), 0.46, 0.34];
        let c = [sand[0] * (1.0 - 0.45 * dark), sand[1] * (1.0 - 0.40 * dark), sand[2] * (1.0 - 0.30 * dark)];
        let s = ((t - 0.1) / 0.9).clamp(0.0, 1.0);
        [c[0], c[1], c[2], s * s * (3.0 - 2.0 * s) * 0.92]
    });
    ImageSource::from_plane_image(&image).unwrap()
}

/// A still height picture (made in code): soft horizontal bands, a little wavy (red 128 = still).
fn height_picture() -> ImageSource {
    let (w, h) = (128u32, 64u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = (128.0 + 90.0 * ((y as f32 * 0.9 + (x as f32 * 0.15).sin() * 2.0).sin())) as u8;
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    ImageSource::from_rgba_vec(rgba, w, h).unwrap()
}

/// The dark water under everything: a vertical gradient.
fn deep_picture() -> ImageSource {
    let mut image = DrawingPlaneImage::new(4, 64).unwrap();
    image.fill_with(|_, y| {
        let t = y as f32 / 63.0;
        [0.16 + (0.03 - 0.16) * t, 0.20 + (0.06 - 0.20) * t, 0.30 + (0.10 - 0.30) * t, 1.0]
    });
    ImageSource::from_plane_image(&image).unwrap()
}

pub struct WaterReflectionScene {
    frame: DrawingSession,
    water: DrawingSession,
    settings: Settings,
    bank: ImageSource,
    bed: ImageSource,
    heights: ImageSource,
    stones: Vec<RippleDrop>,
    rain: Option<RippleField>,
    dirty: bool,
    now: f64,
    stats: RenderStats,
    error: Option<String>,
    once: bool,
}

impl WaterReflectionScene {
    pub fn new(fonts: &FontRegistry) -> WaterReflectionScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "The bank's reflection below the line; the bed refracted on the same layer. Click the water to drop a stone.", WATER.x, WATER.bottom() + 26.0, 14.0, DIM, TextAlign::Left);
        let mut water = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        water.add_layer("Deep", Color::WHITE).add_shape(ImageShape::new(deep_picture(), WATER, ImageShapeOptions { sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap());
        water.add_layer("Scene", Color::WHITE);
        let bank = ImageSource::from_plane_image(&bank_picture()).unwrap();
        WaterReflectionScene { frame, water, settings: Settings::from_controls(CONTROLS), bank, bed: bed_picture(), heights: height_picture(), stones: Vec::new(), rain: None, dirty: true, now: 0.0, stats: RenderStats::default(), error: None, once: false }
    }

    fn rebuild(&mut self) -> Result<(), String> {
        let s = self.settings.clone();
        let mut rain = RippleField::new(WATER, RippleOptions { walls: true, ..RippleOptions::default() })?.with_drops(vec![RippleDrop { center: PointF::new(WATER.x + 0.62 * WATER.width, WATER.y + 0.35 * WATER.height), time: 0.5, strength: 1.0, size: 1.2 }])?.with_drops(self.stones.clone())?;
        if s.get("rain") > 0.0 {
            rain = rain.with_rain(RippleRain::new(s.get("rain"), 3))?;
        }
        let swell = rain.clone().with_swell(RippleSwell::gentle(5, 140.0, 0.5))?;
        let source = match s.get("source") as u32 {
            0 => Some(WaterSource::Ripples(rain.clone())),
            1 => Some(WaterSource::Ripples(swell)),
            2 => Some(WaterSource::Picture { picture: self.heights.clone(), place: WATER, height: 1.5 }),
            _ => None,
        };
        self.once = s.on("refraction") && (s.get("source") as u32) == 0;
        let layer = self.water.get_layer_mut("Scene").expect("made in new");
        layer.clear();
        layer.add_shape(ImageShape::new(self.bank.clone(), BANK, ImageShapeOptions { sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap());
        if s.on("bed") {
            layer.add_shape(ImageShape::new(self.bed.clone(), WATER, ImageShapeOptions { sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap());
        }
        let reflection = s.on("reflection").then(|| WaterReflection {
            gap: s.get("gap"),
            opacity: s.get("opacity"),
            fade_distance: s.get("fade"),
            tint: Color::from_rgba(40, 70, 90, s.get("tint") as u8),
            streak: s.get("streak"),
            water: source,
            water_strength: s.get("break-up"),
            ..WaterReflection::new(s.get("line"))
        });
        layer.set_reflection(reflection)?;
        let refraction = s.on("refraction").then(|| WaterRefraction { strength: s.get("refraction-strength"), ..WaterRefraction::new(WaterSource::Ripples(rain.clone())) });
        layer.set_refraction(refraction)?;
        self.rain = Some(rain);
        Ok(())
    }
}

impl DemoScene for WaterReflectionScene {
    fn name(&self) -> &'static str {
        "water-reflection"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        16.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.now = seconds;
        if self.dirty {
            self.dirty = false;
            self.error = self.rebuild().err();
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.water.set_time(seconds);
        self.water.render(scene, width, height);
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
        self.dirty = true;
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn pointer_down(&mut self, x: f32, y: f32) -> bool {
        let inside = x >= WATER.x && x <= WATER.right() && y >= WATER.y && y <= WATER.bottom();
        if inside {
            self.stones.push(RippleDrop { center: PointF::new(x, y), time: self.now, strength: 1.2, size: 1.2 });
            self.dirty = true;
        }
        inside
    }

    fn see_stats(&mut self, stats: &RenderStats) {
        self.stats = *stats;
    }

    fn readouts(&self) -> Vec<Readout> {
        vec![
            Readout::new("time-exact", "Scene time (exact)", self.now),
            Readout::new("line-now", "Waterline", self.settings.get("line")),
            Readout::new("live-drops", "Live drops of the refraction's ripples now", self.rain.as_ref().map_or(0, |f| f.live_drops(self.now).len())),
            Readout::new("ripples-now", "Ripples evaluated / skipped by the renderer in the last frame", format!("{} / {}", self.stats.ripples_evaluated, self.stats.ripples_skipped)),
            Readout::new("shared", "Reflection and refraction share one water (evaluated once)", if self.once { "yes" } else { "no" }),
            Readout::new("water-errors", "Water refused", self.error.clone().unwrap_or_else(|| "no".to_string())),
        ]
    }
}
