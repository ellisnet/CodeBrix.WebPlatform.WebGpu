//! "safe-areas" (AnimationDemoApp's SafeAreaScene): safe-area guides on a portrait 9:16 post and a
//! landscape 16:9 frame, each a session of its own drawn into a part of the demo frame with a
//! SafeAreaOverlay. The overlay cycles through the built-in presets (and one combination), and
//! each session's check_safe_area findings for the marked text are listed beside them. The
//! landscape side ends with the YouTube end-screen guide, which applies only in the final 20
//! seconds of a pretend 30-second video (the landscape session's time IS that video's clock). The
//! portrait overlay's opacity rises and falls (the tint scales; the edge line stays).

use codebrix_webgpu_animation::rendering::VectorScene;
use codebrix_webgpu_animation::safe_areas::{SafeAreaCheckOptions, SafeAreaFinding, SafeAreaGuide, SafeAreaGuides, SafeAreaOverlay};
use codebrix_webgpu_animation::shapes::{ShapeKind, TextShapeOptions};
use codebrix_webgpu_animation::text::{FontRegistry, TextAlign};
use codebrix_webgpu_animation::{Color, DrawingSession, SessionRenderOptions};

use crate::demo::kit::part;
use crate::demo::pictures::ping_pong;
use crate::demo::{create_frame_session, create_session, label, DemoScene, ACCENT, DIM, INK};

const TITLE: &str = "Safe-area guides";
const DESCRIPTION: &str = "SafeAreaGuides presets as an overlay (SafeAreaOverlay on a session) and CheckSafeArea findings for the marked text";
const PHONE: (f32, f32, f32, f32) = (56.0, 102.0, 342.0, 608.0);
const WIDE: (f32, f32, f32, f32) = (448.0, 102.0, 576.0, 324.0);
const PORTRAIT_SECONDS: f64 = 2.5;

pub struct SafeAreaScene {
    frame: DrawingSession,
    portrait: DrawingSession,
    landscape: DrawingSession,
    fonts: FontRegistry,
    portrait_guides: Vec<SafeAreaGuide>,
    landscape_guides: Vec<SafeAreaGuide>,
}

fn text(color: Color, align: TextAlign) -> TextShapeOptions {
    TextShapeOptions { color: Some(color), align, ..TextShapeOptions::default() }
}

fn truncate(text: &str) -> String {
    if text.chars().count() > 22 {
        format!("{}...", text.chars().take(21).collect::<String>())
    } else {
        text.to_string()
    }
}

impl SafeAreaScene {
    pub fn new(fonts: &FontRegistry) -> SafeAreaScene {
        let mut frame = create_frame_session(fonts, TITLE, DESCRIPTION);
        frame.add_layer("Notes", INK);
        let mut portrait = create_session(1080, 1920, Color::from_rgb(36, 28, 58), fonts);
        portrait.add_layer("Art", Color::from_rgb(120, 90, 200));
        portrait.draw_circle(540.0, 900.0, 330.0, Some(40.0), Some(Color::from_rgb(150, 110, 230)), false).unwrap();
        portrait.draw_circle(540.0, 900.0, 160.0, None, Some(Color::from_rgb(255, 190, 110)), true).unwrap();
        portrait.add_layer("Words", INK).set_is_safe_area_checked(true);
        portrait.draw_text("Venus Rosette", 540.0, 330.0, 96.0, text(INK, TextAlign::Center)).unwrap();
        portrait.draw_text("Eight years, thirteen Venus years", 540.0, 1420.0, 52.0, text(Color::from_rgb(200, 190, 230), TextAlign::Center)).unwrap();
        portrait.draw_text("Follow for more!", 80.0, 1780.0, 60.0, text(Color::from_rgb(255, 220, 120), TextAlign::Left)).unwrap();
        portrait.draw_circle(990.0, 1350.0, 44.0, None, Some(Color::from_rgb(255, 90, 120)), true).unwrap();
        let mut landscape = create_session(1920, 1080, Color::from_rgb(22, 40, 52), fonts);
        landscape.add_layer("Art", Color::from_rgb(60, 160, 170));
        landscape.draw_rectangle(0.0, 860.0, 1920.0, 220.0, None, Some(Color::from_rgb(30, 70, 80)), true, 0.0).unwrap();
        landscape.draw_circle(960.0, 480.0, 260.0, Some(30.0), Some(Color::from_rgb(60, 160, 170)), false).unwrap();
        landscape.add_layer("Captions", INK).set_is_safe_area_checked(true);
        landscape.draw_text("Channel title", 70.0, 120.0, 72.0, text(INK, TextAlign::Left)).unwrap();
        landscape.draw_rectangle(120.0, 880.0, 900.0, 120.0, None, Some(Color::from_rgba(255, 255, 255, 230)), true, 0.0).unwrap();
        landscape.draw_text("Lower third: a name and a role", 160.0, 960.0, 54.0, text(Color::from_rgb(20, 30, 40), TextAlign::Left)).unwrap();
        landscape.draw_text("LOGO", 1850.0, 100.0, 48.0, text(Color::from_rgb(255, 200, 90), TextAlign::Right)).unwrap();
        let portrait_guides = vec![
            SafeAreaGuides::instagram_reels_ad(),
            SafeAreaGuides::facebook_stories_ad(),
            SafeAreaGuides::youtube_shorts_ad(),
            SafeAreaGuides::tiktok(),
            SafeAreaGuides::ordinary_portrait_post(),
            SafeAreaGuides::instagram_profile_grid(),
            SafeAreaGuides::instagram_feed_preview(),
            SafeAreaGuide::combine("All portrait ad zones", &[SafeAreaGuides::instagram_reels_ad(), SafeAreaGuides::youtube_shorts_ad(), SafeAreaGuides::tiktok()]).unwrap(),
        ];
        let landscape_guides = vec![SafeAreaGuides::broadcast_title_safe(), SafeAreaGuides::broadcast_action_safe(), SafeAreaGuides::youtube_end_screen()];
        SafeAreaScene { frame, portrait, landscape, fonts: fonts.clone(), portrait_guides, landscape_guides }
    }

    fn describe(&mut self, guide: &SafeAreaGuide, findings: &[SafeAreaFinding], session: &DrawingSession, x: f32, y: f32) {
        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        label(notes, &fonts, guide.name(), x, y, 18.0, INK, TextAlign::Left);
        let kind = if !guide.parts().is_empty() { "combined".to_string() } else { guide.source().split([' ', ':']).next().unwrap_or("").to_string() };
        label(notes, &fonts, &format!("{kind} - {} finding(s) for the marked text", findings.len()), x, y + 22.0, 13.0, DIM, TextAlign::Left);
        for (i, f) in findings.iter().take(3).enumerate() {
            let name = match session.find_element(f.element()).map(|e| e.kind()) {
                Some(ShapeKind::Text(t)) => t.text().to_string(),
                Some(ShapeKind::Circle(_)) => "CircleShape".to_string(),
                Some(ShapeKind::Rectangle(_)) => "RectangleShape".to_string(),
                _ => "DrawingShape".to_string(),
            };
            let guide_part = match f.guide_name() {
                Some(g) if g != guide.name() => format!("{g}:"),
                _ => String::new(),
            };
            label(notes, &fonts, &format!("\"{}\" in {} {}", truncate(&name), guide_part, f.zone_name().unwrap_or("")), x + 10.0, y + 46.0 + i as f32 * 20.0, 13.0, ACCENT, TextAlign::Left);
        }
    }
}

impl DemoScene for SafeAreaScene {
    fn name(&self) -> &'static str {
        "safe-areas"
    }

    fn title(&self) -> &'static str {
        TITLE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn duration_seconds(&self) -> f64 {
        8.0 * PORTRAIT_SECONDS
    }

    fn render(&mut self, seconds: f64, scene: &mut VectorScene, width: u32, height: u32) {
        let portrait_guide = self.portrait_guides[((seconds / PORTRAIT_SECONDS) as usize).min(self.portrait_guides.len() - 1)].clone();
        let span = self.duration_seconds() / self.landscape_guides.len() as f64;
        let index = ((seconds / span) as usize).min(self.landscape_guides.len() - 1);
        let landscape_guide = self.landscape_guides[index].clone();
        // The end-screen guide plays against a pretend 30-second video: its clock runs 10 s to 30 s.
        let video_seconds = 10.0 + 20.0 * ((seconds - index as f64 * span) / span);
        self.landscape.set_time(video_seconds);
        self.portrait.set_time(seconds);
        let mut wide_overlay = SafeAreaOverlay::new(landscape_guide.clone());
        wide_overlay.duration = Some(30.0);
        self.landscape.set_safe_area_overlay(Some(wide_overlay));
        let tint = ping_pong(seconds, 5.0) as f32;
        let mut phone_overlay = SafeAreaOverlay::new(portrait_guide.clone());
        phone_overlay.set_opacity(tint).unwrap();
        self.portrait.set_safe_area_overlay(Some(phone_overlay));
        self.frame.get_layer_mut("Notes").unwrap().clear();
        let portrait_findings = self.portrait.check_safe_area(&portrait_guide, Some((1080, 1920)), SafeAreaCheckOptions::default());
        let portrait = std::mem::take(&mut self.portrait);
        self.describe(&portrait_guide, &portrait_findings, &portrait, WIDE.0, WIDE.1 + WIDE.3 + 40.0);
        self.portrait = portrait;
        let wide_findings = self.landscape.check_safe_area(&landscape_guide, Some((1920, 1080)), SafeAreaCheckOptions { duration: Some(30.0), ..SafeAreaCheckOptions::default() });
        let landscape = std::mem::take(&mut self.landscape);
        self.describe(&landscape_guide, &wide_findings, &landscape, WIDE.0, WIDE.1 + WIDE.3 + 170.0);
        self.landscape = landscape;
        let fonts = self.fonts.clone();
        let notes = self.frame.get_layer_mut("Notes").unwrap();
        let x = WIDE.0 + WIDE.2 + 20.0;
        if let Some(rule) = landscape_guide.time_rule() {
            let which = if rule.applies_at(Some(video_seconds), Some(30.0)) { "end screen" } else { "before it" };
            label(notes, &fonts, &format!("video at {video_seconds:.1} s of 30 ({which})"), x, WIDE.1 + 20.0, 13.0, ACCENT, TextAlign::Left);
        }
        label(notes, &fonts, &format!("portrait overlay Opacity = {tint:.2}"), x, WIDE.1 + 46.0, 13.0, INK, TextAlign::Left);
        label(notes, &fonts, "the tint scales; the edge line stays", x, WIDE.1 + 64.0, 13.0, DIM, TextAlign::Left);
        self.frame.set_time(seconds);
        self.frame.render_with_background(scene, width, height);
        let phone = part(&self.frame, PHONE.0, PHONE.1, PHONE.2, PHONE.3, width, height);
        self.portrait.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(phone), ..Default::default() });
        let wide = part(&self.frame, WIDE.0, WIDE.1, WIDE.2, WIDE.3, width, height);
        self.landscape.render_with(scene, width, height, &SessionRenderOptions { include_background: true, target: Some(wide), ..Default::default() });
    }
}
