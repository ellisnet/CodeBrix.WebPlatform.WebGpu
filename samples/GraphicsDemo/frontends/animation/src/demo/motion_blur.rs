//! "motion-blur" (AnimationDemoApp's MotionBlurScene): a ball swinging fast from side to side and a
//! bar spinning about the centre, shown as one sharp frame and as the AVERAGE OF 16 SUB-FRAMES
//! spread over the shutter (DrawingMotionBlur::get_sample_times, exact) - over a backdrop and over a
//! checkerboard, so the streak fades into transparency. C# renders each sub-frame to pixels and
//! averages them on the CPU (DrawingFrameAccumulator); here the sub-frames are averaged ON THE GPU
//! by the renderer's compositing: each sub-frame is a group composited with Compose::Plus at 1/16
//! into one offscreen group (sums in the 8-bit target, so a little coarser than C#'s exact integer
//! sums). The CPU accumulator stays available for exact averages of pixels the application has.

use codebrix_webgpu_animation::kurbo::{Affine, Rect};
use codebrix_webgpu_animation::peniko::{BlendMode, Compose, Mix};
use codebrix_webgpu_animation::planes::DrawingMotionBlur;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{PolylineShape, PolylineShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, SessionRenderOptions};

use crate::demo::kit::{checkerboard, disc, fill_rect, part};
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Motion blur";
const DESCRIPTION: &str = "Average sub-frames spread across the shutter; premultiplied averaging keeps a streak over transparency correct (averaged on the GPU)";
const PANEL: f32 = 384.0;
const PANEL_Y: f32 = 128.0;
const SAMPLES: u32 = 16;
const FRAME_DURATION: f64 = 1.0 / 24.0;
const PANEL_X: [f32; 3] = [32.0, 448.0, 864.0];

pub struct MotionBlurScene {
    frame: DrawingSession,
    drawing: DrawingSession,
}

impl MotionBlurScene {
    pub fn new(fonts: &FontRegistry) -> MotionBlurScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "one sharp frame (no blur)", PANEL_X[0], PANEL_Y + PANEL + 26.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "16 sub-frames averaged, over a backdrop", PANEL_X[1], PANEL_Y + PANEL + 26.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "the same average over a checkerboard", PANEL_X[2], PANEL_Y + PANEL + 26.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "DrawingMotionBlur::get_sample_times(t, 1/24 s, 16, shutter 1) -> the drawing recorded at each sample time", 32.0, PANEL_Y + PANEL + 62.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "-> each sample a group composited Compose::Plus at 1/16 into one offscreen group: the average, premultiplied, on the GPU", 32.0, PANEL_Y + PANEL + 84.0, 13.0, DIM, TextAlign::Left);
        let back = frame.add_layer("Backdrops", INK);
        for x in [PANEL_X[0], PANEL_X[1]] {
            back.add_shape(fill_rect(x, PANEL_Y, PANEL, PANEL, Some(Color::from_rgb(26, 31, 56)), 0.0));
        }
        checkerboard(back, PANEL_X[2], PANEL_Y, PANEL, PANEL);
        let mut drawing = create_session(PANEL as u32, PANEL as u32, Color::TRANSPARENT, fonts);
        drawing.add_layer("Moving", Color::WHITE);
        MotionBlurScene { frame, drawing }
    }

    /// The drawing at one moment: a ball swinging from side to side and a bar spinning.
    fn moment(&mut self, time: f64) {
        let moving = self.drawing.get_layer_mut("Moving").unwrap();
        moving.clear();
        let ball_x = (192.0 + 140.0 * (time * 5.0).sin()) as f32;
        moving.add_shape(disc(ball_x, 96.0, 30.0, Some(ACCENT)));
        let angle = time * 9.0;
        let (dx, dy) = ((120.0 * angle.cos()) as f32, (120.0 * angle.sin()) as f32);
        moving.add_shape(PolylineShape::new(&[PointF::new(192.0 - dx, 250.0 - dy), PointF::new(192.0 + dx, 250.0 + dy)], PolylineShapeOptions { stroke_thickness: 18.0, color: Some(Color::from_rgb(120, 200, 255)), ..PolylineShapeOptions::default() }).unwrap());
        self.drawing.set_time(time);
    }
}

impl DemoScene for MotionBlurScene {
    fn name(&self) -> &'static str {
        "motion-blur"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        let targets: Vec<SessionRenderOptions> = PANEL_X.iter().map(|x| SessionRenderOptions { include_background: false, target: Some(part(&self.frame, *x, PANEL_Y, PANEL, PANEL, width, height)) }).collect();
        self.moment(seconds);
        let sharp = targets[0];
        self.drawing.render_with(scene, width, height, &sharp);
        let times = DrawingMotionBlur::get_sample_times(seconds, FRAME_DURATION, SAMPLES, Some(1.0)).unwrap();
        let plus = BlendMode::new(Mix::Normal, Compose::Plus);
        for target in [targets[1], targets[2]] {
            // The groups cover only the panel: a compose other than source-over composites its
            // whole shape, so a canvas-sized shape would cost a canvas-sized pass per sample.
            let r = target.target.unwrap();
            let canvas = Rect::new(r.x as f64, r.y as f64, (r.x + r.width) as f64, (r.y + r.height) as f64);
            // One group holds the average (kept offscreen by the renderer because it holds groups
            // of another compose mode): each sample adds 1/16 of itself.
            scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &canvas);
            for t in &times {
                self.moment(*t);
                scene.push_layer(plus, 1.0 / SAMPLES as f32, Affine::IDENTITY, &canvas);
                self.drawing.render_with(scene, width, height, &target);
                scene.pop_layer();
            }
            scene.pop_layer();
        }
    }
}
