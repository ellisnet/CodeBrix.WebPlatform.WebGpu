//! "ownership-colours" (a feature of the second GO, Not in C#): an ownership map made ONCE (forty
//! brushstrokes and three circles over a 560 x 420 drawing, the last painted wins), re-coloured
//! EVERY FRAME - a palette cycling with the clock - with `OwnershipShape::with_colors` on the shape
//! made once, so only the colour table travels to the GPU. A switch draws it the old way
//! (`OwnershipShape::new` every frame: the whole picture of owner numbers uploaded again). The
//! page's RenderStats line shows the upload counters (`image_uploads`, `image_upload_bytes`,
//! `color_table_uploads`, `color_table_upload_bytes`).

use codebrix_webgpu_animation::painting::DrawingOwnershipMap;
use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, OwnershipShape, OwnershipShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::kit::disc;
use crate::demo::{create_frame_session, create_session, label, Control, DemoScene, Readout, Settings, DIM, INK};

const TITLE: &str = "Ownership map re-coloured every frame";
const DESCRIPTION: &str = "One map, made once; OwnershipShape::with_colors each frame uploads only the colour table - the old way (a new shape each frame) uploads the whole picture";
const MAP_WIDTH: u32 = 560;
const MAP_HEIGHT: u32 = 420;
const DESTINATION: RectangleF = RectangleF { x: 360.0, y: 140.0, width: 560.0, height: 420.0 };

const CONTROLS: &[Control] = &[
    Control::Toggle { key: "old-way", label: "The old way: OwnershipShape::new every frame", default: false },
    Control::Slider { key: "speed", label: "Palette speed", min: 0.0, max: 2.0, step: 0.1, default: 0.5 },
];

const LEGEND: &[&str] = &[
    "The picture looks the same either way.",
    "with_colors (default): after the first frame, image_uploads = 0 and color_table_uploads = 1 of (map width x 4) = 2,240 bytes per frame.",
    "The old way: image_uploads = 1 of 560 x 421 x 4 = 943,040 bytes every frame.",
    "Compare cpu_ms (smoothed) on the stats line between the two.",
];

pub struct OwnershipColoursScene {
    frame: DrawingSession,
    fonts: FontRegistry,
    map: Option<(DrawingOwnershipMap, OwnershipShape)>,
    settings: Settings,
}

fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let byte = |u: f32| ((u + m) * 255.0).round() as u8;
    Color::from_rgb(byte(r), byte(g), byte(b))
}

/// The colours at a time: each element's hue walks round the circle, out of step with the others.
pub fn colours_at(count: usize, seconds: f64, speed: f32) -> Vec<Color> {
    (0..count).map(|i| hsv(i as f32 * 0.137 + seconds as f32 * speed * 0.25, 0.65, 0.92)).collect()
}

fn make_map(fonts: &FontRegistry) -> DrawingOwnershipMap {
    let mut drawing = create_session(MAP_WIDTH, MAP_HEIGHT, Color::TRANSPARENT, fonts);
    let paint = drawing.add_layer("Paint", Color::WHITE);
    let mut random = DrawingRandom::new(2024);
    for _ in 0..40 {
        let (x, y) = (random.uniform(0.0, MAP_WIDTH as f64) as f32, random.uniform(0.0, MAP_HEIGHT as f64) as f32);
        let length = random.uniform(60.0, 140.0) as f32;
        let angle = random.uniform(0.0, 180.0) as f32;
        let bend = random.uniform(-0.15, 0.15) as f32;
        paint.add_shape(BrushStrokeShape::new(x, y, length, length * 0.32, angle, BrushStrokeShapeOptions { bend, ..BrushStrokeShapeOptions::default() }).unwrap());
    }
    for c in 0..3 {
        paint.add_shape(disc(140.0 + c as f32 * 140.0, 210.0 + (c as f32 - 1.0) * 60.0, 40.0, None));
    }
    drawing.export_layer_ownership_map("Paint", (MAP_WIDTH, MAP_HEIGHT)).unwrap()
}

impl OwnershipColoursScene {
    pub fn new(fonts: &FontRegistry) -> OwnershipColoursScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let labels = frame.add_layer("Labels", DIM);
        label(labels, fonts, "export_layer_ownership_map once (560 x 420)", 40.0, 170.0, 15.0, INK, TextAlign::Left);
        label(labels, fonts, "OwnershipShape::new once,", 40.0, 200.0, 15.0, INK, TextAlign::Left);
        label(labels, fonts, "then base.with_colors(&colours_at(t))", 40.0, 222.0, 15.0, INK, TextAlign::Left);
        label(labels, fonts, "every frame", 40.0, 244.0, 15.0, INK, TextAlign::Left);
        label(labels, fonts, "Upload counters: the RenderStats", 40.0, 300.0, 13.0, DIM, TextAlign::Left);
        label(labels, fonts, "line under the canvas", 40.0, 318.0, 13.0, DIM, TextAlign::Left);
        frame.add_layer("Map", INK);
        OwnershipColoursScene { frame, fonts: fonts.clone(), map: None, settings: Settings::from_controls(CONTROLS) }
    }

    fn table_bytes(map: &DrawingOwnershipMap) -> usize {
        let w = map.width() as usize;
        w * map.element_count().div_ceil(w).max(1) * 4
    }
}

impl DemoScene for OwnershipColoursScene {
    fn name(&self) -> &'static str {
        "ownership-colours"
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
        if self.map.is_none() {
            let map = make_map(&self.fonts);
            let base = OwnershipShape::new(&map, &colours_at(map.element_count(), 0.0, 0.0), DESTINATION, OwnershipShapeOptions { unowned: Color::from_rgb(26, 28, 40), ..OwnershipShapeOptions::default() }).unwrap();
            self.map = Some((map, base));
        }
        let (map, base) = self.map.as_ref().unwrap();
        let colours = colours_at(map.element_count(), seconds, self.settings.get("speed"));
        let shape = if self.settings.on("old-way") {
            OwnershipShape::new(map, &colours, DESTINATION, OwnershipShapeOptions { unowned: Color::from_rgb(26, 28, 40), ..OwnershipShapeOptions::default() }).unwrap()
        } else {
            base.with_colors(&colours).unwrap()
        };
        let layer = self.frame.get_layer_mut("Map").unwrap();
        layer.clear();
        layer.add_shape(shape);
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
    }

    fn controls(&self) -> &'static [Control] {
        CONTROLS
    }

    fn apply_settings(&mut self, settings: &Settings) {
        self.settings = settings.clone();
    }

    fn legend(&self) -> &'static [&'static str] {
        LEGEND
    }

    fn readouts(&self) -> Vec<Readout> {
        let Some((map, _)) = self.map.as_ref() else { return Vec::new() };
        vec![
            Readout::new("elements", "Elements in the map", map.element_count()),
            Readout::new("picture-bytes", "Whole picture (owner numbers + colour rows)", format!("{} x {} x 4 = {} bytes", map.width(), map.height() + 1, map.width() as usize * (map.height() as usize + 1) * 4)),
            Readout::new("table-bytes", "Colour table alone", format!("{} bytes", Self::table_bytes(map))),
            Readout::new("way", "Drawn with", if self.settings.on("old-way") { "OwnershipShape::new each frame" } else { "base.with_colors each frame" }),
        ]
    }
}
