//! "lottie-files": the renderer trial's three INVENTED Lottie files - "many shapes" (300 shape
//! layers, trim paths), "painterly" (gradients, a mask, a matte, morphing paths) and "busy scene"
//! (nested precompositions and repeaters) - played through codebrix-webgpu-animation's `lottie`
//! module (velato reads them, the crate's renderer draws them) as elements of a session on the
//! session's time: one LOOPS, one PING-PONGS, one plays at ONE AND A HALF TIMES the speed. THE
//! LOTTIE FILES ARE RESOURCES OF THE BACKEND (the sample's rule): fetched once with the generated
//! `..._json_bytes` calls, read into DrawingLottie documents and kept by `set_backend_lottie`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::lottie::{DrawingLottie, DrawingLottieAspect, DrawingLottieOptions, DrawingLottieShape, LottiePlayOptions, LottiePlayback};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, TextShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{DrawingSession, RectangleF};

use crate::demo::kit::{fill_rect, outline_rect};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Lottie: the three trial files";
const DESCRIPTION: &str = "DrawingLottie::from_json + DrawingSession::draw_lottie: loop, ping-pong and 1.5x speed, on the session's time, drawn by the crate's renderer";

thread_local! {
    static BACKEND_LOTTIE: RefCell<HashMap<&'static str, Arc<DrawingLottie>>> = RefCell::new(HashMap::new());
}

/// Reads a Lottie file the backend served and keeps it, by name, for the Lottie scenes. The
/// error says what is wrong with the file.
pub fn set_backend_lottie(name: &'static str, json: &[u8]) -> Result<(), String> {
    let lottie = DrawingLottie::load(json, &DrawingLottieOptions::default()).map_err(|e| format!("The Lottie file {name} from the backend could not be read: {e}"))?;
    BACKEND_LOTTIE.with(|l| l.borrow_mut().insert(name, Arc::new(lottie)));
    Ok(())
}

/// A Lottie file the backend served (the page fetched every one before the scenes are made).
pub fn backend_lottie(name: &str) -> Option<Arc<DrawingLottie>> {
    BACKEND_LOTTIE.with(|l| l.borrow().get(name).cloned())
}

/// Places a document, draws its box, its caption and a live frame read-out under it.
pub(crate) fn place(session: &mut DrawingSession, fonts: &FontRegistry, name: &str, destination: RectangleF, options: LottiePlayOptions, caption: &str) -> Option<DrawingLottieShape> {
    place_with_images(session, fonts, name, destination, options, caption, &[])
}

/// [`place`], with the pictures of the document's image assets handed over (asset id, picture).
#[allow(clippy::too_many_arguments)]
pub(crate) fn place_with_images(session: &mut DrawingSession, fonts: &FontRegistry, name: &str, destination: RectangleF, options: LottiePlayOptions, caption: &str, images: &[(&str, ImageSource)]) -> Option<DrawingLottieShape> {
    let lottie = backend_lottie(name)?;
    let d = destination;
    session.get_layer_mut("Frames").unwrap().add_shape(fill_rect(d.x, d.y, d.width, d.height, Some(codebrix_webgpu_animation::Color::from_rgb(28, 31, 44)), 10.0));
    session.get_layer_mut("Frames").unwrap().add_shape(outline_rect(d.x, d.y, d.width, d.height, 1.0, Some(codebrix_webgpu_animation::Color::from_rgba(132, 138, 164, 90)), 10.0));
    session.set_active_layer("Lottie");
    let id = session.draw_lottie(lottie.clone(), destination, DrawingLottieAspect::Meet, options).ok()?;
    for (asset_id, picture) in images {
        session.find_element_mut(id)?.as_lottie_mut()?.set_image(asset_id, picture.clone());
    }
    let shape = session.find_element(id)?.as_lottie()?.clone();
    let notes = session.get_layer_mut("Notes").unwrap();
    label(notes, fonts, caption, d.x + d.width / 2.0, d.bottom() + 26.0, 15.0, INK, TextAlign::Center);
    let summary = format!("{} x {} at {} fps, {:.1} s, {} draws at frame 0", lottie.width(), lottie.height(), lottie.frame_rate(), lottie.duration(), lottie.draw_list(lottie.in_point(), kurbo_identity(), 1.0, true).draw_count());
    label(notes, fonts, &summary, d.x + d.width / 2.0, d.bottom() + 46.0, 13.0, DIM, TextAlign::Center);
    let readout = shape.clone();
    let fonts = fonts.clone();
    let (x, y) = (d.x + d.width / 2.0, d.bottom() + 66.0);
    session.get_layer_mut("Notes").unwrap().add_shape(AnimatedShape::new(
        move |t| Some(TextShape::new(&format!("frame {:.1}", readout.frame_at(t)), x, y, 13.0, &fonts, TextShapeOptions { color: Some(ACCENT), align: TextAlign::Center, ..TextShapeOptions::default() }).ok()?.into()),
        0.0,
    ));
    Some(shape)
}

fn kurbo_identity() -> codebrix_webgpu_animation::kurbo::Affine {
    codebrix_webgpu_animation::kurbo::Affine::IDENTITY
}

pub struct LottieFilesScene {
    session: DrawingSession,
}

impl LottieFilesScene {
    pub fn new(fonts: &FontRegistry) -> LottieFilesScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        session.add_layer("Frames", DIM);
        session.add_layer("Lottie", INK);
        session.add_layer("Notes", DIM);
        let loop_ = LottiePlayOptions::default();
        let ping_pong = LottiePlayOptions { playback: LottiePlayback::PingPong, ..LottiePlayOptions::default() };
        let faster = LottiePlayOptions { speed: 1.5, ..LottiePlayOptions::default() };
        place(&mut session, fonts, "A_many_shapes.json", RectangleF::new(32.0, 130.0, 392.0, 392.0), loop_, "\"many shapes\" - loop");
        place(&mut session, fonts, "B_painterly.json", RectangleF::new(444.0, 130.0, 392.0, 392.0), ping_pong, "\"painterly\" - ping-pong");
        place(&mut session, fonts, "C_busy_scene.json", RectangleF::new(856.0, 130.0, 392.0, 392.0), faster, "\"busy scene\" - 1.5x speed");
        let notes = session.get_layer_mut("Notes").unwrap();
        label(notes, fonts, "Invented for the renderer trial (nothing downloaded); served by the backend as files; velato reads them, the crate's pass guards velato's panics, the crate's renderer draws them.", 32.0, 680.0, 13.0, DIM, TextAlign::Left);
        LottieFilesScene { session }
    }
}

impl DemoScene for LottieFilesScene {
    fn name(&self) -> &'static str {
        "lottie-files"
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
