//! "svg-smil": an SVG document that ANIMATES ITSELF with time-based SMIL (served by the backend,
//! written for the sample), played by the crate at the session's time: a car driving along a road
//! (animateMotion with an mpath, turned with the road), traffic lights handing over through
//! syncbase begins that loop, a windmill turning (animateTransform) with discrete colour changes
//! (keyTimes), a ball bouncing on key splines, a group's animated fill that the square with a fill
//! of its own does NOT take (as browsers do), a pulsing ring (values), a line drawing itself
//! (stroke-dashoffset, frozen) - and a click begin that is not played and is reported.

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::svg::DrawingSvgAspect;
use codebrix_webgpu_animation::text::FontRegistry;
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::svg_files::{load_svg, notes_and_report};
use crate::demo::{create_frame_session, DemoScene, DIM, INK};

const TITLE: &str = "Animated SVG: SMIL";
const DESCRIPTION: &str = "DrawingSvg plays the document's own animate / set / animateTransform / animateMotion at the session's time";

pub struct SvgSmilScene {
    session: DrawingSession,
}

impl SvgSmilScene {
    pub fn new(fonts: &FontRegistry) -> SvgSmilScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let svg = load_svg("smil-scene").expect("the SMIL document loads");
        let notes = session.add_layer("Notes", DIM);
        notes_and_report(notes, fonts, &svg, &["car: animateMotion along the road (mpath, rotate=auto)", "lights: set with syncbase begins (red -> amber -> green -> red)", "windmill: animateTransform rotate; fill at keyTimes (discrete)", "ball: translate with keySplines (calcMode spline)", "swatches: the group's fill animates; the white square", "   sets its own fill and keeps it", "ring: opacity and scale from values; line: dashoffset, frozen"], 830.0, 140.0, 64);
        session.add_layer("Art", INK);
        session.draw_svg(&svg, RectangleF::new(32.0, 120.0, 780.0, 520.0), DrawingSvgAspect::Meet);
        SvgSmilScene { session }
    }
}

impl DemoScene for SvgSmilScene {
    fn name(&self) -> &'static str {
        "svg-smil"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn description(&self) -> &'static str {
        DESCRIPTION
    }
    fn duration_seconds(&self) -> f64 {
        12.0
    }
    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
