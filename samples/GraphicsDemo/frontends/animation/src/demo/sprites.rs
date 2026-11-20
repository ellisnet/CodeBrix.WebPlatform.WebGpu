//! "sprites" (AnimationDemoApp's SpriteScene): sprites from sheets GENERATED in code - a
//! bouncing-ball sheet of eight 96 x 96 cells and a pixel-art flame of twelve 16 x 24 cells (see
//! pictures.rs). Top: the ball sheet itself, the loop's current cell outlined. Middle: the sheet
//! played by time as a loop, a ping-pong and a "hold" (it waits, plays once and stops), and one
//! stepped by hand through its FrameIndex (a float track). Right: the flame scaled up seven times
//! with nearest sampling, with bilinear sampling, and tinted. Every frame is a function of the
//! session time.

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::images::{ImageSampling, SpritePlayback, SpriteSheet};
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{AnimatedShape, SpriteShape, SpriteShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::kit::{fill_rect, floats, outline_rect, L};
use crate::demo::pictures::{ball_sheet, flame_sheet};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Sprites from generated sheets";
const DESCRIPTION: &str = "ImageSource.FromRgba, SpriteSheet.FromGrid, SpriteShape (fps, Loop/PingPong/Hold, FrameIndex, PlaybackStartTime, Tint, sampling)";
const CELL: f32 = 96.0;

pub struct SpriteScene {
    session: DrawingSession,
}

impl SpriteScene {
    pub fn new(fonts: &FontRegistry) -> SpriteScene {
        let ball_image = ball_sheet();
        let balls = SpriteSheet::from_grid(&ball_image, 8, 1, 0).unwrap();
        let flames = SpriteSheet::from_grid(&flame_sheet(), 12, 1, 0).unwrap();
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "the generated sheet (8 cells), the loop's current cell outlined", 40.0, 130.0, 14.0, DIM, TextAlign::Left);
        label(notes, fonts, "Loop, 12 fps", 130.0, 548.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "PingPong, 12 fps", 345.0, 548.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "Hold (starts at 2 s)", 560.0, 548.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "FrameIndex from a track", 775.0, 548.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "flame from noise:", 1086.0, 548.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "nearest | bilinear | tinted", 1086.0, 568.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "SpriteSheet::from_grid cuts each cell into its own picture, so cells never bleed into each other", 40.0, 660.0, 14.0, DIM, TextAlign::Left);
        session.add_layer("Sprites", INK);
        let floor = Color::from_rgb(30, 33, 46);
        session.draw_shape(fill_rect(40.0, 146.0, CELL * 8.0, CELL, Some(floor), 0.0));
        session.draw_image(&ball_image, RectangleF::new(40.0, 146.0, CELL * 8.0, CELL), ImageSampling::Linear).unwrap();
        for i in 0..4 {
            session.draw_shape(fill_rect(30.0 + i as f32 * 215.0, 320.0, 200.0, 200.0, Some(floor), 12.0));
        }
        let timed = |fps: f64, playback: SpritePlayback| SpriteShapeOptions { frames_per_second: fps, playback, sampling: ImageSampling::Linear };
        session.draw_sprite(&balls, RectangleF::new(30.0, 320.0, 200.0, 200.0), timed(12.0, SpritePlayback::Loop)).unwrap();
        session.draw_sprite(&balls, RectangleF::new(245.0, 320.0, 200.0, 200.0), timed(12.0, SpritePlayback::PingPong)).unwrap();
        let mut hold = SpriteShape::new(balls.clone(), RectangleF::new(460.0, 320.0, 200.0, 200.0), timed(6.0, SpritePlayback::Hold)).unwrap();
        hold.set_playback_start_time(2.0).unwrap();
        session.draw_shape(hold);
        let stepped = session.draw_sprite(&balls, RectangleF::new(675.0, 320.0, 200.0, 200.0), SpriteShapeOptions::default()).unwrap();
        session.animations_mut().bind(stepped, AnimatedProperty::FrameIndex, floats(&[(0.0, 0.0, L), (4.0, 7.99, EasingKind::SineInOut), (8.0, 0.0, EasingKind::SineInOut)])).unwrap();
        let flame = |x: f32, sampling: ImageSampling| SpriteShape::new(flames.clone(), RectangleF::new(x, 340.0, 16.0 * 7.0, 24.0 * 7.0), SpriteShapeOptions { frames_per_second: 10.0, playback: SpritePlayback::Loop, sampling }).unwrap();
        session.draw_shape(flame(915.0, ImageSampling::Nearest));
        session.draw_shape(flame(1030.0, ImageSampling::Linear));
        session.draw_shape(flame(1145.0, ImageSampling::Nearest).with_tint(Some(Color::from_rgb(255, 120, 210))).with_frame_index(5));
        // The loop's current cell, outlined on the sheet: the same frame arithmetic as the sprite.
        let looping = SpriteShape::new(balls, RectangleF::new(30.0, 320.0, 200.0, 200.0), timed(12.0, SpritePlayback::Loop)).unwrap();
        session.add_layer("Now", ACCENT);
        session.draw_shape(AnimatedShape::new(move |t| Some(outline_rect(40.0 + looping.get_frame_at(t) as f32 * CELL, 146.0, CELL, CELL, 3.0, Some(ACCENT), 0.0).into()), 0.0));
        SpriteScene { session }
    }
}

impl DemoScene for SpriteScene {
    fn name(&self) -> &'static str {
        "sprites"
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
