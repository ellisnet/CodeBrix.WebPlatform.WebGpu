//! "painted-pond" (painterly P1-P3 + water A1-A2, Not in C#): THE PIECE THAT BRINGS THE ROUND
//! TOGETHER. A pond painted in code-made brush strokes - a far bank of trees with a willow and a
//! red boathouse under a soft sky, still water, lily pads with pink and white flowers, and irises
//! on the near bank at both lower corners (painted.rs `pond_pictures`: the picture and its floating
//! parts alone) - in THREE stroke sets with bristle tips, laid out by DIRECTION FIELDS
//! (`StrokeLayout::fill`): the bank along the picture's own structure (`DirectionField::from_picture`)
//! blended with a level field where the structure is weak, with small strokes where the detail map
//! is high; the water in level strokes (`DirectionGuide::Fixed { angle: 0 }`); the floating parts
//! round each pad and upright on the bank. Colours are PICKED from the picture (and, for the water,
//! from its own colour function).
//!
//! The water is the animation crate's: ONE `RippleField` over the water (gentle rain rings, a small
//! swell, no walls - a walled drop costs nine; a click on the water drops a stone) is the source of
//! the WATER layer's `WaterRefraction` (the rings read by their light on the painted water - its
//! marks have little contrast to bend - with glints) and of the BANK layer's `WaterReflection` (the
//! bank's brushwork mirrored below the waterline - no second stroke set - broken up by the same
//! rings). The floating parts lie on top, untouched by the water. The painting paints itself on in
//! the first seven seconds (bank, water, then the pads and irises), the strokes boil gently, and
//! the whole thing is a function of the scene's time: scrub it and every frame comes back exactly.
//! The surface finish (impasto, on both water-carrying layers - it rides the same effect group) is
//! a switch, off by default.

use codebrix_webgpu_animation::effects::SurfaceFinish;
use codebrix_webgpu_animation::formations::FormationArea;
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::painting::{DetailMap, DirectionField, DirectionGuide, PictureFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeSize};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::planes::{DrawingPlane, DrawingPlaneImage, DrawingRandom};
use codebrix_webgpu_animation::rendering::{RenderStats, VectorScene};
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTips, StrokeBoil};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::water::{RippleDrop, RippleField, RippleOptions, RippleRain, RippleSwell, WaterGlints, WaterReflection, WaterRefraction, WaterSource};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, Rectangle, RectangleF};

use crate::demo::painted::{bristle_tips, bristle_tips_with_height, lily_pads, pad_at, pick, pond_pictures, pond_water_color, PICTURE_SIZE, POND_LINE};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, FRAME_HEIGHT, FRAME_WIDTH};

const TITLE: &str = "Painted pond";
const DESCRIPTION: &str = "Stroke sets laid by direction fields, the water refracting and reflecting the brushwork, gentle rain rings - scrubbable";
/// The painting (frame units, 16:9).
const PAINTING: RectangleF = RectangleF { x: 119.0, y: 100.0, width: 1042.0, height: 586.0 };

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "water", label: "Water (refraction and reflection)", default: true },
    Control::Slider { key: "rain", label: "Rain (drops / s)", min: 0.0, max: 6.0, step: 0.1, default: 0.8 },
    Control::Toggle { key: "swell", label: "Swell", default: true },
    Control::Slider { key: "refraction", label: "Refraction of the painted water", min: 0.0, max: 20.0, step: 0.5, default: 10.0 },
    Control::Slider { key: "shading", label: "Water light (shading of the ring slopes)", min: 0.0, max: 1.5, step: 0.05, default: 0.8 },
    Control::Slider { key: "rings", label: "Ring height", min: 0.5, max: 5.0, step: 0.1, default: 2.5 },
    Control::Slider { key: "reflection", label: "Reflection opacity", min: 0.0, max: 1.0, step: 0.05, default: 0.55 },
    Control::Slider { key: "break-up", label: "Reflection break-up", min: 0.0, max: 30.0, step: 0.5, default: 12.0 },
    Control::Toggle { key: "glints", label: "Glints", default: true },
    Control::Toggle { key: "boil", label: "Boil (strokes shimmer, 10 / s)", default: true },
    Control::Toggle { key: "finish", label: "Surface finish (impasto)", default: false },
    Control::Toggle { key: "paint-on", label: "Paint it on (first 7 s)", default: true },
    Control::Choice { key: "density", label: "Strokes", options: &[("standard", 1.0), ("dense", 2.0)], default: 1.0 },
];

const LEGEND: &[&str] = &[
    "The painting paints itself on (bank, then water, then the pads and irises), then rain rings open on the water: lit on the slopes that face the light, with small glints, the painted water marks bending under them.",
    "The bank's brushwork is mirrored below the waterline (the SAME strokes, no second set), broken up by the same rings; the pads, flowers and irises float on top, untouched.",
    "Click the water to drop a stone. Drag the scene time: every frame comes back exactly (painting-on, boil and water are functions of time).",
    "For the owner's eye: is this the kind of animation you want to make? The stroke sizes, the ring height (2.5), water light (shading 0.8, light at 60 degrees, glints 1 / 40), refraction (10) and reflection (0.55, break-up 12) chosen here - painted water has little contrast to bend, so the rings read by their light, the boil, and the finish switched on.",
];

/// The set of the strokes made for one density.
struct Built {
    density: f32,
    bank: BrushStrokeSet,
    water: BrushStrokeSet,
    floating: BrushStrokeSet,
}

/// The waterline (frame units).
fn line_y() -> f32 {
    PAINTING.y + POND_LINE * PAINTING.height
}

/// The water's rectangle (frame units).
fn water_area() -> RectangleF {
    RectangleF::new(PAINTING.x, line_y(), PAINTING.width, PAINTING.bottom() - line_y())
}

fn to_color(c: [f32; 3]) -> Color {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_rgb(byte(c[0]), byte(c[1]), byte(c[2]))
}

/// When a stroke paints on: `at` seconds, for about half a second; or always whole.
fn timed(records: &mut [BrushStrokeRecord], paint_on: bool, at: &dyn Fn(&BrushStrokeRecord, f32) -> f32) {
    let mut random = DrawingRandom::new(9);
    for r in records.iter_mut() {
        r.tip = 1 + (r.seed % 4) as u16;
        if paint_on {
            r.start = at(r, random.next_single());
            r.end = r.start + 0.5;
        } else {
            r.start = BrushStrokeRecord::ALWAYS;
            r.end = BrushStrokeRecord::ALWAYS;
        }
    }
}

fn build(density: f32, paint_on: bool) -> Built {
    let (picture, floating) = pond_pictures();
    let p = PAINTING;
    let line = line_y();
    let area = |r: RectangleF| FormationArea::from_geometry(&PathGeometry::rectangle(r), None).unwrap();
    let scale = if density > 1.5 { 0.72 } else { 1.0 };
    // THE BANK: the picture's structure, a level field where it is weak, small strokes at edges.
    let structure = DirectionField::from_picture(&picture, p, PictureFieldOptions { field_size: Some((96, 54)), smoothing: 2.0 }).unwrap();
    let level = DirectionField::from_guides((24, 14), p, &[DirectionGuide::Fixed { angle: 0.0, center: PointF::new(p.x, p.y), reach: f32::INFINITY, weight: 1.0 }]).unwrap();
    let mut weak = DrawingPlane::new(96, 54).unwrap();
    weak.fill_with(|x, y| 1.0 - structure.strength_at(p.x + (x as f32 + 0.5) * p.width / 96.0, p.y + (y as f32 + 0.5) * p.height / 54.0));
    let field = structure.blend(&level, &weak).unwrap();
    let detail = DetailMap::from_picture_edges(&picture, 2.0).unwrap();
    let bank_rect = RectangleF::new(p.x, p.y, p.width, line - p.y + 3.0);
    let small = StrokeSize { min_detail: 0.35, variation: 0.25, ..StrokeSize::new(8.0 * scale, 3.0 * scale, 4.5 * scale) };
    let options = StrokeLayoutOptions { sizes: vec![StrokeSize { variation: 0.25, ..StrokeSize::new(18.0 * scale, 7.0 * scale, 8.0 * scale) }, small], angle_jitter: 6.0, seed: 11, ..Default::default() };
    let mut bank: Vec<BrushStrokeRecord> = StrokeLayout::fill(&area(bank_rect), &field, Some((&detail, p)), &|x, y| pick(&picture, p, x, y), &options).unwrap().into_iter().flatten().collect();
    timed(&mut bank, paint_on, &|r, j| 0.3 + (r.center.x - p.x) / p.width * 2.5 + j * 0.3);
    // THE WATER: level strokes in the water's own colours.
    let water_rect = water_area();
    let water_options = StrokeLayoutOptions { sizes: vec![StrokeSize { variation: 0.3, ..StrokeSize::new(24.0 * scale, 5.0 * scale, 6.5 * scale) }], angle_jitter: 4.0, seed: 12, ..Default::default() };
    let water_color = |x: f32, y: f32| to_color(pond_water_color((x - p.x) / p.width, (y - line) / (p.bottom() - line)));
    let mut water: Vec<BrushStrokeRecord> = StrokeLayout::fill(&area(water_rect), &level, None, &water_color, &water_options).unwrap().into_iter().flatten().collect();
    timed(&mut water, paint_on, &|r, j| 2.8 + (r.center.y - line) / water_rect.height * 2.0 + j * 0.3);
    // THE FLOATING PARTS: strokes on the pads (round them) and on the near bank (upright), where
    // the floating picture is covered, in its colours.
    let pads = lily_pads();
    let (w, h) = PICTURE_SIZE;
    let (sx, sy) = (p.width / w as f32, p.height / h as f32);
    let spacing = 4.2 * scale;
    let mut random = DrawingRandom::new(21);
    let mut top = Vec::new();
    let mut y = p.y;
    let mut index = 0u32;
    while y < p.bottom() {
        let mut x = p.x;
        while x < p.right() {
            let (jx, jy) = (x + random.next_single() * spacing, y + random.next_single() * spacing);
            let (px, py) = ((jx - p.x) / sx, (jy - p.y) / sy);
            let cover = floating.get_pixel((px as u32).min(w - 1), (py as u32).min(h - 1)).map_or(0.0, |c| c[3]);
            if cover > 0.6 {
                let c = floating.get_pixel((px as u32).min(w - 1), (py as u32).min(h - 1)).unwrap();
                let (angle, length, width) = match pad_at(&pads, px, py) {
                    Some(pad) => {
                        let (dx, dy) = ((px - pad.x * w as f32) * 3.0, py - pad.y * h as f32);
                        (dy.atan2(dx).to_degrees() + 90.0, 10.0 * scale, 4.0 * scale)
                    }
                    None => (-80.0 + random.uniform(-12.0, 12.0) as f32, 12.0 * scale, 3.5 * scale),
                };
                let mut r = BrushStrokeRecord::new(PointF::new(jx, jy), length, width, angle, to_color([c[0], c[1], c[2]]));
                r.load = 0.9;
                r.seed = index;
                index += 1;
                top.push(r);
            }
            x += spacing;
        }
        y += spacing;
    }
    timed(&mut top, paint_on, &|_, j| 5.0 + j * 1.5);
    Built { density, bank: BrushStrokeSet::new(bank).unwrap(), water: BrushStrokeSet::new(water).unwrap(), floating: BrushStrokeSet::new(top).unwrap() }
}

pub struct PaintedPondScene {
    frame: DrawingSession,
    painting: DrawingSession,
    settings: Settings,
    built: Option<Built>,
    built_paint_on: bool,
    tips: BrushTips,
    tips_with_height: BrushTips,
    stones: Vec<RippleDrop>,
    field: Option<RippleField>,
    dirty: bool,
    now: f64,
    stats: RenderStats,
    error: Option<String>,
}

impl PaintedPondScene {
    pub fn new(fonts: &FontRegistry) -> PaintedPondScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "Click the water to drop a stone; drag the scene time back and forth.", PAINTING.x, PAINTING.bottom() + 22.0, 14.0, DIM, TextAlign::Left);
        let mut painting = create_session(FRAME_WIDTH, FRAME_HEIGHT, Color::TRANSPARENT, fonts);
        for name in ["Water", "Bank", "Floating"] {
            painting.add_layer(name, Color::WHITE);
        }
        PaintedPondScene { frame, painting, settings: Settings::from_controls(CONTROLS), built: None, built_paint_on: true, tips: bristle_tips(), tips_with_height: bristle_tips_with_height(), stones: Vec::new(), field: None, dirty: true, now: 0.0, stats: RenderStats::default(), error: None }
    }

    fn rebuild(&mut self) -> Result<(), String> {
        let s = self.settings.clone();
        let density = s.get("density");
        let paint_on = s.on("paint-on");
        if self.built.as_ref().map(|b| b.density) != Some(density) || self.built_paint_on != paint_on {
            self.built = Some(build(density, paint_on));
            self.built_paint_on = paint_on;
        }
        let built = self.built.as_ref().expect("just built");
        let tips = if s.on("finish") { self.tips_with_height.clone() } else { self.tips.clone() };
        let boil = s.on("boil").then(|| StrokeBoil { rate: 10.0, ..StrokeBoil::default() });
        let shape = |set: &BrushStrokeSet| BrushStrokesShape::new(set.clone(), BrushStrokesOptions { head_softness: 0.15, boil, ..BrushStrokesOptions::default() }).expect("a set").with_tips(tips.clone());
        let p = PAINTING;
        let line = line_y();
        let (picture, _) = pond_pictures();
        let (w, h) = PICTURE_SIZE;
        // THE UNDERPAINTING under each set (no gaps show): the bank's part of the picture, and the
        // water's own colours.
        let bank_under = ImageShape::new(ImageSource::from_plane_image(&picture).unwrap(), RectangleF::new(p.x, p.y, p.width, line - p.y), ImageShapeOptions { source_rectangle: Some(Rectangle::new(0, 0, w as i32, (h as f32 * POND_LINE) as i32)), sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap();
        let mut water_image = DrawingPlaneImage::new(64, 32).unwrap();
        water_image.fill_with(|x, y| {
            let c = pond_water_color(x as f32 / 63.0, y as f32 / 31.0);
            [c[0], c[1], c[2], 1.0]
        });
        let water_under = ImageShape::new(ImageSource::from_plane_image(&water_image).unwrap(), water_area(), ImageShapeOptions { sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap();
        // ONE ripple field over the water.
        let mut field = RippleField::new(water_area(), RippleOptions { walls: false, height: s.get("rings"), ..RippleOptions::default() })?.with_drops(self.stones.clone())?;
        if s.get("rain") > 0.0 {
            field = field.with_rain(RippleRain::new(s.get("rain"), 5))?;
        }
        if s.on("swell") {
            field = field.with_swell(RippleSwell::gentle(6, 180.0, 0.35))?;
        }
        let finish = s.on("finish").then(SurfaceFinish::default);
        let water_on = s.on("water");
        {
            let layer = self.painting.get_layer_mut("Water").expect("made in new");
            layer.clear();
            layer.add_shape(water_under);
            layer.set_refraction(water_on.then(|| WaterRefraction { strength: s.get("refraction"), shading: s.get("shading"), light_elevation: 60.0, glints: s.on("glints").then(|| WaterGlints { amount: 1.0, sharpness: 40.0, ..WaterGlints::default() }), ..WaterRefraction::new(WaterSource::Ripples(field.clone())) }))?;
            layer.set_surface_finish(finish)?;
        }
        self.painting.set_active_layer("Water");
        self.painting.draw_brush_strokes(shape(&built.water));
        {
            let layer = self.painting.get_layer_mut("Bank").expect("made in new");
            layer.clear();
            layer.add_shape(bank_under);
            layer.set_reflection(water_on.then(|| WaterReflection { opacity: s.get("reflection"), streak: 4.0, water: Some(WaterSource::Ripples(field.clone())), water_strength: s.get("break-up"), ..WaterReflection::new(line) }))?;
            layer.set_surface_finish(finish)?;
        }
        self.painting.set_active_layer("Bank");
        self.painting.draw_brush_strokes(shape(&built.bank));
        self.painting.get_layer_mut("Floating").expect("made in new").clear();
        self.painting.set_active_layer("Floating");
        self.painting.draw_brush_strokes(shape(&built.floating));
        self.field = Some(field);
        Ok(())
    }
}

impl DemoScene for PaintedPondScene {
    fn name(&self) -> &'static str {
        "painted-pond"
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
        self.now = seconds;
        if self.dirty {
            self.dirty = false;
            self.error = self.rebuild().err();
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render(scene, width, height);
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
        let water = water_area();
        let inside = x >= water.x && x <= water.right() && y >= water.y && y <= water.bottom();
        if inside {
            self.stones.push(RippleDrop { center: PointF::new(x, y), time: self.now, strength: 1.0, size: 1.1 });
            self.dirty = true;
        }
        inside
    }

    fn see_stats(&mut self, stats: &RenderStats) {
        self.stats = *stats;
    }

    fn readouts(&self) -> Vec<Readout> {
        let counts = self.built.as_ref().map_or("-".to_string(), |b| format!("bank {}, water {}, floating {}", b.bank.len(), b.water.len(), b.floating.len()));
        vec![
            Readout::new("time-exact", "Scene time (exact)", self.now),
            Readout::new("sets", "Strokes per set", counts),
            Readout::new("live-drops", "Live drops now", self.field.as_ref().map_or(0, |f| f.live_drops(self.now).len())),
            Readout::new("ripples-now", "Ripples evaluated / skipped by the renderer in the last frame", format!("{} / {}", self.stats.ripples_evaluated, self.stats.ripples_skipped)),
            Readout::new("pond-errors", "Refused", self.error.clone().unwrap_or_else(|| "no".to_string())),
        ]
    }
}
