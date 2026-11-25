//! "layout" (AnimationDemoApp's LayoutScene): ONE scene description built for two frame shapes -
//! portrait 9:16 with the Instagram Reels ad safe area, landscape 16:9 with the broadcast title-safe
//! area - placing everything with DrawingLayout: anchors of the safe area plus fractions, sizes from
//! the short side; a marker tours the nine anchors, one second each. As in C#, the layouts come from
//! the SafeAreaGuide presets (DrawingLayout::from_guide) and each session shows its guide's
//! SafeAreaOverlay.

use codebrix_webgpu_animation::layout::{DrawingLayout, DrawingLayoutGuideOptions, LayoutAnchor, LayoutPosition, LayoutPositionOptions, LayoutReference};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::safe_areas::{SafeAreaGuide, SafeAreaGuides, SafeAreaOverlay};
use codebrix_webgpu_animation::shapes::{AnimatedShape, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, SessionRenderOptions};

use crate::demo::kit::{disc, part};
use crate::demo::{create_frame_session, create_session, label, DemoScene, DIM, INK};

const TITLE: &str = "Layout anchors";
const DESCRIPTION: &str = "DrawingLayout, LayoutAnchor, LayoutPosition: place against the frame or a safe area's safe rectangle; one scene, two formats";
const TOUR: [LayoutAnchor; 9] = [LayoutAnchor::TOP_LEFT, LayoutAnchor::TOP_CENTER, LayoutAnchor::TOP_RIGHT, LayoutAnchor::CENTER_RIGHT, LayoutAnchor::BOTTOM_RIGHT, LayoutAnchor::BOTTOM_CENTER, LayoutAnchor::BOTTOM_LEFT, LayoutAnchor::CENTER_LEFT, LayoutAnchor::CENTER];

pub struct LayoutScene {
    frame: DrawingSession,
    portrait: DrawingSession,
    landscape: DrawingSession,
}

fn position(anchor: LayoutAnchor, options: LayoutPositionOptions) -> LayoutPosition {
    LayoutPosition::new(anchor, LayoutPositionOptions { reference: LayoutReference::SafeArea, ..options }).unwrap()
}

/// The one scene description, for any frame shape.
fn build(calibration_size: (u32, u32), guide: SafeAreaGuide, fonts: &FontRegistry) -> DrawingSession {
    let layout = DrawingLayout::from_guide(calibration_size, Some(&guide), DrawingLayoutGuideOptions::default()).unwrap();
    let (w, h) = layout.calibration_size();
    let mut session = create_session(w, h, Color::from_rgb(34, 30, 56), fonts);
    let unit = layout.short_side();
    session.add_layer("Art", Color::from_rgb(120, 90, 200));
    let centre = layout.place_at(LayoutAnchor::CENTER, LayoutReference::SafeArea, 0.0, 0.0);
    let ring = 0.25 * layout.safe_area().width.min(layout.safe_area().height);
    session.draw_circle(centre.x, centre.y, ring, Some(unit * 0.03), Some(Color::from_rgb(150, 110, 230)), false).unwrap();
    session.draw_circle(centre.x, centre.y, ring * 0.4, None, Some(Color::from_rgb(255, 190, 110)), true).unwrap();
    // The guide's overlay: its unsafe zones tinted, a thin line along the safe edge.
    session.set_safe_area_overlay(Some(SafeAreaOverlay::new(guide)));
    session.add_layer("Words", INK);
    let title = layout.place(&position(LayoutAnchor::TOP_CENTER, LayoutPositionOptions { fraction_y: 0.08, ..LayoutPositionOptions::default() }));
    session.draw_text("Layout anchors", title.x, title.y, unit * 0.06, TextShapeOptions { color: Some(INK), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap();
    let caption = layout.place(&position(LayoutAnchor::BOTTOM_CENTER, LayoutPositionOptions { fraction_y: -0.05, ..LayoutPositionOptions::default() }));
    session.draw_text("centred, 5% above the safe bottom", caption.x, caption.y, unit * 0.045, TextShapeOptions { color: Some(Color::from_rgb(200, 190, 230)), align: TextAlign::Center, ..TextShapeOptions::default() }).unwrap();
    let logo = layout.place_box((unit * 0.1, unit * 0.1), &position(LayoutAnchor::TOP_LEFT, LayoutPositionOptions { offset_x: unit * 0.02, offset_y: unit * 0.02, ..LayoutPositionOptions::default() }));
    session.draw_rectangle(logo.x, logo.y, logo.width, logo.height, None, Some(Color::from_rgb(90, 200, 255)), true, unit * 0.015).unwrap();
    let badge = layout.place_box((unit * 0.16, unit * 0.06), &position(LayoutAnchor::TOP_RIGHT, LayoutPositionOptions { offset_x: -unit * 0.02, offset_y: unit * 0.02, ..LayoutPositionOptions::default() }));
    session.draw_rectangle(badge.x, badge.y, badge.width, badge.height, None, Some(Color::from_rgb(255, 90, 120)), true, unit * 0.03).unwrap();
    // A marker touring the nine anchors of the safe area, one second each.
    session.draw_shape(AnimatedShape::new(
        move |t| {
            let stop = (t.floor() as usize) % TOUR.len();
            let at = layout.place_at(TOUR[stop], LayoutReference::SafeArea, 0.0, 0.0);
            Some(disc(at.x, at.y, unit * 0.035, Some(Color::from_rgb(255, 220, 120))).into())
        },
        0.0,
    ));
    session
}

impl LayoutScene {
    pub fn new(fonts: &FontRegistry) -> LayoutScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        let notes = frame.add_layer("Notes", DIM);
        label(notes, fonts, "portrait 9:16, Instagram Reels ad margins", 64.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "landscape 16:9, broadcast title safe", 440.0, 128.0, 13.0, DIM, TextAlign::Left);
        label(notes, fonts, "Both are the SAME code: positions are anchors of the safe area plus fractions; sizes come from the short side.", 64.0, 680.0, 15.0, INK, TextAlign::Left);
        let portrait = build((1080, 1920), SafeAreaGuides::instagram_reels_ad(), fonts);
        let landscape = build((1920, 1080), SafeAreaGuides::broadcast_title_safe(), fonts);
        LayoutScene { frame, portrait, landscape }
    }
}

impl DemoScene for LayoutScene {
    fn name(&self) -> &'static str {
        "layout"
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
        self.portrait.set_time(seconds);
        self.landscape.set_time(seconds);
        let p = part(&self.frame, 64.0, 140.0, 270.0, 480.0, width, height);
        let l = part(&self.frame, 440.0, 140.0, 784.0, 441.0, width, height);
        self.portrait.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(p), ..Default::default() });
        self.landscape.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(l), ..Default::default() });
    }
}
