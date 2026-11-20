//! "clip-mask" (AnimationDemoApp's ClipMaskScene): clipping and masks. Top: stripes shown only
//! inside a star - the mask of a group, so it turns with the group; a card with a keyhole cut out
//! of it (a layer mask "outside a path", with a drop shadow) over dots moving behind; a spotlight -
//! a layer whose "inside a circle" mask is replaced every frame as the light moves. Bottom: a
//! layer matte (moving stripes shown only where another layer's text is) and a dissolve. THE
//! DISSOLVE: C# masks it with an alpha mask from a noise plane (`DrawingMask.FromPlane` - pixel
//! planes are a later step of the crate); here the same thresholded noise is a picture on a
//! hidden layer that is the dissolving layer's MATTE, which looks the same.

use codebrix_webgpu_animation::effects::{DrawingEffects, DrawingMask};
use codebrix_webgpu_animation::fills::{LinearGradientFill, RadialGradientFill};
use codebrix_webgpu_animation::images::ImageSampling;
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::paths::{PathGeometry, PathGeometryBuilder};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::shapes::{DrawingGroup, ImageShape, ImageShapeOptions, LineShape, LineShapeOptions, RectangleShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, RectangleF};

use crate::demo::kit::{disc, fill_rect, floats, points, L};
use crate::demo::pictures::{ping_pong, DissolveNoise};
use crate::demo::{create_frame_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Clipping and masks";
const DESCRIPTION: &str = "DrawingMask.Inside/Outside (paths), FromLayer (mattes); on DrawingLayer.Mask and DrawingGroup.Mask - the dissolve uses a matte where C# uses a plane";
const DISSOLVE_AREA: RectangleF = RectangleF::new(680.0, 450.0, 540.0, 220.0);

pub struct ClipMaskScene {
    session: DrawingSession,
    noise: DissolveNoise,
}

impl ClipMaskScene {
    pub fn new(fonts: &FontRegistry) -> ClipMaskScene {
        let mut session = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = session.add_layer("Notes", DIM);
        label(notes, fonts, "a group's mask: inside a star (turns with it)", 230.0, 410.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a layer mask: outside a keyhole", 640.0, 410.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a mask replaced per frame (a spotlight)", 1050.0, 410.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a layer matte: shown where another layer's text is", 330.0, 690.0, 14.0, DIM, TextAlign::Center);
        label(notes, fonts, "a dissolve: a matte of thresholded noise", 950.0, 690.0, 14.0, DIM, TextAlign::Center);

        // Inside a star: a group of stripes and dots, masked by a star in the group's coordinates.
        session.add_layer("Star", INK);
        let mut striped = DrawingGroup::new(None);
        for i in 0..14u8 {
            striped.add(fill_rect(60.0 + i as f32 * 26.0, 90.0, 27.0, 340.0, Some(Color::from_rgb(120 + i * 9, 60 + i * 12, 220 - i * 10)), 0.0));
        }
        for i in 0..5 {
            striped.add(disc(150.0 + i as f32 * 40.0, 180.0 + i as f32 * 40.0, 18.0, Some(Color::from_rgba(255, 255, 255, 170))));
        }
        striped.set_mask(Some(DrawingMask::inside(PathGeometry::star(PointF::new(230.0, 260.0), 135.0, 58.0, 5, 0.0).unwrap())));
        let striped = session.draw_shape(striped);
        let mut a = session.animations_mut();
        a.bind(striped, AnimatedProperty::Anchor, points(&[(0.0, 230.0, 260.0, L)])).unwrap();
        a.bind(striped, AnimatedProperty::Rotation, floats(&[(0.0, 0.0, L), (8.0, 144.0, L)])).unwrap();

        // Outside a keyhole: dots drift behind a card with a hole cut through it.
        session.add_layer("Behind", ACCENT);
        let mut dots = DrawingGroup::new(None);
        for row in 0..8u8 {
            for column in 0..20u8 {
                dots.add(disc(420.0 + column as f32 * 36.0 + (row % 2) as f32 * 18.0, 70.0 + row as f32 * 48.0, 10.0, Some(Color::from_rgb(200 + row * 6, 100 + column * 7, 80))));
            }
        }
        let dots = session.draw_shape(dots);
        session.animations_mut().bind(dots, AnimatedProperty::Position, points(&[(0.0, 0.0, 0.0, L), (8.0, -180.0, 40.0, L)])).unwrap();
        session.get_layer_mut("Behind").unwrap().set_mask(Some(DrawingMask::inside(PathGeometry::rounded_rectangle(RectangleF::new(470.0, 120.0, 340.0, 270.0), 16.0))));
        let card = session.add_layer("Card", Color::from_rgb(60, 66, 90));
        card.add_shape(fill_rect(470.0, 120.0, 340.0, 270.0, None, 16.0));
        card.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("THROUGH THE KEYHOLE", 640.0, 368.0, 16.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(200, 205, 225)), align: TextAlign::Center, tracking: 0.12, ..TextShapeOptions::default() }).unwrap());
        let keyhole = PathGeometryBuilder::new().add_circle(PointF::new(640.0, 210.0), 48.0).add_polygon(&[PointF::new(615.0, 230.0), PointF::new(665.0, 230.0), PointF::new(690.0, 330.0), PointF::new(590.0, 330.0)], true).build();
        card.set_mask(Some(DrawingMask::outside(keyhole)));
        card.set_effects(Some(DrawingEffects::drop_shadow(PointF::new(0.0, 0.0), 10.0, Color::from_rgba(0, 0, 0, 200)).unwrap()));

        // The spotlight: a dim copy underneath, the bright copy shown inside a moving circle.
        for (name, bright) in [("Dim copy", false), ("Spotlight", true)] {
            let layer = session.add_layer(name, if bright { INK } else { Color::from_rgba(236, 236, 244, 40) });
            let big = TextShapeOptions { color: if bright { Some(ACCENT) } else { None }, align: TextAlign::Center, ..TextShapeOptions::default() };
            let small = TextShapeOptions { align: TextAlign::Center, ..TextShapeOptions::default() };
            layer.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("Spotlight", 1050.0, 210.0, 54.0, fonts, big).unwrap());
            layer.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("a circular mask follows", 1050.0, 260.0, 22.0, fonts, small.clone()).unwrap());
            layer.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("the light from frame to frame", 1050.0, 292.0, 22.0, fonts, small).unwrap());
            if bright {
                layer.add_shape(crate::demo::kit::outline_rect(900.0, 320.0, 300.0, 50.0, 3.0, Some(Color::from_rgb(120, 180, 255)), 25.0));
            } else {
                layer.add_shape(fill_rect(900.0, 320.0, 300.0, 50.0, None, 25.0));
            }
        }

        // A layer matte: moving stripes shown only where the hidden matte layer's text is.
        let matte = session.add_layer("Matte", Color::WHITE);
        matte.set_is_visible(false);
        matte.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("MATTE", 340.0, 625.0, 165.0, fonts, TextShapeOptions { align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap());
        session.add_layer("Stripes", INK);
        session.draw_shape(RectangleShape::with_fill(40.0, 440.0, 600.0, 230.0, LinearGradientFill::from_colors(PointF::new(40.0, 0.0), PointF::new(640.0, 0.0), Color::from_rgb(255, 120, 80), Color::from_rgb(120, 120, 255)).unwrap().into(), 0.0).unwrap());
        let mut moving = DrawingGroup::new(Some(Color::from_rgba(255, 255, 255, 110)));
        for i in 0..30 {
            moving.add(LineShape::new(-200.0 + i as f32 * 40.0, 680.0, -60.0 + i as f32 * 40.0, 430.0, LineShapeOptions { stroke_thickness: 12.0, color: None }).unwrap());
        }
        let moving = session.draw_shape(moving);
        session.animations_mut().bind(moving, AnimatedProperty::Position, points(&[(0.0, 0.0, 0.0, L), (8.0, 160.0, 0.0, L)])).unwrap();
        session.get_layer_mut("Stripes").unwrap().set_mask(Some(DrawingMask::from_layer("Matte", false)));

        // The dissolve: its matte (a hidden layer) is replaced every frame.
        session.add_layer("Dissolve matte", Color::WHITE).set_is_visible(false);
        let dissolve = session.add_layer("Dissolve", INK);
        dissolve.add_shape(RectangleShape::with_fill(DISSOLVE_AREA.x, DISSOLVE_AREA.y, DISSOLVE_AREA.width, DISSOLVE_AREA.height, RadialGradientFill::from_colors(PointF::new(950.0, 560.0), 300.0, Color::from_rgb(255, 230, 140), Color::from_rgb(200, 60, 120)).unwrap().into(), 18.0).unwrap());
        dissolve.add_shape(codebrix_webgpu_animation::shapes::TextShape::new("DISSOLVE", 950.0, 585.0, 72.0, fonts, TextShapeOptions { color: Some(Color::from_rgb(40, 20, 60)), align: TextAlign::Center, tracking: 0.08, ..TextShapeOptions::default() }).unwrap());
        dissolve.set_mask(Some(DrawingMask::from_layer("Dissolve matte", false)));
        ClipMaskScene { session, noise: DissolveNoise::new(145, 60) }
    }
}

impl DemoScene for ClipMaskScene {
    fn name(&self) -> &'static str {
        "clip-mask"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.session.set_time(seconds);
        let angle = seconds * std::f64::consts::PI * 2.0 / 4.0;
        let light = PointF::new(1050.0 + (120.0 * angle.cos()) as f32, 260.0 + (70.0 * (angle * 2.0).sin()) as f32);
        self.session.get_layer_mut("Spotlight").unwrap().set_mask(Some(DrawingMask::inside(PathGeometry::circle(light, 85.0))));
        let sweep = ping_pong(seconds, 8.0);
        let threshold = sweep * sweep * 1.25 - 0.15;
        let matte = self.session.get_layer_mut("Dissolve matte").unwrap();
        matte.clear();
        matte.add_shape(ImageShape::new(self.noise.mask(threshold), DISSOLVE_AREA, ImageShapeOptions { sampling: ImageSampling::Linear, ..ImageShapeOptions::default() }).unwrap());
        self.session.render_with_background(scene, width, height);
    }
}
