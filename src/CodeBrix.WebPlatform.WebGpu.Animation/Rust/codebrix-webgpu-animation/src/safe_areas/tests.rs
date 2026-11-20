//! Host tests of the safe areas, ported from NoSkia's SafeAreaGuideTests, SafeAreaCheckTests and
//! SafeAreaTransformedCheckTests with their stated values, and of DrawingLayout::from_guide.

use crate::color::Color;
use crate::geometry::{PointF, RectangleF};
use crate::layout::{DrawingLayout, DrawingLayoutGuideOptions, LayoutReference};
use crate::models::{DrawingCamera, DrawingTransform, ElementId};
use crate::safe_areas::{SafeAreaCheckOptions, SafeAreaCrop, SafeAreaFindingKind, SafeAreaGuide, SafeAreaGuideOptions, SafeAreaGuides, SafeAreaMargins, SafeAreaTimeRule, SafeAreaZone};
use crate::shapes::{AnimatedShape, DrawingGroup, DrawingShape, RectangleShape, RectangleShapeOptions};
use crate::{DrawingSession, DrawingSessionOptions};

const PORTRAIT: (f32, f32) = (1080.0, 1920.0);
const LANDSCAPE: (f32, f32) = (1920.0, 1080.0);

fn zone<'a>(zones: &'a [SafeAreaZone], name: &str) -> &'a SafeAreaZone {
    let found: Vec<&SafeAreaZone> = zones.iter().filter(|z| z.zone_name() == name).collect();
    assert_eq!(found.len(), 1, "{name}");
    found[0]
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.01
}

fn presets() -> Vec<SafeAreaGuide> {
    vec![
        SafeAreaGuides::instagram_reels_ad(),
        SafeAreaGuides::instagram_stories_ad(),
        SafeAreaGuides::facebook_reels_ad(),
        SafeAreaGuides::facebook_stories_ad(),
        SafeAreaGuides::youtube_shorts_ad(),
        SafeAreaGuides::tiktok(),
        SafeAreaGuides::ordinary_portrait_post(),
        SafeAreaGuides::instagram_profile_grid(),
        SafeAreaGuides::instagram_feed_preview(),
        SafeAreaGuides::broadcast_title_safe(),
        SafeAreaGuides::broadcast_action_safe(),
        SafeAreaGuides::youtube_end_screen(),
    ]
}

#[test]
fn every_preset_is_named_sourced_and_marked_advisory_or_a_standard() {
    for guide in presets() {
        assert!(!guide.name().trim().is_empty());
        assert!(guide.source().starts_with("FACT") || guide.source().contains("Advisory"), "{}", guide.name());
        assert!(!guide.get_unsafe_zones(PORTRAIT, None, None).is_empty(), "{}", guide.name());
    }
}

#[test]
fn meta_ad_margins_lay_out_on_a_portrait_frame() {
    let zones = SafeAreaGuides::instagram_reels_ad().get_unsafe_zones(PORTRAIT, None, None);
    assert_eq!(zones.len(), 4);
    assert!(near(zone(&zones, "top margin").bounds().height, 268.8));
    assert!(near(zone(&zones, "bottom margin").bounds().y, 1248.0));
    assert!(near(zone(&zones, "left margin").bounds().width, 64.8));
    assert!(near(zone(&zones, "right margin").bounds().x, 1015.2));
    assert!(zones.iter().all(|z| z.guide_name() == "Instagram Reels ad"));
}

#[test]
fn youtube_shorts_ad_matches_the_reported_pixels() {
    let zones = SafeAreaGuides::youtube_shorts_ad().get_unsafe_zones(PORTRAIT, None, None);
    assert!(near(zone(&zones, "top margin").bounds().bottom(), 288.0));
    assert!(near(zone(&zones, "bottom margin").bounds().y, 1248.0));
    assert!(near(zone(&zones, "left margin").bounds().right(), 48.0));
    assert!(near(zone(&zones, "right margin").bounds().x, 888.0));
}

#[test]
fn instagram_crops_cut_away_the_top_and_bottom_of_a_portrait_frame_and_the_sides_of_a_landscape_one() {
    let grid = SafeAreaGuides::instagram_profile_grid().get_unsafe_zones(PORTRAIT, None, None);
    let feed = SafeAreaGuides::instagram_feed_preview().get_unsafe_zones(PORTRAIT, None, None);
    assert_eq!(grid.len(), 2);
    assert!(near(grid[0].bounds().height, 240.0) && near(grid[1].bounds().y, 1680.0));
    assert!(near(feed[0].bounds().height, 285.0) && near(feed[1].bounds().y, 1635.0));
    let sides = SafeAreaGuides::instagram_feed_preview().get_unsafe_zones(LANDSCAPE, None, None);
    assert_eq!(sides.len(), 2);
    assert!(near(sides[0].bounds().width, 528.0) && near(sides[1].bounds().x, 1392.0));
}

#[test]
fn broadcast_guides_use_the_smpte_margins() {
    let title = SafeAreaGuides::broadcast_title_safe().get_unsafe_zones(LANDSCAPE, None, None);
    let action = SafeAreaGuides::broadcast_action_safe().get_unsafe_zones(LANDSCAPE, None, None);
    assert!(near(zone(&title, "left margin").bounds().width, 96.0));
    assert!(near(zone(&title, "top margin").bounds().height, 54.0));
    assert!(near(zone(&action, "left margin").bounds().width, 67.2));
}

#[test]
fn end_screen_covers_the_whole_frame_only_in_the_final_twenty_seconds() {
    let guide = SafeAreaGuides::youtube_end_screen();
    assert!(guide.get_unsafe_zones(LANDSCAPE, Some(30.0), Some(60.0)).is_empty());
    let late = guide.get_unsafe_zones(LANDSCAPE, Some(41.0), Some(60.0));
    assert_eq!(late.len(), 1);
    assert_eq!(late[0].zone_name(), SafeAreaGuide::WHOLE_FRAME_ZONE_NAME);
    assert_eq!(late[0].bounds(), RectangleF::new(0.0, 0.0, 1920.0, 1080.0));
    assert_eq!(guide.get_unsafe_zones(LANDSCAPE, None, None).len(), 1);
    assert_eq!(guide.get_unsafe_zones(LANDSCAPE, Some(1.0), None).len(), 1);
}

#[test]
fn between_rule_applies_inside_its_window_only() {
    let guide = SafeAreaGuide::new("Lower third card", SafeAreaMargins::new(0.0, 0.3, 0.0, 0.0).unwrap(), SafeAreaGuideOptions { time_rule: Some(SafeAreaTimeRule::between(5.0, 8.0).unwrap()), ..SafeAreaGuideOptions::default() }).unwrap();
    let counts: Vec<usize> = [4.9, 5.0, 8.0, 8.1].iter().map(|t| guide.get_unsafe_zones(LANDSCAPE, Some(*t), None).len()).collect();
    assert_eq!(counts, vec![0, 1, 1, 0]);
}

#[test]
fn combine_keeps_every_part_zone_under_its_own_name_and_flattens() {
    let everywhere = SafeAreaGuide::combine("Everywhere", &[SafeAreaGuides::instagram_reels_ad(), SafeAreaGuides::tiktok(), SafeAreaGuides::youtube_end_screen()]).unwrap();
    let zones = everywhere.get_unsafe_zones(PORTRAIT, Some(1.0), Some(60.0));
    assert_eq!(everywhere.parts().len(), 3);
    assert_eq!(zones.len(), 8);
    let mut names: Vec<&str> = zones.iter().map(|z| z.guide_name()).collect();
    names.dedup();
    assert_eq!(names, vec!["Instagram Reels ad", "TikTok"]);
    assert!(everywhere.source().contains("TikTok: "));
    let inner = SafeAreaGuide::combine("Inner", &[SafeAreaGuides::tiktok(), SafeAreaGuides::youtube_shorts_ad()]).unwrap();
    assert_eq!(SafeAreaGuide::combine("Outer", &[inner, SafeAreaGuides::broadcast_title_safe()]).unwrap().parts().len(), 3);
}

#[test]
fn time_rules_report_their_window_and_its_duration() {
    let last = SafeAreaTimeRule::final_seconds(20.0).unwrap();
    let window = SafeAreaTimeRule::between(5.0, 8.0).unwrap();
    assert!(last.is_final_seconds());
    assert_eq!((last.duration(), last.start_time(), last.end_time()), (20.0, 0.0, 0.0));
    assert!(last.applies_at(Some(41.0), Some(60.0)));
    assert!(!last.applies_at(Some(39.0), Some(60.0)));
    assert!(!window.is_final_seconds());
    assert_eq!((window.start_time(), window.end_time(), window.duration()), (5.0, 8.0, 3.0));
    assert!(window.applies_at(None, None));
}

#[test]
fn custom_guides_and_margins_validate() {
    assert!(SafeAreaGuide::new(" ", SafeAreaMargins::NONE, SafeAreaGuideOptions::default()).is_err());
    assert!(SafeAreaMargins::new(0.6, 0.5, 0.0, 0.0).is_err());
    assert!(SafeAreaMargins::new(0.0, 0.0, -0.1, 0.0).is_err());
    assert!(SafeAreaCrop::new("x", 0.0, 1.0).is_err());
    assert!(SafeAreaGuide::combine("x", &[]).is_err());
    assert!(SafeAreaTimeRule::final_seconds(0.0).is_err());
}

#[test]
fn margins_max_covers_both_and_a_crop_of_the_frame_aspect_removes_nothing() {
    let max = SafeAreaMargins::max(SafeAreaMargins::new(0.1, 0.3, 0.05, 0.2).unwrap(), SafeAreaMargins::new(0.15, 0.2, 0.08, 0.1).unwrap());
    assert_eq!(max, SafeAreaMargins::new(0.15, 0.3, 0.08, 0.2).unwrap());
    assert!(SafeAreaMargins::NONE.is_empty());
    let same = SafeAreaGuide::new("Same", SafeAreaMargins::NONE, SafeAreaGuideOptions { crops: vec![SafeAreaCrop::new("16:9", 16.0, 9.0).unwrap()], ..SafeAreaGuideOptions::default() }).unwrap();
    assert!(same.get_unsafe_zones(LANDSCAPE, None, None).is_empty());
}

// ---- CheckSafeArea (SafeAreaCheckTests, SafeAreaTransformedCheckTests) ----

fn session(w: u32, h: u32, layer: &str, checked: bool) -> DrawingSession {
    let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (w, h), ..DrawingSessionOptions::default() });
    s.add_layer(layer, Color::WHITE).set_is_safe_area_checked(checked);
    s
}

fn filled_rect(s: &mut DrawingSession, x: f32, y: f32, w: f32, h: f32) -> ElementId {
    s.draw_rectangle(x, y, w, h, None, None, true, 0.0).unwrap()
}

#[test]
fn unmarked_elements_are_not_checked() {
    let mut s = session(1080, 1920, "Captions", false);
    filled_rect(&mut s, 0.0, 1800.0, 1080.0, 100.0);
    assert!(s.check_safe_area(&SafeAreaGuides::instagram_reels_ad(), None, SafeAreaCheckOptions::default()).is_empty());
}

#[test]
fn a_marked_element_in_the_bottom_band_is_reported_with_its_overlap() {
    let mut s = session(1080, 1920, "Captions", false);
    let caption = filled_rect(&mut s, 200.0, 1200.0, 600.0, 100.0);
    s.find_element_mut(caption).unwrap().set_is_safe_area_checked(true);
    let findings = s.check_safe_area(&SafeAreaGuides::instagram_reels_ad(), Some((1080, 1920)), SafeAreaCheckOptions::default());
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!((f.kind(), f.element(), f.layer_name(), f.element_index()), (SafeAreaFindingKind::Overlap, caption, "Captions", 0));
    assert_eq!((f.guide_name(), f.zone_name()), (Some("Instagram Reels ad"), Some("bottom margin")));
    let o = f.overlap().unwrap();
    assert!(near(o.y, 1248.0) && near(o.height, 52.0) && near(o.width, 600.0));
    assert!(f.to_string().starts_with("Captions #0 (RectangleShape): in Instagram Reels ad bottom margin"));
}

#[test]
fn a_marked_layer_checks_all_of_its_elements_and_reports_each_zone() {
    let mut s = session(1080, 1920, "Titles", true);
    filled_rect(&mut s, 20.0, 20.0, 100.0, 100.0);
    filled_rect(&mut s, 400.0, 800.0, 100.0, 100.0);
    let findings = s.check_safe_area(&SafeAreaGuides::instagram_reels_ad(), None, SafeAreaCheckOptions::default());
    assert_eq!(findings.iter().map(|f| f.zone_name().unwrap()).collect::<Vec<_>>(), vec!["top margin", "left margin"]);
    assert!(findings.iter().all(|f| f.element_index() == 0));
}

#[test]
fn unknown_bounds_are_skipped_by_default_and_listed_on_request() {
    let mut s = session(1080, 1920, "Custom", true);
    // Every Rust shape knows its bounds (C#'s test uses custom shapes that do not): bounds that
    // are not finite count as unknown; an animated shape that made nothing has an empty
    // rectangle, which overlaps nothing.
    let id = filled_rect(&mut s, 10.0, 10.0, 10.0, 10.0);
    let infinite = DrawingTransform::IDENTITY.with_scale_xy(f32::MAX, 1.0);
    s.find_element_mut(id).unwrap().set_transform(infinite);
    s.draw_shape(AnimatedShape::new(|_| None, 0.0));
    let quiet = s.check_safe_area(&SafeAreaGuides::tiktok(), None, SafeAreaCheckOptions::default());
    let listed = s.check_safe_area(&SafeAreaGuides::tiktok(), None, SafeAreaCheckOptions { report_unchecked_elements: true, ..SafeAreaCheckOptions::default() });
    assert!(quiet.is_empty());
    assert_eq!(listed.len(), 1);
    assert!(listed.iter().all(|f| f.kind() == SafeAreaFindingKind::CouldNotCheck && f.zone().is_none() && f.overlap().is_none()));
}

#[test]
fn check_all_elements_ignores_the_marks() {
    let mut s = session(1080, 1920, "Everything", false);
    s.draw_circle(540.0, 1850.0, 40.0, None, None, false).unwrap();
    let findings = s.check_safe_area(&SafeAreaGuides::ordinary_portrait_post(), None, SafeAreaCheckOptions { check_all_elements: true, ..SafeAreaCheckOptions::default() });
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].zone_name(), Some("bottom margin"));
}

#[test]
fn time_rules_are_evaluated_at_the_session_time() {
    let mut s = session(1920, 1080, "Logo", true);
    filled_rect(&mut s, 800.0, 400.0, 300.0, 200.0);
    let options = SafeAreaCheckOptions { duration: Some(60.0), ..SafeAreaCheckOptions::default() };
    let guide = SafeAreaGuides::youtube_end_screen();
    s.set_time(10.0);
    let early = s.check_safe_area(&guide, None, options).len();
    let unknown_duration = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    s.set_time(45.0);
    let late = s.check_safe_area(&guide, None, options).len();
    assert_eq!((early, unknown_duration, late), (0, 1, 1));
    let card = SafeAreaGuide::new("Mid-roll card", SafeAreaMargins::WHOLE_FRAME, SafeAreaGuideOptions { time_rule: Some(SafeAreaTimeRule::between(5.0, 10.0).unwrap()), ..SafeAreaGuideOptions::default() }).unwrap();
    let counts: Vec<usize> = [0.0, 7.0, 12.0]
        .iter()
        .map(|t| {
            s.set_time(*t);
            s.check_safe_area(&card, None, SafeAreaCheckOptions::default()).len()
        })
        .collect();
    assert_eq!(counts, vec![0, 1, 0]);
}

#[test]
fn a_letterboxed_frame_maps_the_drawing_into_its_centered_rectangle() {
    let mut s = session(1000, 1000, "Art", true);
    filled_rect(&mut s, 0.0, 900.0, 1000.0, 100.0);
    let findings = s.check_safe_area(&SafeAreaGuides::instagram_reels_ad(), Some((1080, 1920)), SafeAreaCheckOptions::default());
    let bottom: Vec<_> = findings.iter().filter(|f| f.zone_name() == Some("bottom margin")).collect();
    assert_eq!(bottom.len(), 1);
    let (frame, cal) = (bottom[0].frame_overlap().unwrap(), bottom[0].overlap().unwrap());
    assert!(near(frame.y, 1392.0) && near(frame.bottom(), 1500.0));
    assert!(near(cal.y, 900.0) && near(cal.height, 100.0));
}

fn centred_box() -> (DrawingSession, ElementId) {
    let mut s = session(1000, 1000, "Titles", true);
    let b = filled_rect(&mut s, 400.0, 400.0, 200.0, 200.0);
    (s, b)
}

#[test]
fn an_element_moved_into_a_margin_by_its_transform_is_reported() {
    let (mut s, b) = centred_box();
    let guide = SafeAreaGuides::broadcast_title_safe();
    let before = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default());
    s.find_element_mut(b).unwrap().set_transform(DrawingTransform::translate(0.0, -380.0));
    let after = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default());
    assert!(before.is_empty());
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].zone_name(), Some("top margin"));
    let o = after[0].overlap().unwrap();
    assert_eq!((o.x, o.y, o.width), (400.0, 20.0, 200.0));
    assert!((o.height - 30.0).abs() <= 1e-3);
}

#[test]
fn a_layer_transform_and_the_camera_count_too() {
    let (mut s, _) = centred_box();
    let guide = SafeAreaGuides::broadcast_title_safe();
    s.get_layer_mut("Titles").unwrap().set_transform(DrawingTransform::scale(5.0, PointF::new(500.0, 500.0)));
    let scaled = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    s.get_layer_mut("Titles").unwrap().set_transform(DrawingTransform::IDENTITY);
    let mut camera = DrawingCamera::new();
    camera.set_pan(PointF::new(-420.0, 0.0)).unwrap();
    s.set_camera(Some(camera));
    let panned = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default());
    s.get_layer_mut("Titles").unwrap().set_parallax(0.0).unwrap();
    let fixed = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    assert_eq!(scaled, 4);
    assert_eq!(panned.len(), 1);
    assert_eq!(panned[0].zone_name(), Some("right margin"));
    assert_eq!(fixed, 0);
}

#[test]
fn elements_outside_their_time_window_and_hidden_layers_are_not_checked() {
    let (mut s, b) = centred_box();
    let guide = SafeAreaGuides::broadcast_title_safe();
    let e = s.find_element_mut(b).unwrap();
    e.set_transform(DrawingTransform::translate(0.0, -380.0));
    e.set_time_window(Some(2.0), None);
    let early = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    s.set_time(2.0);
    let later = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    s.get_layer_mut("Titles").unwrap().set_is_visible(false);
    let hidden = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default()).len();
    assert_eq!((early, later, hidden), (0, 1, 0));
}

#[test]
fn a_group_is_measured_by_its_children_at_the_session_time() {
    let mut s = session(1000, 1000, "Titles", true);
    let filled = RectangleShapeOptions { is_filled: true, ..RectangleShapeOptions::default() };
    let mut group = DrawingGroup::new(None);
    group.add(RectangleShape::new(400.0, 400.0, 100.0, 100.0, filled).unwrap());
    group.add(DrawingShape::from(RectangleShape::new(400.0, 960.0, 100.0, 30.0, filled).unwrap()).with_time_window(None, Some(1.0)));
    s.draw_shape(group);
    let guide = SafeAreaGuides::broadcast_title_safe();
    let early = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default());
    s.set_time(1.0);
    let later = s.check_safe_area(&guide, None, SafeAreaCheckOptions::default());
    assert_eq!(early.len(), 1);
    assert_eq!(early[0].zone_name(), Some("bottom margin"));
    assert!(later.is_empty());
}

// ---- DrawingLayout from a guide (C#'s DrawingLayout(calibrationSize, guide, ...)) ----

#[test]
fn a_layout_from_a_guide_maps_its_safe_rectangle_into_calibrated_units() {
    // Portrait 1080 x 1920 drawn into its own frame: the Reels ad safe rectangle in pixels is
    // the calibrated one - x 64.8, y 268.8, 950.4 x 979.2 (C#'s arithmetic in f32).
    let layout = DrawingLayout::from_guide((1080, 1920), Some(&SafeAreaGuides::instagram_reels_ad()), DrawingLayoutGuideOptions::default()).unwrap();
    let r = layout.get_rectangle(LayoutReference::SafeArea);
    let (w, h) = (1080.0f32, 1920.0f32);
    let (left, top) = (0.06f32 * w, 0.14f32 * h);
    assert_eq!(r, RectangleF::new(left, top, (w - (0.06 * w)) - left, (h - (0.35 * h)) - top));
    // A square drawing letterboxed into the portrait frame: the safe rectangle comes back through
    // the aspect fit (drawing rect y 420, scale 1.08).
    let square = DrawingLayout::from_guide((1000, 1000), Some(&SafeAreaGuides::broadcast_title_safe()), DrawingLayoutGuideOptions { frame_size: Some((1080, 1920)), ..DrawingLayoutGuideOptions::default() }).unwrap();
    let s = square.safe_area();
    assert!(near(s.x, 50.0) && near(s.width, 900.0) && near(s.y, (96.0 - 420.0) / 1.08));
    assert_eq!(DrawingLayout::from_guide((10, 10), None, DrawingLayoutGuideOptions::default()).unwrap().safe_area(), RectangleF::new(0.0, 0.0, 10.0, 10.0));
    let gone = DrawingLayout::from_guide((10, 10), Some(&SafeAreaGuides::youtube_end_screen()), DrawingLayoutGuideOptions::default()).unwrap();
    assert!(gone.safe_area().is_empty());
}

// ---- SafeAreaOverlay: its settings and what the renderer fills ----

#[test]
fn the_overlay_plans_the_zones_at_its_alpha_and_a_ring_outside_the_safe_rectangle() {
    let mut overlay = crate::safe_areas::SafeAreaOverlay::new(SafeAreaGuides::broadcast_title_safe());
    overlay.set_opacity(0.5).unwrap();
    let plan = overlay.get_draw_plan(LANDSCAPE, 0.0);
    let (color, alpha, zones) = plan.mask.clone().unwrap();
    // 40 x 0.5 = 20; the zones are painted opaque into a layer at that alpha.
    assert_eq!((color, alpha, zones.len()), (Color::from_rgba(255, 64, 112, 255), 20, 4));
    let (_, ring_alpha, bands) = plan.outline.unwrap();
    assert_eq!(ring_alpha, 230, "the ring keeps its colour's alpha (the opacity is the mask's)");
    // Safe rectangle 96..1824 x 54..1026; the top band spans the corners, 2 px above it.
    assert_eq!(bands[0], RectangleF::new(94.0, 52.0, 1732.0, 2.0));
    assert_eq!(bands[3], RectangleF::new(1824.0, 54.0, 2.0, 972.0));
    assert!(overlay.set_opacity(1.5).is_err() && overlay.set_outline_width(-1.0).is_err());
    overlay.set_outline_width(0.0).unwrap();
    assert!(overlay.get_draw_plan(LANDSCAPE, 0.0).outline.is_none());
    let mut end = crate::safe_areas::SafeAreaOverlay::new(SafeAreaGuides::youtube_end_screen());
    end.duration = Some(60.0);
    assert!(end.get_draw_plan(LANDSCAPE, 30.0).mask.is_none(), "outside the final 20 seconds nothing is drawn");
    assert_eq!(end.get_draw_plan(LANDSCAPE, 50.0).mask.unwrap().2, vec![RectangleF::new(0.0, 0.0, 1920.0, 1080.0)]);
    assert!(end.get_draw_plan(LANDSCAPE, 50.0).outline.is_none(), "no safe rectangle, no ring");
}
