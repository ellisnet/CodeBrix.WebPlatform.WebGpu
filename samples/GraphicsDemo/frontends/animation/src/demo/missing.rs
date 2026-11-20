//! "missing-characters" (no C# scene: the owner's rule for characters a font lacks): the same
//! string, holding characters neither font has, drawn in the three MissingCharacterMode choices
//! side by side - with Roboto, whose .notdef glyph is a box, and with Open Sans, whose .notdef
//! has no ink - and the report of what was missing under each line. The strings change every
//! 2.5 s and a marker runs under each line. No backdrop, so the sample points can tell ink from
//! nothing in the layer's own pixels.

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, MissingCharacterMode, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, DrawingSessionOptions};

use crate::demo::{add_header, label, DemoScene, Expect, SamplePoint, ACCENT, DIM, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Missing characters";
const DESCRIPTION: &str = "MissingCharacterMode: ShowTofu (the default) always shows a box, FontDefault shows the font's own .notdef, Blank shows nothing";
const COLUMNS: [f32; 3] = [48.0, 456.0, 864.0];
const BASELINES: [f32; 2] = [250.0, 480.0];
const SIZE: f32 = 44.0;
const MODES: [(MissingCharacterMode, &str); 3] = [(MissingCharacterMode::ShowTofu, "ShowTofu (the default)"), (MissingCharacterMode::FontDefault, "FontDefault"), (MissingCharacterMode::Blank, "Blank")];
const FAMILIES: [&str; 2] = ["Roboto", "Open Sans"];
const STRINGS: [&str; 3] = ["Tea 日 time", "Cake 🎂 day", "Snow ☃ man"];

pub struct MissingCharactersScene {
    session: DrawingSession,
    fonts: FontRegistry,
    /// The lines drawn last, row by row (for the sample points).
    lines: Vec<TextShape>,
}

impl MissingCharactersScene {
    pub fn new(fonts: &FontRegistry) -> MissingCharactersScene {
        let mut session = DrawingSession::new(DrawingSessionOptions { calibration_size: (FRAME_WIDTH, FRAME_HEIGHT), layer_opacity: 255, fonts: Some(fonts.clone()), ..DrawingSessionOptions::default() });
        add_header(&mut session, TITLE, DESCRIPTION, false);
        let labels = session.add_layer("Labels", DIM);
        for (row, family) in FAMILIES.iter().enumerate() {
            let what = if row == 0 { "Roboto - its .notdef glyph is a box" } else { "Open Sans - its .notdef glyph has no ink" };
            label(labels, fonts, what, COLUMNS[0], BASELINES[row] - 92.0, 17.0, INK, TextAlign::Left);
            for (col, (_, name)) in MODES.iter().enumerate() {
                label(labels, fonts, &format!("{name}, {family}"), COLUMNS[col], BASELINES[row] - 62.0, 13.0, DIM, TextAlign::Left);
            }
        }
        label(labels, fonts, "No fallback font and no system font, ever: the advance stays (the .notdef's own, else the space's, else half an em), and the shape reports what was missing.", COLUMNS[0], 660.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Lines", INK);
        MissingCharactersScene { session, fonts: fonts.clone(), lines: Vec::new() }
    }
}

impl DemoScene for MissingCharactersScene {
    fn name(&self) -> &'static str {
        "missing-characters"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let fonts = self.fonts.clone();
        let text = STRINGS[((seconds / 2.5) as usize) % STRINGS.len()];
        let layer = self.session.get_layer_mut("Lines").unwrap();
        layer.clear();
        self.lines.clear();
        for (row, family) in FAMILIES.iter().enumerate() {
            for (col, (mode, _)) in MODES.iter().enumerate() {
                let x = COLUMNS[col];
                let y = BASELINES[row];
                let shape = TextShape::new(text, x, y, SIZE, &fonts, TextShapeOptions { color: Some(INK), family_name: Some(family.to_string()), missing_characters: *mode, ..TextShapeOptions::default() }).unwrap();
                // A marker runs under the line.
                let run = ((seconds % 2.5) / 2.5) as f32;
                let mx = x + run * shape.width();
                layer.add_shape(LineShape::new(x, y + 14.0, x + shape.width(), y + 14.0, LineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 120)) }).unwrap());
                layer.add_shape(LineShape::new(mx - 10.0, y + 14.0, mx + 10.0, y + 14.0, LineShapeOptions { stroke_thickness: 3.0, color: Some(ACCENT) }).unwrap());
                let report: Vec<String> = shape.missing_characters().iter().map(|m| format!("'{}' U+{:04X} at byte {}", m.character, m.character as u32, m.text_index)).collect();
                let report = if report.is_empty() { "nothing missing".to_string() } else { format!("missing: {}", report.join(", ")) };
                label(layer, &fonts, &report, x, y + 40.0, 13.0, ACCENT, TextAlign::Left);
                layer.add_shape(shape.clone());
                self.lines.push(shape);
            }
        }
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }

    fn checks(&self) -> Vec<SamplePoint> {
        // Over each line's first missing cluster, from the baseline up to the cap height: ink, or none.
        let expected = [[true, true, false], [true, false, false]];
        let mut points = Vec::new();
        for (i, shape) in self.lines.iter().enumerate() {
            let (row, col) = (i / 3, i % 3);
            let Some(cluster) = shape.clusters().iter().find(|c| c.is_missing) else {
                continue;
            };
            let name: &'static str = match (row, col) {
                (0, 0) => "Roboto ShowTofu: a box",
                (0, 1) => "Roboto FontDefault: its .notdef box",
                (0, 2) => "Roboto Blank: nothing",
                (1, 0) => "Open Sans ShowTofu: the crate's box",
                (1, 1) => "Open Sans FontDefault: its ink-less .notdef, nothing",
                _ => "Open Sans Blank: nothing",
            };
            let w = cluster.advance;
            let h = SIZE * 0.7;
            let (x, y) = (shape.left() + cluster.x, shape.y() - h);
            points.push(SamplePoint { name, x, y, expect: if expected[row][col] { Expect::InkInBox(w, h) } else { Expect::NoInkInBox(w, h) } });
        }
        points
    }
}
