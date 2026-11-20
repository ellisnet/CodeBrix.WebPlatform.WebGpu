//! "lottie-features": the sample's own Lottie file (written by hand, a resource of the backend) -
//! PARENTING (a null layer turns an arm that carries a star), a TRIM PATH (a ring drawn on), a
//! GRADIENT seen through a moving ALPHA MATTE, and three MARKERS ("build", "wave", "settle") -
//! played whole and looping on the left; on the right the "wave" MARKER'S SPAN alone, ping-pong.
//! THE MARKERS ARE NAMED POINTS IN TIME: the scene hands them to the page (`time_points`), which
//! defines them on the host's clock ("lottie-features/wave" ...) beside the scenes' starts; the
//! row under the animation lights the marker the playhead is in.

use codebrix_webgpu_animation::lottie::{DrawingLottieShape, LottiePlayOptions, LottiePlayback, LottieRange};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::kit::fill_rect;
use crate::demo::lottie_files::{backend_lottie, place};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Lottie: parenting, trim path, gradient, matte, markers";
const DESCRIPTION: &str = "The sample's own file; LottieRange::Marker plays one marker's span; marker_times() become the clock's named points";
const FILE: &str = "D_features.json";

pub struct LottieFeaturesScene {
    session: DrawingSession,
    whole: Option<DrawingLottieShape>,
}

impl LottieFeaturesScene {
    pub fn new(fonts: &FontRegistry) -> LottieFeaturesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Frames", DIM);
        session.add_layer("Lottie", INK);
        session.add_layer("Notes", DIM);
        let whole = place(&mut session, fonts, FILE, RectangleF::new(40.0, 120.0, 560.0, 420.0), LottiePlayOptions::default(), "the whole file, looping");
        let wave = LottiePlayOptions { range: LottieRange::Marker("wave".to_string()), playback: LottiePlayback::PingPong, ..LottiePlayOptions::default() };
        place(&mut session, fonts, FILE, RectangleF::new(720.0, 120.0, 480.0, 360.0), wave, "LottieRange::Marker(\"wave\"), ping-pong");
        // The marker row: each marker's name, lit while the playhead of the whole file is in it.
        if let (Some(shape), Some(lottie)) = (whole.clone(), backend_lottie(FILE)) {
            let markers = lottie.markers().to_vec();
            let fonts_row = fonts.clone();
            session.get_layer_mut("Notes").unwrap().add_shape(AnimatedShape::new(
                move |t| {
                    let frame = shape.frame_at(t);
                    let mut group = DrawingGroup::new(None);
                    for (i, m) in markers.iter().enumerate() {
                        let x = 70.0 + i as f32 * 180.0;
                        let inside = frame >= m.frame && frame < m.end_frame();
                        let (back, ink) = if inside { (ACCENT, codebrix_webgpu_animation::Color::from_rgb(18, 20, 30)) } else { (codebrix_webgpu_animation::Color::from_rgb(40, 44, 60), INK) };
                        group.add(fill_rect(x, 618.0, 160.0, 30.0, Some(back), 15.0));
                        group.add(TextShape::new(&format!("{} ({}-{})", m.name, m.frame, m.end_frame()), x + 80.0, 638.0, 14.0, &fonts_row, TextShapeOptions { color: Some(ink), align: TextAlign::Center, ..TextShapeOptions::default() }).ok()?);
                    }
                    Some(group.into())
                },
                0.0,
            ));
            let report = lottie.report();
            let line = if report.is_empty() { "report(): nothing in this file is left out or approximated".to_string() } else { format!("report(): {}", report.join("; ")) };
            let notes = session.get_layer_mut("Notes").unwrap();
            label(notes, fonts, "the markers above are named points on the clock: lottie-features/<name>", 70.0, 672.0, 13.0, DIM, TextAlign::Left);
            label(notes, fonts, &line, 720.0, 560.0, 13.0, DIM, TextAlign::Left);
            label(notes, fonts, "null layer -> arm -> star (parenting); ring: trim path; disc: alpha matte over a gradient", 720.0, 580.0, 13.0, DIM, TextAlign::Left);
        }
        LottieFeaturesScene { session, whole }
    }
}

impl DemoScene for LottieFeaturesScene {
    fn name(&self) -> &'static str {
        "lottie-features"
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

    fn time_points(&self) -> Vec<(String, f64)> {
        self.whole.as_ref().map(|s| s.marker_times()).unwrap_or_default()
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
