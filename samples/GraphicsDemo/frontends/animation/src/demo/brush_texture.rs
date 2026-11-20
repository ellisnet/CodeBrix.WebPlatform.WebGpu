//! "brush-texture" (AnimationDemoApp's BrushTextureScene): eight brushstrokes bending and turning
//! on a 290 x 340 drawing, their ownership map made every frame and textured in each stroke's OWN
//! axes - four panels: the flat colours (apply_colors), times bristle ridges across each stroke
//! (fill_by_element), times a ramp along it, and with dark seams (create_boundary_plane through
//! blend_color). All CPU calculation with NoSkia's values at the C# demo's own small size, shown
//! through pictures (ImageSource::from_plane_image).

use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::planes::{DrawingPlane, DrawingPlaneImage, DrawingRandom};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokeShape, BrushStrokeShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, RectangleF};

use crate::demo::{create_frame_session, create_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Brush texture: measuring pixels in a stroke's own axes";
const DESCRIPTION: &str = "Ownership map + fill_by_element (along / across each stroke) + create_boundary_plane - bristles, ramps and seams";
const PW: u32 = 290;
const PH: u32 = 340;
const PANEL_Y: f32 = 160.0;
const PANEL_X: [f32; 4] = [22.0, 336.0, 650.0, 964.0];
const PAPER: Color = Color::from_rgb(236, 228, 210);
const COLORS: [[f32; 4]; 8] = [[0.16, 0.27, 0.62, 1.0], [0.91, 0.62, 0.15, 1.0], [0.2, 0.52, 0.47, 1.0], [0.78, 0.25, 0.18, 1.0], [0.33, 0.4, 0.76, 1.0], [0.95, 0.82, 0.36, 1.0], [0.12, 0.36, 0.3, 1.0], [0.62, 0.3, 0.52, 1.0]];

pub struct BrushTextureScene {
    frame: DrawingSession,
    drawing: DrawingSession,
    phases: Vec<f32>,
}

impl BrushTextureScene {
    pub fn new(fonts: &FontRegistry) -> BrushTextureScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        let titles = ["1  apply_colors: flat colour", "2  x bristle ridges", "3  x ramp along the stroke", "4  + dark seams"];
        let notes = ["the ownership map, one colour per stroke", "fill_by_element: 1 + 0.2 sin(across x 1.1 + phase)", "fill_by_element: 0.8 + 0.25 x normalized_along", "create_boundary_plane(1.1) through blend_color"];
        for i in 0..4 {
            label(captions, fonts, titles[i], PANEL_X[i], PANEL_Y + PH as f32 + 28.0, 15.0, INK, TextAlign::Left);
            label(captions, fonts, notes[i], PANEL_X[i], PANEL_Y + PH as f32 + 50.0, 13.0, DIM, TextAlign::Left);
        }
        label(captions, fonts, "Across and along are measured in each stroke's own axes (centre, angle, length, width from BrushStrokeShape::get_axes),", 22.0, 640.0, 14.0, DIM, TextAlign::Left);
        label(captions, fonts, "so the ridges turn and bend with the strokes. Paper (unowned pixels) is left untouched: unowned value 1.", 22.0, 662.0, 14.0, DIM, TextAlign::Left);
        frame.add_layer("Panels", INK);
        let mut drawing = create_session(PW, PH, Color::TRANSPARENT, fonts);
        drawing.add_layer("Paint", Color::WHITE);
        let mut random = DrawingRandom::new(77);
        let phases = (0..COLORS.len()).map(|_| random.uniform(0.0, 2.0 * std::f64::consts::PI) as f32).collect();
        BrushTextureScene { frame, drawing, phases }
    }
}

impl DemoScene for BrushTextureScene {
    fn name(&self) -> &'static str {
        "brush-texture"
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
        for i in 0..COLORS.len() {
            let (row, column) = ((i / 2) as f64, (i % 2) as f64);
            let bend = (0.14 * (seconds * 0.9 + i as f64 * 1.3).sin()) as f32;
            let angle = ((if column == 0.0 { 29.0 } else { -23.0 }) + 14.0 * (seconds * 0.5 + i as f64).sin()) as f32;
            paint.add_shape(BrushStrokeShape::new((80.0 + 130.0 * column) as f32, (52.0 + 78.0 * row) as f32, 230.0, 66.0, angle, BrushStrokeShapeOptions { bend, ..BrushStrokeShapeOptions::default() }).unwrap());
        }
        self.drawing.set_time(seconds);
        let map = self.drawing.export_layer_ownership_map("Paint", (PW, PH)).unwrap();
        let shift = seconds as f32;
        let phases = self.phases.clone();
        let mut ridges = DrawingPlane::new(PW, PH).unwrap();
        map.fill_by_element(&mut ridges, |p| (1.0 + 0.2 * (p.across() * 1.1 + phases[p.element_index()] as f64 + shift as f64).sin()) as f32, Some(1.0), None).unwrap();
        let mut ramp = DrawingPlane::new(PW, PH).unwrap();
        map.fill_by_element(&mut ramp, |p| (0.8 + 0.25 * p.normalized_along()) as f32, Some(1.0), None).unwrap();
        let mut seams = map.create_boundary_plane(Some(1.1)).unwrap();
        seams.multiply(1.6).clamp(0.0, 1.0).unwrap();
        let mut flat = DrawingPlaneImage::new(PW, PH).unwrap();
        flat.fill_color(PAPER);
        map.apply_colors(&mut flat, &COLORS).unwrap();
        let mut ridged = flat.clone();
        ridged.multiply_plane(&ridges).unwrap();
        let mut ramped = ridged.clone();
        ramped.multiply_plane(&ramp).unwrap();
        let mut finished = ramped.clone();
        seams.multiply(0.8);
        finished.blend_color(Color::from_rgb(24, 22, 40), &seams).unwrap();
        self.frame.set_time(seconds);
        self.frame.get_layer_mut("Panels").unwrap().clear();
        self.frame.set_active_layer("Panels");
        for (i, image) in [flat, ridged, ramped, finished].iter().enumerate() {
            let picture = ImageSource::from_plane_image(image).unwrap();
            self.frame.draw_image(&picture, RectangleF::new(PANEL_X[i], PANEL_Y, PW as f32, PH as f32), ImageSampling::Nearest).unwrap();
        }
        self.frame.render_with_background(scene, width, height);
    }
}
