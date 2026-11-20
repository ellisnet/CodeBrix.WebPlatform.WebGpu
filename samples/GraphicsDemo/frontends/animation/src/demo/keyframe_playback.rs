//! "keyframe-playback" (AnimationDemoApp's KeyframePlaybackScene): a painting made every second
//! (three stroke layouts in turn pick up the colours of a sky that changes with time) and the frames
//! between dissolved from keyframe k to k + 1. THE REAL-TIME ROUTE of the painterly playback here:
//! the KeyframePlayer paints AHEAD one keyframe at a time (prefetch after each frame - one thread,
//! no worker), and the DISSOLVE runs ON THE GPU (keyframe k drawn, keyframe k + 1 over it at the
//! fraction u - exact for opaque frames), so a frame costs no CPU mixing at all; C# mixes on the
//! CPU. The two keyframes are shown as thumbnails on the right with the strip of keyframes and the
//! play head below them; the read-out counts the paint calls and the cache.

use codebrix_webgpu_animation::images::ImageSource;
use codebrix_webgpu_animation::painting::{DrawingOwnershipMap, KeyframePlayer, KeyframePlayerOptions};
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{ImageShape, ImageShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::brushwork::layout;
use crate::demo::kit::fill_rect;
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Keyframe playback: paint every second, dissolve between";
const DESCRIPTION: &str = "KeyframePlayer: keyframes painted ahead one at a time, a bounded cache, exact frames after seeks; the dissolve drawn on the GPU";
const W: u32 = 480;
const H: u32 = 360;
const PANEL_Y: f32 = 150.0;
const LEFT: f32 = 48.0;

pub struct KeyframePlaybackScene {
    frame: DrawingSession,
    player: KeyframePlayer,
    /// The pictures of the keyframes on screen (keyframe number, picture), kept while shown.
    pictures: Vec<(i32, ImageSource)>,
    buffer: Vec<u8>,
    fonts: FontRegistry,
}

/// One keyframe: a sky that changes with time, picked up by the keyframe's stroke layout.
fn paint(maps: &[DrawingOwnershipMap], keyframe: i32, seconds: f64, rgba: &mut [u8]) -> Result<(), String> {
    let map = &maps[keyframe.rem_euclid(maps.len() as i32) as usize];
    let mut sky = DrawingPlaneImage::new(W / 2, H / 2)?;
    let (sun_x, sun_y) = (40.0 + 25.0 * seconds, 50.0 + 8.0 * seconds.sin());
    sky.fill_with(|x, y| {
        let (x, y) = (x as f64, y as f64);
        let v = y / (H / 2) as f64;
        let swirl = (x / 17.0 + y / 23.0 + seconds * 0.7).sin();
        let d = ((x - sun_x) * (x - sun_x) + (y - sun_y) * (y - sun_y)).sqrt();
        let sun = (-d / 22.0).exp();
        [(0.1 + 0.25 * v + 0.9 * sun) as f32, (0.18 + 0.12 * swirl + 0.6 * sun) as f32, (0.55 - 0.25 * v + 0.1 * swirl) as f32, 1.0]
    });
    let mut painting = DrawingPlaneImage::new(W, H)?;
    map.apply_colors(&mut painting, &map.get_mean_colors(&sky))?;
    let mut seams = map.create_boundary_plane(Some(0.7))?;
    seams.multiply(0.9).clamp(0.0, 1.0)?;
    painting.blend_color(Color::from_rgb(12, 14, 34), &seams)?;
    painting.to_rgba_into(rgba)
}

impl KeyframePlaybackScene {
    pub fn new(fonts: &FontRegistry) -> KeyframePlaybackScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "keyframe k, then keyframe k + 1 over it at u: the dissolve on the GPU", LEFT, PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "keyframe k", 600.0, PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "keyframe k + 1", 870.0, PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "One painting per second on three stroke layouts in turn; k = floor(t / interval), u = t / interval - k.", LEFT, 640.0, 14.0, DIM, TextAlign::Left);
        label(captions, fonts, "One thread: prefetch paints at most one keyframe per frame, ahead of the play head; seek with Left/Right - frames stay exact.", LEFT, 662.0, 14.0, DIM, TextAlign::Left);
        frame.add_layer("Frames", INK);
        frame.add_layer("Readout", DIM);
        let maps: Vec<DrawingOwnershipMap> = (0..3)
            .map(|i| {
                let mut strokes = create_session(W, H, Color::TRANSPARENT, fonts);
                let layer = strokes.add_layer("Strokes", Color::WHITE);
                for stroke in layout(W, H, 40.0 + 6.0 * i as f32, 100 + i as i64) {
                    layer.add_shape(stroke);
                }
                strokes.export_layer_ownership_map("Strokes", (W, H)).unwrap().fill_unowned()
            })
            .collect();
        let player = KeyframePlayer::new(W, H, 1.0, move |k, seconds, rgba| paint(&maps, k, seconds, rgba), KeyframePlayerOptions { cache_capacity: 6, ..KeyframePlayerOptions::default() }).unwrap();
        KeyframePlaybackScene { frame, player, pictures: Vec::new(), buffer: vec![0; (W * H * 4) as usize], fonts: fonts.clone() }
    }

    /// The picture of keyframe k (read through the player - cached, so a copy - and kept while it
    /// is on screen, so it is uploaded to the GPU once).
    fn keyframe_picture(&mut self, k: i32) -> ImageSource {
        if let Some((_, p)) = self.pictures.iter().find(|(n, _)| *n == k) {
            return p.clone();
        }
        self.player.render_frame(k as f64 * 1.0, &mut self.buffer).unwrap();
        let picture = ImageSource::from_rgba(&self.buffer, W, H).unwrap();
        self.pictures.push((k, picture.clone()));
        if self.pictures.len() > 4 {
            self.pictures.remove(0);
        }
        picture
    }
}

impl DemoScene for KeyframePlaybackScene {
    fn name(&self) -> &'static str {
        "keyframe-playback"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let seconds = seconds.max(0.0);
        let k = self.player.keyframe_at(seconds).unwrap();
        let u = (seconds - k as f64).clamp(0.0, 1.0) as f32;
        let (a, b) = (self.keyframe_picture(k), self.keyframe_picture(k + 1));
        self.frame.set_time(seconds);
        let frames = self.frame.get_layer_mut("Frames").unwrap();
        frames.clear();
        let place = RectangleF::new(LEFT, PANEL_Y, W as f32, H as f32);
        frames.add_shape(ImageShape::new(a.clone(), place, ImageShapeOptions::default()).unwrap());
        if u > 0.0 {
            let mut over: codebrix_webgpu_animation::shapes::DrawingShape = ImageShape::new(b.clone(), place, ImageShapeOptions::default()).unwrap().into();
            over.set_opacity(u);
            frames.add_shape(over);
        }
        frames.add_shape(ImageShape::new(a, RectangleF::new(600.0, PANEL_Y, 240.0, 180.0), ImageShapeOptions::default()).unwrap());
        frames.add_shape(ImageShape::new(b, RectangleF::new(870.0, PANEL_Y, 240.0, 180.0), ImageShapeOptions::default()).unwrap());
        // The strip: a tick per keyframe, the play head in the accent colour.
        frames.add_shape(fill_rect(600.0, 380.0, 510.0, 24.0, Some(Color::from_rgb(44, 48, 66)), 0.0));
        for i in 0..=8 {
            frames.add_shape(fill_rect(600.0 + i as f32 * 510.0 / 8.0 - 1.0, 380.0, 2.0, 24.0, Some(Color::from_rgb(200, 204, 220)), 0.0));
        }
        frames.add_shape(fill_rect(600.0 + (seconds as f32 * 510.0 / 8.0).min(510.0) - 1.5, 380.0, 3.0, 24.0, Some(ACCENT), 0.0));
        let fonts = self.fonts.clone();
        let (paints, cached) = (self.player.paint_count(), self.player.cached_keyframe_count());
        let readout = self.frame.get_layer_mut("Readout").unwrap();
        readout.clear();
        label(readout, &fonts, &format!("t = {seconds:.2} s    keyframe {k} -> {}    u = {u:.2}", k + 1), LEFT, PANEL_Y + H as f32 + 32.0, 17.0, INK, TextAlign::Left);
        label(readout, &fonts, &format!("paint calls so far: {paints}    keyframes cached: {cached} (of 6)"), LEFT, PANEL_Y + H as f32 + 58.0, 14.0, DIM, TextAlign::Left);
        self.frame.render_with_background(scene, width, height);
        // Paint ahead: at most one keyframe per frame, after this frame is recorded.
        let _ = self.player.prefetch(seconds);
    }
}
