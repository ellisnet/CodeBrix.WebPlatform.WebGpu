//! "direction-fields" (painterly P2, drawn with P1's GPU stroke sets; Not in C#): strokes laid
//! along a DIRECTION FIELD three ways. Panel 1, STRUCTURE: a picture made in code (concentric rings
//! and an oval face with a nose line) - `DirectionField::from_picture`, blended with a noise field
//! where the structure is weak, and a DETAIL MAP (`DetailMap::from_picture_edges`) that puts a band
//! of small strokes where the picture has edges, over broad strokes everywhere. Panel 2, NOISE: a
//! sky laid by `DirectionField::from_noise`, swirling with the GPU flow (`StrokeFlow`). Panel 3,
//! GUIDES: two swirl centres (`DirectionGuide::Around`), a horizon (`Along`), a radial burst
//! (`Radial`) and a weak fixed background (`Fixed`). Switches overlay the field as short ticks and
//! the detail map as a grey wash.

use codebrix_webgpu_animation::formations::FormationArea;
use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::painting::{DetailMap, DirectionField, DirectionGuide, NoiseFieldOptions, PictureFieldOptions, StrokeLayout, StrokeLayoutOptions, StrokeSize};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::planes::{DrawingNoiseKind, DrawingPlane, DrawingPlaneImage};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokesOptions, BrushStrokesShape, ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::strokes::{BrushStrokeRecord, BrushStrokeSet, BrushTips, StrokeFlow};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{fill_rect, line};
use crate::demo::painted::{bristle_tips, pick};
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Direction fields, guides and a detail map";
const DESCRIPTION: &str = "Strokes laid along a field from a picture's structure, from noise and from hand-placed guides; small strokes where the detail map is high";
/// The three panels (frame units).
pub const PANELS: [RectangleF; 3] = [RectangleF { x: 24.0, y: 150.0, width: 390.0, height: 292.0 }, RectangleF { x: 445.0, y: 150.0, width: 390.0, height: 292.0 }, RectangleF { x: 866.0, y: 150.0, width: 390.0, height: 292.0 }];

const CONTROLS: &[Control] = &[
    Control::Slider { key: "smoothing", label: "Structure smoothing (cells)", min: 0.0, max: 8.0, step: 0.5, default: 3.0 },
    Control::Slider { key: "min-detail", label: "Small strokes where detail >=", min: 0.0, max: 1.0, step: 0.05, default: 0.35 },
    Control::Slider { key: "angle-jitter", label: "Angle jitter (degrees)", min: 0.0, max: 20.0, step: 1.0, default: 6.0 },
    Control::Slider { key: "inward", label: "Second swirl's inward turn", min: -45.0, max: 45.0, step: 5.0, default: 25.0 },
    Control::Toggle { key: "flow", label: "Flow on the noise panel", default: true },
    Control::Toggle { key: "ticks", label: "Show the field as ticks", default: false },
    Control::Toggle { key: "detail", label: "Show the detail map (panel 1)", default: false },
];

const LEGEND: &[&str] = &[
    "Panel 1: strokes follow the rings and the face's oval outline; small strokes crowd at the edges (face, nose line), broad strokes fill flat regions.",
    "Raise 'small strokes where detail >=' and the small band shrinks to the strongest edges (the detail map shows where).",
    "Panel 2: the noise field over a sky, swirling slowly with the GPU flow. Panel 3: swirls look like swirls; strokes lie along the horizon.",
    "No 90-degree flips where a direction wraps from 179 to 1 degree (the field stores doubled angles).",
];

/// The structure picture: concentric rings on the left, an oval face with a nose line on the right.
pub fn structure_picture() -> DrawingPlaneImage {
    let mut image = DrawingPlaneImage::new(160, 120).unwrap();
    image.fill_with(|x, y| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let d = ((fx - 48.0).powi(2) + (fy - 60.0).powi(2)).sqrt();
        let ring = 0.5 + 0.5 * (d * 0.55).sin();
        let mut c = [0.18 + 0.5 * ring, 0.30 + 0.35 * ring, 0.62 - 0.2 * ring];
        let e = (((fx - 116.0) / 30.0).powi(2) + ((fy - 60.0) / 42.0).powi(2)).sqrt();
        if e < 1.0 {
            c = [0.92, 0.74, 0.58];
            if e > 0.9 {
                c = [0.35, 0.20, 0.15];
            }
            let nose = (fx - 116.0 - (fy - 50.0) * 0.1).abs() < 1.2 && fy > 48.0 && fy < 74.0;
            let eyes = ((fx - 106.0).powi(2) + (fy - 48.0).powi(2)).sqrt() < 3.0 || ((fx - 126.0).powi(2) + (fy - 48.0).powi(2)).sqrt() < 3.0;
            if nose || eyes {
                c = [0.30, 0.16, 0.12];
            }
        }
        [c[0], c[1], c[2], 1.0]
    });
    image
}

struct Built {
    sets: [BrushStrokeSet; 3],
    counts: [usize; 4],
    fields: [DirectionField; 3],
    detail: DrawingPlane,
}

fn tip_all(records: &mut [BrushStrokeRecord]) {
    for r in records.iter_mut() {
        r.tip = 1 + (r.seed % 4) as u16;
    }
}

fn build(settings: &Settings, picture: &DrawingPlaneImage) -> Built {
    let jitter = settings.get("angle-jitter");
    // PANEL 1: the picture's structure, noise where it is weak, the detail map's two bands.
    let p = PANELS[0];
    let structure = DirectionField::from_picture(picture, p, PictureFieldOptions { field_size: Some((80, 60)), smoothing: settings.get("smoothing") }).unwrap();
    let noise = DirectionField::from_noise((40, 30), p, DrawingNoiseKind::Gradient, 7, 0.01, NoiseFieldOptions::default()).unwrap();
    let mut weak = DrawingPlane::new(80, 60).unwrap();
    weak.fill_with(|x, y| 1.0 - structure.strength_at(p.x + (x as f32 + 0.5) * p.width / 80.0, p.y + (y as f32 + 0.5) * p.height / 60.0));
    let field1 = structure.blend(&noise, &weak).unwrap();
    let detail = DetailMap::from_picture_edges(picture, 2.0).unwrap();
    let area = |r: RectangleF| FormationArea::from_geometry(&PathGeometry::rectangle(r), None).unwrap();
    let small = StrokeSize { min_detail: settings.get("min-detail"), variation: 0.2, ..StrokeSize::new(7.0, 2.5, 4.0) };
    let options = StrokeLayoutOptions { sizes: vec![StrokeSize::new(18.0, 5.0, 9.0), small], angle_jitter: jitter, ..Default::default() };
    let bands = StrokeLayout::fill(&area(p), &field1, Some((&detail, p)), &|x, y| pick(picture, p, x, y), &options).unwrap();
    let counts1 = (bands[0].len(), bands[1].len());
    let mut first: Vec<BrushStrokeRecord> = bands.into_iter().flatten().collect();
    tip_all(&mut first);
    // PANEL 2: a sky laid by noise (the GPU flow swirls it).
    let p = PANELS[1];
    let field2 = DirectionField::from_noise((40, 30), p, DrawingNoiseKind::Gradient, 3, 0.008, NoiseFieldOptions { turns: 0.5, ..Default::default() }).unwrap();
    let sky = |_: f32, y: f32| {
        let t = ((y - p.y) / p.height).clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| (a + (b - a) * t) as u8;
        Color::from_rgb(mix(40.0, 250.0), mix(70.0, 170.0), mix(170.0, 110.0))
    };
    let bands = StrokeLayout::fill(&area(p), &field2, None, &sky, &StrokeLayoutOptions { sizes: vec![StrokeSize::new(16.0, 5.0, 7.0)], angle_jitter: jitter, seed: 2, ..Default::default() }).unwrap();
    let mut second: Vec<BrushStrokeRecord> = bands.into_iter().flatten().collect();
    tip_all(&mut second);
    // PANEL 3: hand-placed guides.
    let p = PANELS[2];
    let horizon_y = p.y + p.height * 0.62;
    let guides = [
        DirectionGuide::Around { center: PointF::new(p.x + 100.0, p.y + 80.0), reach: 120.0, inward: 0.0, weight: 1.0 },
        DirectionGuide::Around { center: PointF::new(p.x + 290.0, p.y + 100.0), reach: 120.0, inward: settings.get("inward"), weight: 1.0 },
        DirectionGuide::Along { path: PathGeometry::parse(&format!("M {} {} L {} {}", p.x, horizon_y, p.right(), horizon_y - 10.0)).unwrap(), reach: 40.0, weight: 1.0 },
        DirectionGuide::Radial { center: PointF::new(p.x + 300.0, p.y + 250.0), reach: 50.0, weight: 1.0 },
        DirectionGuide::Fixed { angle: 0.0, center: PointF::new(p.x, p.y), reach: f32::INFINITY, weight: 0.2 },
    ];
    let field3 = DirectionField::from_guides((60, 45), p, &guides).unwrap();
    let sea = |x: f32, y: f32| {
        let swirl = ((x - p.x) * 0.05).sin() * 20.0;
        if y < horizon_y { Color::from_rgb(120, (170.0 + swirl) as u8, 230) } else { Color::from_rgb(20, (70.0 + swirl) as u8, 120) }
    };
    let bands = StrokeLayout::fill(&area(p), &field3, None, &sea, &StrokeLayoutOptions { sizes: vec![StrokeSize::new(16.0, 4.5, 6.0)], angle_jitter: jitter, seed: 3, ..Default::default() }).unwrap();
    let mut third: Vec<BrushStrokeRecord> = bands.into_iter().flatten().collect();
    tip_all(&mut third);
    let counts = [counts1.0, counts1.1, second.len(), third.len()];
    Built { sets: [BrushStrokeSet::new(first).unwrap(), BrushStrokeSet::new(second).unwrap(), BrushStrokeSet::new(third).unwrap()], counts, fields: [field1, field2, field3], detail }
}

pub struct DirectionFieldsScene {
    frame: DrawingSession,
    painting: DrawingSession,
    picture: DrawingPlaneImage,
    tips: BrushTips,
    built: Option<Built>,
    settings: Settings,
    dirty: bool,
}

impl DirectionFieldsScene {
    pub fn new(fonts: &FontRegistry) -> DirectionFieldsScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = frame.add_layer("Labels", DIM);
        for (i, text) in ["from_picture (structure) + detail map", "from_noise, with GPU flow", "from_guides: around x 2, along, radial, fixed"].iter().enumerate() {
            label(labels, fonts, text, PANELS[i].x, PANELS[i].y - 12.0, 14.0, INK, TextAlign::Left);
        }
        let mut painting = create_session(1280, 720, Color::TRANSPARENT, fonts);
        painting.add_layer("Under", Color::WHITE);
        painting.add_layer("Strokes", Color::WHITE);
        painting.add_layer("Over", Color::WHITE);
        DirectionFieldsScene { frame, painting, picture: structure_picture(), tips: bristle_tips(), built: None, settings: Settings::from_controls(CONTROLS), dirty: true }
    }

    fn refill(&mut self) {
        let built = build(&self.settings, &self.picture);
        let s = &self.settings;
        let under = self.painting.get_layer_mut("Under").unwrap();
        under.clear();
        for p in PANELS {
            under.add_shape(fill_rect(p.x, p.y, p.width, p.height, Some(Color::from_rgb(40, 42, 54)), 0.0));
        }
        let strokes = self.painting.get_layer_mut("Strokes").unwrap();
        strokes.clear();
        for (i, set) in built.sets.iter().enumerate() {
            let flow = (i == 1 && s.on("flow")).then(|| StrokeFlow { speed: 0.2, turn: 30.0, ..StrokeFlow::default() });
            let shape = BrushStrokesShape::new(set.clone(), BrushStrokesOptions { head_softness: 0.1, flow, ..Default::default() }).unwrap().with_tips(self.tips.clone());
            strokes.add_shape(shape);
        }
        let over = self.painting.get_layer_mut("Over").unwrap();
        over.clear();
        if s.on("detail") {
            let (w, h) = (built.detail.width(), built.detail.height());
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let v = built.detail.get(x, y).unwrap_or(0.0).clamp(0.0, 1.0);
                    rgba.extend_from_slice(&[255, 255, 255, (v * 230.0) as u8]);
                }
            }
            let wash = ImageSource::from_rgba_vec(rgba, w, h).unwrap();
            over.add_shape(fill_rect(PANELS[0].x, PANELS[0].y, PANELS[0].width, PANELS[0].height, Some(Color::from_rgba(0, 0, 0, 150)), 0.0));
            over.add_shape(ImageShape::new(wash, PANELS[0], ImageShapeOptions::default()).unwrap());
        }
        if s.on("ticks") {
            for (i, p) in PANELS.iter().enumerate() {
                let mut y = p.y + 10.0;
                while y < p.bottom() {
                    let mut x = p.x + 10.0;
                    while x < p.right() {
                        let a = built.fields[i].angle_at(x, y).to_radians();
                        let (dx, dy) = (4.0 * a.cos(), 4.0 * a.sin());
                        over.add_shape(line(x - dx, y - dy, x + dx, y + dy, 1.5, Some(Color::from_rgba(230, 230, 230, 220))));
                        x += 20.0;
                    }
                    y += 20.0;
                }
            }
        }
        self.built = Some(built);
        self.dirty = false;
    }
}

impl DemoScene for DirectionFieldsScene {
    fn name(&self) -> &'static str {
        "direction-fields"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        14.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        if self.dirty {
            self.refill();
        }
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        self.painting.set_time(seconds);
        self.painting.render_with(scene, width, height, &codebrix_webgpu_animation::SessionRenderOptions { include_background: false, ..Default::default() });
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

    fn readouts(&self) -> Vec<Readout> {
        let Some(b) = self.built.as_ref() else { return Vec::new() };
        vec![
            Readout::new("broad", "Panel 1: broad strokes (18 x 5, everywhere)", b.counts[0]),
            Readout::new("small", "Panel 1: small strokes (7 x 2.5, where detail is high)", b.counts[1]),
            Readout::new("noise", "Panel 2: strokes", b.counts[2]),
            Readout::new("guides", "Panel 3: strokes", b.counts[3]),
        ]
    }
}
