//! "hard-edges" (AnimationDemoApp's HardEdgeScene): a tiny 80 x 60 drawing - a turning star, a
//! breathing circle, a swinging brushstroke and a thin line - shown three ways at 5x: as the
//! renderer DRAWS it (anti-aliased by the GPU's multisampling), as its HARD-EDGED mask
//! (export_mask with MaskMeaning::HardEdged: 0 or 1 per pixel by the pixel-centre rule, the same
//! bits as C#), and that mask USED as a plane mask (DrawingMask::from_plane) over moving stripes.
//! C#'s comparison with the anti-aliased Coverage mask is left out (that mask is a GPU readback
//! here, kept for tests; the owner's ruling).

use codebrix_webgpu_animation::effects::DrawingMask;
use codebrix_webgpu_animation::fills::LinearGradientFill;
use codebrix_webgpu_animation::images::{ImageSampling, ImageSource};
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::planes::DrawingPlaneImage;
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{BrushStrokeShapeOptions, DrawingGroup, RectangleShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, MaskMeaning, PointF, RectangleF, SessionRenderOptions};

use crate::demo::kit::{disc, line, part, points, polygon, L};
use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, create_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Hard-edged masks: one pixel rule";
const DESCRIPTION: &str = "MaskMeaning::HardEdged: coverage by pixel centres, computed from geometry on the CPU - the same bits as C#";
const DW: u32 = 80;
const DH: u32 = 60;
const ZOOM: f32 = 5.0;
const PANEL_Y: f32 = 170.0;
const PANEL_X: [f32; 3] = [32.0, 448.0, 864.0];

pub struct HardEdgeScene {
    frame: DrawingSession,
    drawing: DrawingSession,
}

impl HardEdgeScene {
    pub fn new(fonts: &FontRegistry) -> HardEdgeScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let captions = frame.add_layer("Captions", DIM);
        let below = PANEL_Y + DH as f32 * ZOOM + 26.0;
        label(captions, fonts, "drawn by the renderer at 5x", PANEL_X[0], below, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "anti-aliased edges (4x multisampling)", PANEL_X[0], below + 20.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "export_mask(size, MaskMeaning::HardEdged)", PANEL_X[1], below, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "0 or 1 per pixel - the same bits as C#", PANEL_X[1], below + 20.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "that mask as DrawingMask::from_plane", PANEL_X[2], below, 14.0, INK, TextAlign::Left);
        label(captions, fonts, "moving stripes seen through its pixels", PANEL_X[2], below + 20.0, 13.0, DIM, TextAlign::Left);
        label(captions, fonts, "The rule: a pixel belongs to a shape when its CENTRE is inside it, under the shape's fill rule.", 32.0, 600.0, 16.0, INK, TextAlign::Left);
        label(captions, fonts, "A centre exactly on an edge counts on a left or top edge and not on a right or bottom edge, so neighbours never share a pixel.", 32.0, 626.0, 14.0, DIM, TextAlign::Left);
        label(captions, fonts, "Used for: masks as exact data, ownership maps, anything that must come out the same as in C#.", 32.0, 650.0, 14.0, DIM, TextAlign::Left);
        frame.add_layer("Mask", INK);
        // The masked panel: a gradient with white stripes sliding across, masked by the plane.
        let gradient = LinearGradientFill::from_colors(PointF::new(PANEL_X[2], 0.0), PointF::new(PANEL_X[2] + 400.0, 0.0), Color::from_rgb(255, 140, 70), Color::from_rgb(110, 120, 255)).unwrap();
        let masked = frame.add_layer("Masked", INK);
        masked.add_shape(RectangleShape::with_fill(PANEL_X[2], PANEL_Y, 400.0, 300.0, gradient.into(), 0.0).unwrap());
        let mut stripes = DrawingGroup::new(Some(Color::from_rgba(255, 255, 255, 150)));
        for i in 0..14 {
            stripes.add(line(PANEL_X[2] - 100.0 + i as f32 * 40.0, PANEL_Y + 300.0, PANEL_X[2] + i as f32 * 40.0, PANEL_Y, 12.0, None));
        }
        let stripes = masked.add_element(stripes);
        frame.animations_mut().bind(stripes, AnimatedProperty::Position, points(&[(0.0, 0.0, 0.0, L), (8.0, 160.0, 0.0, L)])).unwrap();
        let mut drawing = create_session(DW, DH, Color::from_rgb(8, 9, 14), fonts);
        drawing.add_layer("Shapes", Color::WHITE);
        HardEdgeScene { frame, drawing }
    }

    /// A turning star, a circle that breathes, a brushstroke that swings and a thin line.
    fn build(&mut self, seconds: f64) {
        let shapes = self.drawing.get_layer_mut("Shapes").unwrap();
        shapes.clear();
        let star: Vec<PointF> = (0..10)
            .map(|i| {
                let angle = seconds * 0.4 + i as f64 * std::f64::consts::PI / 5.0;
                let radius = if i % 2 == 0 { 21.0 } else { 9.0 };
                PointF::new((22.0 + radius * angle.cos()) as f32, (24.0 + radius * angle.sin()) as f32)
            })
            .collect();
        shapes.add_shape(polygon(&star, Some(Color::WHITE)));
        shapes.add_shape(disc(60.0, 18.0, (9.0 + 3.0 * ping_pong(seconds, 4.0)) as f32, Some(Color::WHITE)));
        shapes.add_shape(line(4.0, 56.0, 76.0, (50.0 - 6.0 * ping_pong(seconds, 3.0)) as f32, 1.6, Some(Color::WHITE)));
        self.drawing.set_active_layer("Shapes");
        self.drawing.draw_brush_stroke(56.0, 44.0, 34.0, 11.0, (-23.0 + 29.0 * ping_pong(seconds, 6.0)) as f32, BrushStrokeShapeOptions { bend: 0.08, color: Some(Color::WHITE), ..BrushStrokeShapeOptions::default() }).unwrap();
    }
}

impl DemoScene for HardEdgeScene {
    fn name(&self) -> &'static str {
        "hard-edges"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.build(seconds);
        self.drawing.set_time(seconds);
        let hard = self.drawing.export_mask((DW, DH), MaskMeaning::HardEdged).unwrap();
        let mut image = DrawingPlaneImage::new(DW, DH).unwrap();
        let values = hard.data().to_vec();
        image.fill_with(|x, y| {
            let v = values[(y * DW + x) as usize];
            [v, v, v, 1.0]
        });
        let picture = ImageSource::from_plane_image(&image).unwrap();
        self.frame.set_time(seconds);
        let mask = self.frame.get_layer_mut("Mask").unwrap();
        mask.clear();
        self.frame.set_active_layer("Mask");
        self.frame.draw_image(&picture, RectangleF::new(PANEL_X[1], PANEL_Y, DW as f32 * ZOOM, DH as f32 * ZOOM), ImageSampling::Nearest).unwrap();
        self.frame.get_layer_mut("Masked").unwrap().set_mask(Some(DrawingMask::from_plane(&hard, RectangleF::new(PANEL_X[2], PANEL_Y, DW as f32 * ZOOM, DH as f32 * ZOOM), false).unwrap()));
        self.frame.render_with_background(scene, width, height);
        let target = part(&self.frame, PANEL_X[0], PANEL_Y, DW as f32 * ZOOM, DH as f32 * ZOOM, width, height);
        self.drawing.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(target), ..Default::default() });
    }
}
