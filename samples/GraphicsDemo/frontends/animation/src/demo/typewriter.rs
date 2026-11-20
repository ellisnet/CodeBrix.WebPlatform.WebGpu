//! "typewriter" (AnimationDemoApp's TypewriterScene): revealing text over time. Top left, a
//! typewriter - RevealCount bound to a track, one character at a time, with a blinking caret placed
//! from the layout's cluster boxes; top right, words fading in with a reveal softness; bottom, the
//! block split into word pieces and line pieces (create_piece_group) animated one after another by
//! bind_stagger - words rising and growing, lines sliding in.

use std::rc::Rc;

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::models::{AnimatedProperty, StaggerOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingShape, TextBlockShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign, TextLayoutOptions, TextLayoutResult, TextUnit};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{fill_rect, floats, L};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Typewriter and staggered text";
const DESCRIPTION: &str = "TextBlockShape RevealCount / RevealFraction / RevealSoftness, pieces per word and line, DrawingAnimations.BindStagger";
const TYPED: &str = "A typewriter reveal counts characters: spaces take their turn, and the caret follows the layout.";
const FADED: &str = "A softer reveal counts words and fades each one in over the two before it.";
const STAGGERED: &str = "Staggered words rise into place one after another";
const LINES: &str = "Lines can arrive\none at a time,\nsliding in from the left.";

pub struct TypewriterScene {
    session: DrawingSession,
}

/// A thin bar after the last character shown, blinking twice a second (C#'s Caret).
fn caret(layout: &TextLayoutResult, count: f32, seconds: f64) -> Option<DrawingShape> {
    if (seconds * 2.0) as i64 % 2 == 1 {
        return None;
    }
    let clusters = layout.clusters();
    let shown = (count.floor().max(0.0) as usize).min(clusters.len());
    let b = if shown < clusters.len() {
        clusters[shown].bounds()
    } else {
        let last = &layout.lines()[layout.line_count() - 1];
        RectangleF::new(last.left() + last.width(), last.top(), 0.0, last.height())
    };
    Some(fill_rect(40.0 + b.x, 150.0 + b.y + b.height * 0.12, 3.0, b.height * 0.8, Some(ACCENT), 0.0).into())
}

impl TypewriterScene {
    pub fn new(fonts: &FontRegistry) -> TypewriterScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "RevealCount bound to a track (characters); caret from Clusters[n].Bounds", 40.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "RevealUnit = Word, RevealSoftness = 2", 680.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "CreatePieceGroup(TextUnit.Word) + BindStagger (offset, scale, BackOut)", 40.0, 410.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "CreatePieceGroup(TextUnit.Line) + BindStagger (slide)", 680.0, 410.0, 13.0, DIM, TextAlign::Left);
        session.add_layer("Text", INK);
        let column = |spacing: f32| TextLayoutOptions { max_width: Some(560.0), line_spacing: spacing, ..TextLayoutOptions::default() };
        // The typewriter: one more character every 1/24 s, and a caret after the last one shown.
        let typed_layout = session.layout_text(TYPED, 30.0, None, Some(&column(1.2))).unwrap();
        let typed = session.draw_text_block(typed_layout.clone(), 40.0, 150.0, Some(INK)).unwrap();
        let n = typed_layout.cluster_count();
        let typing = floats(&[(0.3, 0.0, L), (0.3 + n as f64 / 24.0, n as f32, L)]);
        session.animations_mut().bind(typed, AnimatedProperty::RevealCount, typing.clone()).unwrap();
        let caret_layout: Rc<TextLayoutResult> = typed_layout;
        session.draw_shape(AnimatedShape::new(move |t| caret(&caret_layout, typing.evaluate(t).unwrap_or(0.0), t), 0.0));
        // Words fading in.
        let faded_layout = session.layout_text(FADED, 30.0, None, Some(&column(1.2))).unwrap();
        let mut faded = TextBlockShape::new(faded_layout, 680.0, 150.0, Some(Color::from_rgb(140, 200, 255))).unwrap();
        faded.set_reveal_unit(TextUnit::Word);
        faded.set_reveal_softness(2.0).unwrap();
        let faded = session.draw_shape(faded);
        session.animations_mut().bind(faded, AnimatedProperty::RevealFraction, floats(&[(0.5, 0.0, L), (4.5, 1.0, EasingKind::SineInOut)])).unwrap();
        // Words as pieces, each rising and growing into place.
        let staggered_layout = session.layout_text(STAGGERED, 40.0, None, Some(&column(1.15))).unwrap();
        let words = TextBlockShape::new(staggered_layout, 40.0, 440.0, Some(ACCENT)).unwrap().create_piece_group(TextUnit::Word);
        let word_ids: Vec<_> = words.get_children().iter().map(|c| c.id()).collect();
        session.draw_shape(words);
        let rise = StaggerOptions { start_time: 1.0, interval: 0.18, duration: 0.7, easing: EasingKind::BackOut, from_offset: PointF::new(0.0, 40.0), from_scale: 0.4, ..StaggerOptions::default() };
        session.animations_mut().bind_stagger(&word_ids, Some(&rise)).unwrap();
        // Lines as pieces, sliding in.
        let line_layout = session.layout_text(LINES, 36.0, None, Some(&TextLayoutOptions { line_spacing: 1.25, ..TextLayoutOptions::default() })).unwrap();
        let lines = TextBlockShape::new(line_layout, 680.0, 440.0, Some(INK)).unwrap().create_piece_group(TextUnit::Line);
        let line_ids: Vec<_> = lines.get_children().iter().map(|c| c.id()).collect();
        session.draw_shape(lines);
        let slide = StaggerOptions { start_time: 1.5, interval: 0.6, duration: 0.9, easing: EasingKind::CubicOut, from_offset: PointF::new(-120.0, 0.0), ..StaggerOptions::default() };
        session.animations_mut().bind_stagger(&line_ids, Some(&slide)).unwrap();
        TypewriterScene { session }
    }
}

impl DemoScene for TypewriterScene {
    fn name(&self) -> &'static str {
        "typewriter"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
