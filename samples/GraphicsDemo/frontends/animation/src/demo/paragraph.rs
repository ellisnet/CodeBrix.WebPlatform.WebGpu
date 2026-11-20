//! "paragraph" (AnimationDemoApp's ParagraphScene): one text laid out four times by the layout
//! engine - wrapped to a column and aligned left, centre, right and justified - with each line's
//! baseline and the column box drawn from the layout's line metrics; below, a box whose width
//! breathes: the text re-wraps every frame, is limited to two lines, ends in an ellipsis after the
//! last whole word that fits, and is centred vertically in the box.

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{LineShape, LineShapeOptions, RectangleShape, RectangleShapeOptions, TextBlockShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign, TextLayoutOptions, TextTrimming, TextVerticalAlign};
use codebrix_webgpu_animation::{Color, DrawingSession};

use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Paragraphs: wrapping, alignment, trimming";
const DESCRIPTION: &str = "TextLayoutEngine: MaxWidth wrapping, line breaks, Left/Center/Right/Justify, line metrics, ellipsis, vertical alignment";
const TEXT: &str = "Text blocks wrap at spaces and hyphens, keep explicit line breaks, and align each line within the column. Every line reports its top, height, baseline and width,\nso guides like these come straight from the layout.";
const TRIMMED: &str = "When the box is too small the last kept line ends in an ellipsis, cut after the last whole word that fits, and the lines stay centred vertically in the box however wide it is.";

pub struct ParagraphScene {
    session: DrawingSession,
    fonts: FontRegistry,
}

impl ParagraphScene {
    pub fn new(fonts: &FontRegistry) -> ParagraphScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let aligns = [(TextAlign::Left, "Left"), (TextAlign::Center, "Center"), (TextAlign::Right, "Right"), (TextAlign::Justify, "Justify")];
        let column_width = 270.0;
        let mut blocks = Vec::new();
        {
            let guides = session.add_layer("Guides", DIM);
            for (i, (align, name)) in aligns.iter().enumerate() {
                let (x, y) = (40.0 + i as f32 * 306.0, 140.0);
                let layout = codebrix_webgpu_animation::text::TextLayoutEngine::layout(fonts, TEXT, 20.0, None, Some(&TextLayoutOptions { max_width: Some(column_width), align: *align, line_spacing: 1.2, ..TextLayoutOptions::default() })).unwrap();
                label(guides, fonts, &format!("TextAlign.{name}"), x, 128.0, 13.0, ACCENT, TextAlign::Left);
                guides.add_shape(RectangleShape::new(x, y, layout.width(), layout.height(), RectangleShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(132, 138, 164, 110)), ..RectangleShapeOptions::default() }).unwrap());
                for line in layout.lines() {
                    guides.add_shape(LineShape::new(x + line.left(), y + line.baseline(), x + line.left() + line.width(), y + line.baseline(), LineShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(255, 176, 72, 90)) }).unwrap());
                }
                blocks.push(TextBlockShape::new(layout, x, y, Some(INK)).unwrap());
            }
            label(guides, fonts, "MaxWidth animates; MaxLines = 2, TextTrimming.WordEllipsis, MaxHeight 120 with TextVerticalAlign.Center", 40.0, 470.0, 13.0, DIM, TextAlign::Left);
        }
        let text = session.add_layer("Text", INK);
        for b in blocks {
            text.add_shape(b);
        }
        session.add_layer("Box", INK);
        ParagraphScene { session, fonts: fonts.clone() }
    }
}

impl DemoScene for ParagraphScene {
    fn name(&self) -> &'static str {
        "paragraph"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let box_width = (360.0 + ping_pong(seconds, 8.0) * 820.0) as f32;
        let box_height = 120.0;
        let options = TextLayoutOptions { max_width: Some(box_width), max_height: Some(box_height), max_lines: 2, trimming: TextTrimming::WordEllipsis, vertical_align: TextVerticalAlign::Center, align: TextAlign::Center, ..TextLayoutOptions::default() };
        let layout = self.session.layout_text(TRIMMED, 26.0, None, Some(&options)).unwrap();
        let fonts = self.fonts.clone();
        let moving = self.session.get_layer_mut("Box").unwrap();
        moving.clear();
        moving.add_shape(RectangleShape::new(40.0, 500.0, box_width, box_height, RectangleShapeOptions { stroke_thickness: 1.0, color: Some(Color::from_rgba(255, 255, 255, 14)), is_filled: true, corner_radius: 10.0 }).unwrap());
        moving.add_shape(RectangleShape::new(40.0, 500.0, box_width, box_height, RectangleShapeOptions { stroke_thickness: 1.5, color: Some(ACCENT), is_filled: false, corner_radius: 10.0 }).unwrap());
        let (lines, truncated) = (layout.line_count(), layout.is_truncated());
        moving.add_shape(TextBlockShape::new(layout, 40.0, 500.0, Some(INK)).unwrap());
        label(moving, &fonts, &format!("MaxWidth {box_width:.0}   lines {lines}   truncated {}", if truncated { "yes" } else { "no" }), 40.0, 650.0, 14.0, ACCENT, TextAlign::Left);
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
