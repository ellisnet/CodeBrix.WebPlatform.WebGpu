//! "svg-css": an SVG document that ANIMATES ITSELF with CSS @keyframes in its <style> (served by
//! the backend, written for the sample), played by the crate at the session's time: a spinner
//! turning about its own centre (transform-box: fill-box), a loading bar scaling from its left
//! edge and fading, three dots hopping with negative delays and alternate direction, a heart
//! beating in steps(), a group whose fill cycles while the square with its own CSS rule keeps
//! white, a trace drawing itself while its colour warms (fill-mode both, a delay), a caret blinking
//! (visibility, step-start) and a pendulum swinging about a pivot in px - and an @media rule that
//! is not used and is reported.

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::svg::DrawingSvgAspect;
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::svg_files::{load_svg, notes_and_report};
use crate::demo::{create_frame_session, DemoScene, DIM, INK};

const TITLE: &str = "Animated SVG: CSS keyframes";
const DESCRIPTION: &str = "DrawingSvg plays the document's own @keyframes animations (the CSS subset) at the session's time";

pub struct SvgCssScene {
    session: DrawingSession,
}

impl SvgCssScene {
    pub fn new(fonts: &FontRegistry) -> SvgCssScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let svg = load_svg("css-scene").expect("the CSS document loads");
        let notes = session.add_layer("Notes", DIM);
        notes_and_report(notes, fonts, &svg, &["spinner: rotate about its own centre (fill-box)", "bar: scaleX from its left edge, then fades", "dots: translateY, alternate, negative delays", "heart: scale in steps(6, end)", "swatches: the group's fill cycles; the white square", "   has its own CSS rule (.keep) and keeps it", "trace: stroke-dashoffset and stroke, delayed, fill-mode both", "caret: visibility (step-start); pendulum: rotate about 470px 220px"], 830.0, 140.0, 64);
        session.add_layer("Art", INK);
        session.draw_svg(&svg, RectangleF::new(32.0, 120.0, 780.0, 520.0), DrawingSvgAspect::Meet);
        SvgCssScene { session }
    }
}

impl DemoScene for SvgCssScene {
    fn name(&self) -> &'static str {
        "svg-css"
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
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
