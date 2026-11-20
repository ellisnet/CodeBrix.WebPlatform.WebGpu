//! "ownership" (AnimationDemoApp's OwnershipScene): forty brushstrokes and three circles drifting
//! over a 560 x 420 drawing, the ownership map exported every frame (hard-edged pixel rule, the last
//! painted wins; dark where nothing covers a pixel) and coloured one colour per element; on the
//! right the map after FillUnowned with the seams between owners (CreateBoundaryPlane) blended
//! in. The MAPS are computed on the CPU every frame at the C# demo's own size (export and
//! fill_unowned, exact); the COLOURING and the SEAMS run on the GPU (OwnershipShape: the map goes
//! up as a picture of owner numbers with the colours beside it) - colouring two whole maps on the
//! CPU every frame (C#'s ApplyColors, CreateBoundaryPlane, BlendColor) did not hold 60 fps here.

use codebrix_webgpu_animation::planes::DrawingRandom;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions, OwnershipShape, OwnershipShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::brushwork::distinct_colors;
use crate::demo::kit::disc;
use crate::demo::{create_frame_session, create_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Ownership map: which element owns each pixel";
const DESCRIPTION: &str = "Hard-edged, last painted wins; Nobody where nothing covers a pixel; fill_unowned gives gaps to the nearest owner (exact Euclidean)";
const MAP_WIDTH: u32 = 560;
const MAP_HEIGHT: u32 = 420;
const PANEL_Y: f32 = 150.0;
const STROKE_COUNT: usize = 40;
const PANEL_X: [f32; 2] = [48.0, 672.0];

pub struct OwnershipScene {
    frame: DrawingSession,
    drawing: DrawingSession,
    strokes: Vec<(f64, f64, f64, f64, f64, f64)>,
    colors: Vec<[f32; 4]>,
    fonts: FontRegistry,
}

impl OwnershipScene {
    pub fn new(fonts: &FontRegistry) -> OwnershipScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        label(captions, fonts, "export_layer_ownership_map(layer, size) - one colour per element, unowned dark", PANEL_X[0], PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "fill_unowned() + create_boundary_plane() seams", PANEL_X[1], PANEL_Y - 14.0, 14.0, INK, TextAlign::Left);
        frame.add_layer("Maps", INK);
        frame.add_layer("Readout", DIM);
        let mut drawing = create_session(MAP_WIDTH, MAP_HEIGHT, Color::TRANSPARENT, fonts);
        drawing.add_layer("Paint", Color::WHITE);
        let mut random = DrawingRandom::new(2024);
        let strokes = (0..STROKE_COUNT)
            .map(|_| (random.uniform(0.0, MAP_WIDTH as f64), random.uniform(0.0, MAP_HEIGHT as f64), random.uniform(60.0, 140.0), random.uniform(0.0, 180.0), random.uniform(-23.0, 23.0), random.uniform(0.0, 2.0 * std::f64::consts::PI)))
            .collect();
        OwnershipScene { frame, drawing, strokes, colors: distinct_colors(STROKE_COUNT + 3), fonts: fonts.clone() }
    }
}

impl DemoScene for OwnershipScene {
    fn name(&self) -> &'static str {
        "ownership"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let paint = self.drawing.get_layer_mut("Paint").unwrap();
        paint.clear();
        for (x, y, length, angle, speed, phase) in &self.strokes {
            let drift = (seconds * 0.6 + phase).sin();
            let options = BrushStrokeShapeOptions { bend: (0.12 * drift) as f32, ..BrushStrokeShapeOptions::default() };
            paint.add_shape(BrushStrokeShape::new((x + 30.0 * drift) as f32, (y + 14.0 * (seconds * 0.5 + phase).cos()) as f32, *length as f32, (length * 0.32) as f32, (angle + speed * seconds) as f32, options).unwrap());
        }
        for c in 0..3 {
            let a = seconds * 0.5 + c as f64 * 2.1;
            paint.add_shape(disc((280.0 + 170.0 * a.cos()) as f32, (210.0 + 110.0 * a.sin()) as f32, 34.0, None));
        }
        self.drawing.set_time(seconds);
        let map = self.drawing.export_layer_ownership_map("Paint", (MAP_WIDTH, MAP_HEIGHT)).unwrap();
        let filled = map.fill_unowned();
        // THE COLOURING ON THE GPU: each map goes up as a picture of owner numbers with the
        // colours beside it; the renderer colours every pixel and blends the seams in.
        let left = OwnershipShape::from_float_colors(&map, &self.colors, RectangleF::new(PANEL_X[0], PANEL_Y, MAP_WIDTH as f32, MAP_HEIGHT as f32), OwnershipShapeOptions { unowned: Color::from_rgb(26, 28, 40), ..OwnershipShapeOptions::default() }).unwrap();
        let seams = OwnershipShapeOptions { unowned: Color::BLACK, seam_color: Some(Color::from_rgb(14, 16, 26)), seam_strength: 1.8 };
        let right = OwnershipShape::from_float_colors(&filled, &self.colors, RectangleF::new(PANEL_X[1], PANEL_Y, MAP_WIDTH as f32, MAP_HEIGHT as f32), seams).unwrap();
        self.frame.set_time(seconds);
        let maps = self.frame.get_layer_mut("Maps").unwrap();
        maps.clear();
        maps.add_shape(left);
        maps.add_shape(right);
        let fonts = self.fonts.clone();
        let readout = self.frame.get_layer_mut("Readout").unwrap();
        readout.clear();
        let under = PANEL_Y + MAP_HEIGHT as f32 + 30.0;
        let total = (MAP_WIDTH * MAP_HEIGHT) as f64;
        label(readout, &fonts, &format!("{} elements; {} of {} pixels unowned ({:.1} %)", map.element_count(), map.unowned_count(), total as u64, 100.0 * map.unowned_count() as f64 / total), PANEL_X[0], under, 15.0, INK, TextAlign::Left);
        let top = map.get(MAP_WIDTH / 2, MAP_HEIGHT / 2).unwrap();
        let centre = if top < 0 {
            "centre pixel: Nobody".to_string()
        } else {
            let b = map.get_bounds(top as usize).unwrap();
            format!("centre pixel: element {} ({} pixels, bounds {}, {}, {} x {})", top, map.get_pixel_count(top as usize).unwrap(), b.x, b.y, b.width, b.height)
        };
        label(readout, &fonts, &centre, PANEL_X[0], under + 24.0, 14.0, DIM, TextAlign::Left);
        label(readout, &fonts, &format!("after fill_unowned: {} unowned; ties go to the smaller x, then the smaller y", filled.unowned_count()), PANEL_X[1], under, 15.0, INK, TextAlign::Left);
        label(readout, &fonts, "seams: owner differs from the left or upper neighbour, blurred (sigma 0.8)", PANEL_X[1], under + 24.0, 14.0, DIM, TextAlign::Left);
        self.frame.render_with_background(scene, width, height);
    }
}
