//! "lottie-images": a Lottie IMAGE LAYER fed by the application. The sample's own hand-written
//! file (a resource of the backend) has one image asset, "badge" (128 x 128), on a layer that
//! moves, turns and grows. The crate never fetches or decodes the asset itself: the page hands
//! the element its picture with `DrawingLottieShape::set_image`. Left, the BACKEND'S PIXELS - a
//! badge the backend makes in code and writes with the Animation package's PicturePixelsWriter
//! (the generated `lottie_badge_bytes` call, read with `ImageSource::from_picture_pixels`);
//! middle, the same file fed a PNG FILE decoded in the browser (`ImageSource::from_png`: the
//! Cube Pets colour map, stretched into the asset's box); right, no picture at all - the image
//! layer draws nothing, and the document's report says why (ImageNeedsPixels).

use std::cell::RefCell;

use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::lottie::LottiePlayOptions;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::lottie_files::{backend_lottie, place, place_with_images};
use crate::demo::pictures::backend_png_picture;
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Lottie: an image layer fed by the application";
const DESCRIPTION: &str = "DrawingLottieShape::set_image(\"badge\", picture): pixels from the backend (PicturePixelsWriter), a PNG file, or none";
const FILE: &str = "E_image_layer.json";
const ASSET: &str = "badge";

thread_local! {
    static BACKEND_BADGE: RefCell<Option<ImageSource>> = const { RefCell::new(None) };
}

/// Keeps the badge the backend made in code (its picture-pixels form, already read).
pub fn set_backend_badge(picture: ImageSource) {
    BACKEND_BADGE.with(|p| *p.borrow_mut() = Some(picture));
}

/// The badge the backend made (a 1 x 1 grey pixel when none was set).
fn backend_badge() -> ImageSource {
    BACKEND_BADGE.with(|p| p.borrow().clone()).unwrap_or_else(|| ImageSource::from_rgba(&[128, 128, 128, 255], 1, 1).unwrap())
}

pub struct LottieImagesScene {
    session: DrawingSession,
}

impl LottieImagesScene {
    pub fn new(fonts: &FontRegistry) -> LottieImagesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Frames", DIM);
        session.add_layer("Lottie", INK);
        session.add_layer("Notes", DIM);
        let looping = LottiePlayOptions::default();
        place_with_images(&mut session, fonts, FILE, RectangleF::new(32.0, 150.0, 392.0, 294.0), looping.clone(), "pixels from the backend (PicturePixelsWriter)", &[(ASSET, backend_badge())]);
        place_with_images(&mut session, fonts, FILE, RectangleF::new(444.0, 150.0, 392.0, 294.0), looping.clone(), "a PNG file decoded in the browser", &[(ASSET, backend_png_picture())]);
        place(&mut session, fonts, FILE, RectangleF::new(856.0, 150.0, 392.0, 294.0), looping, "no picture handed over");
        let report = backend_lottie(FILE).map(|l| l.report()).unwrap_or_default();
        let line = if report.is_empty() { "report(): nothing".to_string() } else { format!("report(): {}", report.join("; ")) };
        let notes = session.get_layer_mut("Notes").unwrap();
        label(notes, fonts, "The crate never fetches or decodes an image asset: the application hands the element a picture for each asset id.", 32.0, 640.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, &line, 32.0, 664.0, 13.0, DIM, TextAlign::Left);
        LottieImagesScene { session }
    }
}

impl DemoScene for LottieImagesScene {
    fn name(&self) -> &'static str {
        "lottie-images"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        8.0
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        self.session.render_with_background(scene, width, height);
    }
}
