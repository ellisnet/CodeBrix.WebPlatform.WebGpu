//! "colour-pickup" (AnimationDemoApp's ColorPickupScene): a sunset painted per pixel at half size
//! (a plane image; the sun sinks, the river winds) and, beside it, a brushstroke mosaic of it -
//! every stroke of a fixed ownership map takes the MEAN COLOUR of the picture beneath it, every
//! frame (get_mean_colors, exact, on the CPU). The mosaic is COLOURED ON THE GPU (OwnershipShape:
//! the map goes up as owner numbers with the new colours beside it, the seams computed per pixel),
//! so a frame costs the picture and the means on the CPU, not a pass over every map pixel.

use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::painting::DrawingOwnershipMap;
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{OwnershipShape, OwnershipShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::brushwork::layout;
use crate::demo::{create_frame_session, create_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Colour pickup: a picture as a brushstroke mosaic";
const DESCRIPTION: &str = "Each brushstroke of an ownership map takes the mean colour of the picture beneath it, every frame (the mosaic coloured on the GPU)";
const MAP_WIDTH: u32 = 560;
const MAP_HEIGHT: u32 = 420;
const PANEL_Y: f32 = 150.0;
const PANEL_X: [f32; 2] = [48.0, 672.0];

pub struct ColorPickupScene {
    frame: DrawingSession,
    map: DrawingOwnershipMap,
}

/// A sunset at half size: sky gradient, a sinking sun with a glow, two rows of hills and a winding river.
fn picture(seconds: f64) -> DrawingPlaneImage {
    let (w, h) = (MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let (sun_x, sun_y) = (70.0 + 140.0 * (seconds / 8.0), 60.0 + 50.0 * (seconds / 8.0));
    let mut picture = DrawingPlaneImage::new(w, h).unwrap();
    picture.fill_with(|x, y| {
        let (x, y) = (x as f64, y as f64);
        let v = y / h as f64;
        let (mut r, mut g, mut b) = (0.95 - 0.55 * v, 0.55 - 0.35 * v, 0.35 + 0.25 * v);
        let (dx, dy) = (x - sun_x, y - sun_y);
        let d = (dx * dx + dy * dy).sqrt();
        let glow = (-d / 40.0).exp();
        r += 0.5 * glow;
        g += 0.35 * glow;
        if d < 18.0 {
            (r, g, b) = (1.0, 0.92, 0.6);
        }
        let far = 120.0 + 14.0 * (x / 37.0 + 1.0).sin() + 8.0 * (x / 11.0).sin();
        let near = 160.0 + 18.0 * (x / 53.0 + 2.0).sin() + 6.0 * (x / 7.0 + seconds).sin();
        if y > far {
            (r, g, b) = (0.32, 0.22 + 0.1 * glow, 0.36);
        }
        if y > near {
            (r, g, b) = (0.12, 0.28, 0.18);
        }
        let river = w as f64 / 2.0 + 40.0 * (y / 14.0 - seconds * 0.8).sin() + (y - near) * 0.6;
        if y > near && (x - river).abs() < 4.0 + (y - near) * 0.35 {
            (r, g, b) = (0.55 + 0.3 * glow, 0.6, 0.85);
        }
        [r as f32, g as f32, b as f32, 1.0]
    });
    picture
}

impl ColorPickupScene {
    pub fn new(fonts: &FontRegistry) -> ColorPickupScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "the picture: DrawingPlaneImage, 280 x 210, painted per pixel (shown enlarged)", PANEL_X[0], PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "get_mean_colors(picture) -> OwnershipShape: one colour per stroke", PANEL_X[1], PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "Each map pixel reads the picture pixel at floor((2x + 1) * 280 / 1120): half-size sources need no resizing.", PANEL_X[0], 600.0, 14.0, DIM, TextAlign::Left);
        label(captions, fonts, "Means are alpha-weighted and summed row by row on one thread (C#'s values); the mosaic and its seams are coloured on the GPU.", PANEL_X[0], 624.0, 14.0, DIM, TextAlign::Left);
        frame.add_layer("Panels", INK);
        let mut strokes = create_session(MAP_WIDTH, MAP_HEIGHT, Color::TRANSPARENT, fonts);
        let layer = strokes.add_layer("Strokes", Color::WHITE);
        for stroke in layout(MAP_WIDTH, MAP_HEIGHT, 46.0, 9) {
            layer.add_shape(stroke);
        }
        let map = strokes.export_layer_ownership_map("Strokes", (MAP_WIDTH, MAP_HEIGHT)).unwrap().fill_unowned();
        ColorPickupScene { frame, map }
    }
}

impl DemoScene for ColorPickupScene {
    fn name(&self) -> &'static str {
        "colour-pickup"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let picture = picture(seconds);
        let means = self.map.get_mean_colors(&picture);
        let place = |x: f32| RectangleF::new(x, PANEL_Y, MAP_WIDTH as f32, MAP_HEIGHT as f32);
        let options = OwnershipShapeOptions { unowned: Color::BLACK, seam_color: Some(Color::from_rgb(20, 16, 30)), seam_strength: 1.3 * 0.7 };
        let mosaic = OwnershipShape::from_float_colors(&self.map, &means, place(PANEL_X[1]), options).unwrap();
        let shown = ImageSource::from_plane_image(&picture).unwrap();
        self.frame.set_time(seconds);
        let panels = self.frame.get_layer_mut("Panels").unwrap();
        panels.clear();
        panels.add_shape(mosaic);
        self.frame.set_active_layer("Panels");
        self.frame.draw_image(&shown, place(PANEL_X[0]), ImageSampling::Linear).unwrap();
        self.frame.render_with_background(scene, width, height);
    }
}
