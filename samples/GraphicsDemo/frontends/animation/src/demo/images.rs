//! "images" (AnimationDemoApp's ImageScene): pictures as elements. Pictures painted in code (a
//! landscape, a round emblem with a transparent surround, a 13 x 11 pixel heart - pictures.rs)
//! and ONE PICTURE DECODED ON THE BACKEND (the Kenney "Cube Pets" colour map, sent as pixels -
//! the C# scene's "encoded bytes" source, whose decoding stays on the backend here). Top: the
//! landscape turning and scaling through its transform while its opacity pulses; a slow zoom and
//! pan made by rebuilding an ImageShape with a moving source rectangle; a tint bound to a colour
//! track (day, dusk, night). Bottom: the tiny heart drawn large with nearest and with bilinear
//! sampling, the backend's picture between them (decoded on the backend, and as a PNG file decoded by the crate); emblems over transparency (on a checkerboard)
//! in a group that turns, one appearing at 2 s (a time window). The C# scene exports the emblem
//! panel from a second session; here the turning group sits inside a still group whose mask
//! keeps it to the panel.

use codebrix_webgpu_animation::animation::{ColorTrack, EasingKind, TrackLoopMode};
use codebrix_webgpu_animation::effects::DrawingMask;
use codebrix_webgpu_animation::images::ImageSampling;
use codebrix_webgpu_animation::models::{AnimatedProperty, DrawingTransform};
use codebrix_webgpu_animation::paths::PathGeometry;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, DrawingGroup, DrawingShape, ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, Rectangle, RectangleF};

use crate::demo::kit::{checkerboard, floats, outline_rect, points, L};
use crate::demo::pictures::{backend_picture, backend_png_picture, emblem, landscape, ping_pong, pixel_heart};
use crate::demo::{create_frame_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Pictures as elements";
const DESCRIPTION: &str = "ImageSource (from RGBA - painted in code, or decoded on the backend), ImageShape: transform, opacity, source rectangle, Tint, Nearest/Linear";
const PICTURE_WIDTH: f32 = 320.0;
const PICTURE_HEIGHT: f32 = 200.0;
const PANEL: RectangleF = RectangleF::new(870.0, 420.0, 360.0, 250.0);

pub struct ImageScene {
    session: DrawingSession,
}

impl ImageScene {
    pub fn new(fonts: &FontRegistry) -> ImageScene {
        let landscape = landscape(PICTURE_WIDTH as usize, PICTURE_HEIGHT as usize);
        let emblem = emblem(128);
        let heart = pixel_heart();
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "Transform + Opacity", 220.0, 380.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a moving source rectangle (zoom and pan)", 640.0, 380.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "Tint bound to a ColorTrack", 1060.0, 380.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a 13 x 11 pixel picture: Nearest | Linear", 400.0, 690.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "backend pixels | PNG file", 400.0, 652.0, 13.0, DIM, TextAlign::Center);
        label(notes, fonts, "over transparency, in a turning group", PANEL.x + PANEL.width / 2.0, 690.0, 14.0, DIM, TextAlign::Center);
        checkerboard(notes, PANEL.x, PANEL.y, PANEL.width, PANEL.height);
        session.add_layer("Pictures", INK);
        let turning = session.draw_image(&landscape, RectangleF::new(60.0, 140.0, PICTURE_WIDTH, PICTURE_HEIGHT), ImageSampling::Linear).unwrap();
        let mut a = session.animations_mut();
        a.bind(turning, AnimatedProperty::Anchor, points(&[(0.0, 220.0, 240.0, L)])).unwrap();
        a.bind(turning, AnimatedProperty::Rotation, floats(&[(0.0, -12.0, L), (4.0, 12.0, EasingKind::SineInOut), (8.0, -12.0, EasingKind::SineInOut)])).unwrap();
        a.bind(turning, AnimatedProperty::Scale, floats(&[(0.0, 0.8, L), (4.0, 0.95, EasingKind::SineInOut), (8.0, 0.8, EasingKind::SineInOut)])).unwrap();
        a.bind(turning, AnimatedProperty::Opacity, floats(&[(0.0, 1.0, L), (2.0, 0.35, EasingKind::SineInOut), (4.0, 1.0, EasingKind::SineInOut)]).with_loop_mode(TrackLoopMode::Loop)).unwrap();
        // Zoom and pan: the source rectangle shrinks towards the sun and drifts.
        let zoomed = landscape.clone();
        session.draw_shape(AnimatedShape::new(
            move |t| {
                let u = ping_pong(t, 8.0);
                let w = (PICTURE_WIDTH as f64 - 220.0 * u).round() as i32;
                let h = (w as f64 * PICTURE_HEIGHT as f64 / PICTURE_WIDTH as f64).round() as i32;
                let x = ((PICTURE_WIDTH as f64 - w as f64) * (0.15 + 0.6 * u)).round() as i32;
                let y = ((PICTURE_HEIGHT as f64 - h as f64) * 0.3).round() as i32;
                ImageShape::new(zoomed.clone(), RectangleF::new(480.0, 140.0, PICTURE_WIDTH, PICTURE_HEIGHT), ImageShapeOptions { source_rectangle: Some(Rectangle::new(x, y, w, h)), ..ImageShapeOptions::default() }).ok().map(DrawingShape::from)
            },
            0.0,
        ));
        let tinted = session.draw_image(&landscape, RectangleF::new(900.0, 140.0, PICTURE_WIDTH, PICTURE_HEIGHT), ImageSampling::Linear).unwrap();
        let tints = ColorTrack::from_keys(&[(0.0, Color::WHITE, L), (2.5, Color::from_rgb(255, 170, 110), EasingKind::SineInOut), (5.0, Color::from_rgb(70, 90, 170), EasingKind::SineInOut), (8.0, Color::WHITE, EasingKind::SineInOut)]).unwrap();
        session.animations_mut().bind(tinted, AnimatedProperty::Tint, tints).unwrap();
        let nearest = ImageShapeOptions { sampling: ImageSampling::Nearest, ..ImageShapeOptions::default() };
        session.draw_shape(ImageShape::new(heart.clone(), RectangleF::new(90.0, 450.0, 221.0, 187.0), nearest).unwrap());
        session.draw_shape(ImageShape::new(heart, RectangleF::new(490.0, 450.0, 221.0, 187.0), ImageShapeOptions::default()).unwrap());
        // The backend's picture (a 512 x 512 colour map), its top-left quarter drawn nearest: on the
        // left as pixels the backend decoded (the picture-pixels form), on the right as the PNG FILE
        // itself, served by the backend and decoded by the crate (ImageSource::from_png).
        let quarter = ImageShapeOptions { source_rectangle: Some(Rectangle::new(0, 0, 256, 256)), ..nearest };
        session.draw_shape(ImageShape::new(backend_picture(), RectangleF::new(338.0, 530.0, 60.0, 60.0), quarter).unwrap());
        session.draw_shape(outline_rect(338.0, 530.0, 60.0, 60.0, 1.0, Some(DIM), 0.0));
        session.draw_shape(ImageShape::new(backend_png_picture(), RectangleF::new(404.0, 530.0, 60.0, 60.0), quarter).unwrap());
        session.draw_shape(outline_rect(404.0, 530.0, 60.0, 60.0, 1.0, Some(DIM), 0.0));
        // Emblems over transparency in a turning group, kept to the panel by a still group's mask.
        let mut emblems = DrawingGroup::new(None);
        emblems.add(ImageShape::new(emblem.clone(), RectangleF::new(PANEL.x + 60.0, PANEL.y + 75.0, 100.0, 100.0), ImageShapeOptions::default()).unwrap());
        let late = ImageShape::new(emblem, RectangleF::new(PANEL.x + 200.0, PANEL.y + 75.0, 100.0, 100.0), ImageShapeOptions { tint: Some(Color::from_rgba(150, 220, 255, 200)), ..ImageShapeOptions::default() }).unwrap();
        emblems.add(DrawingShape::from(late).with_time_window(Some(2.0), None));
        let centre = PointF::new(PANEL.x + PANEL.width / 2.0, PANEL.y + PANEL.height / 2.0);
        let emblems = DrawingShape::from(emblems).with_transform(DrawingTransform::rotate(0.0, centre));
        let emblem_id = emblems.id();
        let mut panel = DrawingGroup::new(None);
        panel.add(emblems);
        panel.set_mask(Some(DrawingMask::inside(PathGeometry::rectangle(PANEL))));
        session.draw_shape(panel);
        session.animations_mut().bind(emblem_id, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, 360.0, L)])).unwrap();
        ImageScene { session }
    }
}

impl DemoScene for ImageScene {
    fn name(&self) -> &'static str {
        "images"
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
