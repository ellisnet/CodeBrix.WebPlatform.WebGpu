//! "captions" (AnimationDemoApp's CaptionScene): burned-in captions from an SRT file - a TEXT
//! RESOURCE OF THE BACKEND (fetched once with the generated `caption_scene_srt_bytes` call and
//! kept by `set_backend_captions`) - parsed by CaptionTrack::parse_srt (markup and character
//! references cleaned) and shown by the session's captions at its time: wrapped, on a rounded box,
//! two overlapping cues stacked. As in C#, this scene places them on the bottom edge of the
//! broadcast title-safe area (drawn dashed) - THE OPT-IN (CaptionPlacement::from_guide); the
//! crate's default is the frame, kept the style's margin from its edges. The "video" is everything
//! below the header (a session of its own); a small sun moves through it.

use std::cell::RefCell;

use codebrix_webgpu_animation::animation::EasingKind;
use codebrix_webgpu_animation::captions::{CaptionPlacement, CaptionPresenter, CaptionStyle, CaptionTrack};
use codebrix_webgpu_animation::fills::{LinearGradientFill, RadialGradientFill};
use codebrix_webgpu_animation::layout::{DrawingLayout, DrawingLayoutGuideOptions};
use codebrix_webgpu_animation::models::AnimatedProperty;
use codebrix_webgpu_animation::paths::{PathGeometry, PathStrokeCap, PathStrokeJoin, PathStrokeStyle, PathStrokeStyleOptions};
use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::safe_areas::SafeAreaGuides;
use codebrix_webgpu_animation::shapes::{CircleShape, PathShape, RectangleShape};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, PointF, SessionRenderOptions};

use crate::demo::kit::{fill_rect, points};
use crate::demo::{create_frame_session, create_session, label, DemoScene, BACKDROP, CONTENT_TOP, FRAME_HEIGHT, FRAME_WIDTH, INK};

const TITLE: &str = "Burned-in captions";
const DESCRIPTION: &str = "CaptionTrack.ParseSrt + DrawingSession.DrawCaptions: wrapped, boxed, placed on the safe area (opt-in), stacked when they overlap";

thread_local! {
    static BACKEND_CAPTIONS: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Keeps the SRT text the backend served, for the captions scene.
pub fn set_backend_captions(srt: String) {
    BACKEND_CAPTIONS.with(|c| *c.borrow_mut() = Some(srt));
}

fn backend_captions() -> String {
    BACKEND_CAPTIONS.with(|c| c.borrow().clone()).unwrap_or_default()
}

pub struct CaptionScene {
    header: DrawingSession,
    session: DrawingSession,
}

impl CaptionScene {
    pub fn new(fonts: &FontRegistry) -> CaptionScene {
        let header = create_frame_session(fonts, TITLE, DESCRIPTION);
        // The "video": everything below the header, with its own frame, safe area and captions.
        let frame = (FRAME_WIDTH, FRAME_HEIGHT - CONTENT_TOP as u32);
        let (fw, fh) = (frame.0 as f32, frame.1 as f32);
        let mut session = create_session(frame.0, frame.1, BACKDROP, fonts);
        let guide = SafeAreaGuides::broadcast_title_safe();
        let layout = DrawingLayout::from_guide(frame, Some(&guide), DrawingLayoutGuideOptions::default()).unwrap();
        let picture = session.add_layer("Picture", INK);
        let sky = LinearGradientFill::from_colors(PointF::new(0.0, 0.0), PointF::new(0.0, fh), Color::from_rgb(40, 60, 110), Color::from_rgb(200, 110, 90)).unwrap();
        picture.add_shape(RectangleShape::with_fill(0.0, 0.0, fw, fh, sky.into(), 0.0).unwrap());
        picture.add_shape(fill_rect(0.0, 470.0, fw, fh - 470.0, Some(Color::from_rgb(30, 34, 52)), 0.0));
        let glow = RadialGradientFill::from_colors(PointF::new(0.0, 0.0), 60.0, Color::from_rgb(255, 236, 170), Color::from_rgba(255, 200, 120, 0)).unwrap();
        let sun = picture.add_element(CircleShape::with_fill(0.0, 0.0, 60.0, glow.into()).unwrap());
        session.animations_mut().bind(sun, AnimatedProperty::Position, points(&[(0.0, 200.0, 330.0, EasingKind::Linear), (8.0, 1080.0, 150.0, EasingKind::SineInOut)])).unwrap();
        // The safe area the captions are placed in, dashed.
        let safe = layout.safe_area();
        let dashes = PathStrokeStyle::new(1.5, PathStrokeStyleOptions { cap: PathStrokeCap::Butt, join: PathStrokeJoin::Miter, miter_limit: 4.0, dash_pattern: Some(vec![8.0, 6.0]), dash_offset: 0.0, ..Default::default() }).unwrap();
        let guide_layer = session.add_layer("Guide", INK);
        guide_layer.add_shape(PathShape::with_style(PathGeometry::rectangle(safe), dashes, Some(Color::from_rgba(255, 255, 255, 120)), None, None));
        label(guide_layer, fonts, "BroadcastTitleSafe (opt-in placement; the default is the frame)", safe.x + 8.0, safe.y + 22.0, 13.0, Color::from_rgba(255, 255, 255, 160), TextAlign::Left);
        session.add_layer("Captions", INK);
        let track = CaptionTrack::parse_srt(&backend_captions()).unwrap_or_else(|_| CaptionTrack::parse_srt("").unwrap());
        let style = CaptionStyle { size: 32.0, ..CaptionStyle::default() };
        let placement = CaptionPlacement::from_guide(frame, &guide, DrawingLayoutGuideOptions::default()).unwrap();
        let presenter = CaptionPresenter::new(track, fonts, DrawingLayout::new(frame).unwrap(), Some(style)).unwrap().with_placement(placement);
        let shown = session.draw_captions_with(presenter);
        if let Some(e) = session.find_element_mut(shown) {
            e.set_is_safe_area_checked(true);
        }
        CaptionScene { header, session }
    }
}

impl DemoScene for CaptionScene {
    fn name(&self) -> &'static str {
        "captions"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        self.header.set_time(seconds);
        self.header.render_with_background(scene, width, height);
        self.session.set_time(seconds);
        let target = crate::demo::kit::part(&self.header, 0.0, CONTENT_TOP, FRAME_WIDTH as f32, FRAME_HEIGHT as f32 - CONTENT_TOP, width, height);
        self.session.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(target), ..Default::default() });
    }
}
