//! Host tests of captions, ported from CodeBrix.Imaging.Drawing's CaptionTrackTests and
//! CaptionPresenterTests (their texts and expected values; the presenter's C# tests use the
//! broadcast title-safe guide - a 5% margin - which is the SafeAreaGuide placement here), plus the
//! owner's default: captions against the frame with the style's margin.

use crate::captions::{CaptionCue, CaptionFormat, CaptionPlacement, CaptionPresenter, CaptionStyle, CaptionTrack};
use crate::geometry::RectangleF;
use crate::layout::{DrawingLayout, LayoutAnchor, LayoutPosition, LayoutPositionOptions, LayoutReference};
use crate::text::TextAlign;
use crate::Color;

const SRT: &str = "1\r\n00:00:01,000 --> 00:00:04,500\r\nHello <i>there</i>,\r\n{\\an8}friend &amp; neighbour\r\n\r\n2\r\n00:00:03,250 --> 00:00:06,000\r\n<font color=\"#ffcc00\">Second</font> cue\r\n";

const VTT: &str = "WEBVTT - a sample\n\nNOTE this block is a comment\nand spans two lines\n\nintro\n00:01.000 --> 00:02.500 align:start line:10%\n<v Narrator>Once upon</v> a <b>time</b>\n\n01:00:00.000 --> 01:00:01.000 position:25%\nAn hour in &lt;3&gt; &#65;&#x42;\n";

#[test]
fn parse_srt_reads_numbers_timings_and_cleans_the_text() {
    let track = CaptionTrack::parse_srt(SRT).unwrap();
    assert_eq!(track.count(), 2);
    assert_eq!(track.cues()[0].identifier(), Some("1"));
    assert_eq!(track.cues()[0].start_time(), 1.0);
    assert_eq!(track.cues()[0].end_time(), 4.5);
    assert_eq!(track.cues()[0].text(), "Hello there,\nfriend & neighbour");
    assert_eq!(track.cues()[1].start_time(), 3.25);
    assert_eq!(track.cues()[1].text(), "Second cue");
    assert_eq!(track.end_time(), 6.0);
}

#[test]
fn parse_web_vtt_skips_notes_and_applies_basic_settings() {
    let track = CaptionTrack::parse_web_vtt(VTT).unwrap();
    assert_eq!(track.count(), 2);
    let intro = &track.cues()[0];
    assert_eq!(intro.identifier(), Some("intro"));
    assert_eq!((intro.start_time(), intro.end_time()), (1.0, 2.5));
    assert_eq!(intro.text(), "Once upon a time");
    assert_eq!(intro.align(), Some(TextAlign::Left));
    assert_eq!(intro.settings(), Some("align:start line:10%"));
    let p = intro.position().unwrap();
    assert_eq!(p.anchor(), LayoutAnchor::new(0.5, 0.0).unwrap());
    assert_eq!(p.reference(), LayoutReference::Frame);
    assert!((p.fraction_offset().y - 0.1).abs() < 0.00001);
    let hour = &track.cues()[1];
    assert_eq!(hour.start_time(), 3600.0);
    assert_eq!(hour.identifier(), None);
    assert_eq!(hour.text(), "An hour in <3> AB");
    assert_eq!(hour.position().unwrap().reference(), LayoutReference::SafeArea);
    assert!((hour.position().unwrap().anchor().x() - 0.25).abs() < 0.00001);
}

#[test]
fn parse_tells_the_formats_apart() {
    assert_eq!(CaptionTrack::detect_format(VTT), CaptionFormat::WebVtt);
    assert_eq!(CaptionTrack::detect_format(SRT), CaptionFormat::Srt);
    assert_eq!(CaptionTrack::parse(&format!("\u{FEFF}{VTT}")).unwrap().count(), 2);
    assert_eq!(CaptionTrack::parse_as(SRT, CaptionFormat::Srt).unwrap().count(), 2);
}

#[test]
fn get_active_cues_uses_half_open_spans_in_start_order() {
    let track = CaptionTrack::parse_srt(SRT).unwrap();
    assert!(track.get_active_cues(0.999).is_empty());
    let both = track.get_active_cues(3.5);
    assert_eq!(both.len(), 2);
    assert_eq!(both[0].identifier(), Some("1"));
    let at_end = track.get_active_cues(4.5);
    assert_eq!(at_end.len(), 1);
    assert_eq!(at_end[0].identifier(), Some("2"));
    assert!(track.get_active_cues(6.0).is_empty());
    assert_eq!(track.get_active_cue_indices(3.5), vec![0, 1]);
}

#[test]
fn cues_built_in_code_are_sorted_by_start() {
    let track = CaptionTrack::new(vec![CaptionCue::new(5.0, 6.0, "late", None).unwrap(), CaptionCue::new(1.0, 2.0, "early", None).unwrap(), CaptionCue::new(1.0, 3.0, "early too", None).unwrap()]);
    let texts: Vec<&str> = track.cues().iter().map(|c| c.text()).collect();
    assert_eq!(texts, vec!["early", "early too", "late"]);
    assert_eq!(format!("{}", track.cues()[1]), "1 --> 3: early too");
}

#[test]
fn character_references_and_no_break_spaces_are_decoded() {
    let track = CaptionTrack::parse_web_vtt("WEBVTT\n\n00:00.000 --> 00:01.000\na&nbsp;b &quot;c&quot; &unknown; d\n").unwrap();
    assert_eq!(track.cues()[0].text(), "a\u{00A0}b \"c\" &unknown; d");
    // Non-ASCII text passes through untouched; a surrogate or zero reference is left as written.
    let track = CaptionTrack::parse_web_vtt("WEBVTT\n\n00:00.000 --> 00:01.000\nÜber <b>東京</b> &#xD800; &#0; &#x1F600;\n").unwrap();
    assert_eq!(track.cues()[0].text(), "Über 東京 &#xD800; &#0; \u{1F600}");
}

#[test]
fn parse_srt_rejects_malformed_timings_naming_the_line() {
    for text in ["1\n00:00:01,000 --> soon\nno end time\n", "1\n00:00:01,000 -> 00:00:02,000\nbad arrow\n", "1\n00:00:05,000 --> 00:00:02,000\nbackwards\n", "1\n00:61:00,000 --> 00:62:00,000\nbad minutes\n"] {
        let error = CaptionTrack::parse_srt(text).unwrap_err();
        assert!(error.starts_with("Line 2"), "{error}");
    }
}

#[test]
fn parse_web_vtt_requires_the_signature() {
    assert!(CaptionTrack::parse_web_vtt("00:01.000 --> 00:02.000\nhi\n").unwrap_err().starts_with("Line 1"));
    assert!(CaptionTrack::parse_web_vtt("WEBVTTX\n\n00:01.000 --> 00:02.000\nhi\n").is_err());
}

#[test]
fn a_cue_rejects_a_backwards_span() {
    assert!(CaptionCue::new(2.0, 1.0, "x", None).is_err());
    assert!(CaptionCue::new(f64::NAN, 1.0, "x", None).is_err());
}

#[test]
fn times_are_read_through_whole_milliseconds() {
    use crate::captions::caption_parser::try_parse_time;
    assert_eq!(try_parse_time("00:00:01,5"), Some(1.5));
    assert_eq!(try_parse_time("00:00:01.25"), Some(1.25));
    assert_eq!(try_parse_time("1:02:03.004"), Some(3723.004));
    assert_eq!(try_parse_time("02:03"), Some(123.0));
    assert_eq!(try_parse_time("02:60"), None);
    assert_eq!(try_parse_time("2:03.1234"), None);
    assert_eq!(try_parse_time("a:03"), None);
}

// ---- the presenter ----

const PRESENTER_SRT: &str = "1\n00:00:01,000 --> 00:00:03,000\nThe first caption, long enough that it has to wrap onto a second line here\n\n2\n00:00:02,000 --> 00:00:04,000\nOverlapping\n\n3\n00:00:05,000 --> 00:00:06,000\nLast\n";

/// The broadcast title-safe guide of a 640 x 360 frame: a 5% margin per edge.
const TITLE_SAFE: RectangleF = RectangleF { x: 32.0, y: 18.0, width: 576.0, height: 324.0 };

fn fonts() -> Option<crate::text::FontRegistry> {
    crate::text::layout_tests::fonts()
}

fn presenter(style: Option<CaptionStyle>, placement: CaptionPlacement) -> Option<CaptionPresenter> {
    let fonts = fonts()?;
    Some(CaptionPresenter::new(CaptionTrack::parse_srt(PRESENTER_SRT).unwrap(), &fonts, DrawingLayout::new((640, 360)).unwrap(), style).unwrap().with_placement(placement))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn between_cues_nothing_shows_and_the_same_placement_is_returned_while_cues_hold() {
    let Some(p) = presenter(None, CaptionPlacement::SafeAreaGuide(TITLE_SAFE)) else { return };
    let none = p.get_placed_captions_at(0.5).unwrap();
    let first = p.get_placed_captions_at(1.2).unwrap();
    let again = p.get_placed_captions_at(1.9).unwrap();
    let both = p.get_placed_captions_at(2.5).unwrap();
    assert!(none.is_empty());
    assert!(std::rc::Rc::ptr_eq(&first, &again));
    assert!(!std::rc::Rc::ptr_eq(&first, &both));
    assert_eq!(first.len(), 1);
    assert!(first[0].has_box());
    assert_eq!(first[0].create_text_shapes().unwrap().len(), 1);
    assert_eq!(both.len(), 2);
    // The same cue keeps the same laid-out text.
    assert!(std::rc::Rc::ptr_eq(&first[0].text, &both[0].text));
}

#[test]
fn opted_into_a_safe_area_guide_captions_sit_at_its_bottom_centre() {
    let Some(p) = presenter(None, CaptionPlacement::SafeAreaGuide(TITLE_SAFE)) else { return };
    let b = p.get_placed_captions_at(1.5).unwrap()[0].box_bounds;
    assert!(b.x >= TITLE_SAFE.x && b.right() <= TITLE_SAFE.right());
    assert!(close(b.bottom(), TITLE_SAFE.bottom()), "{b:?}");
    assert!(close((b.x + b.right()) / 2.0, (TITLE_SAFE.x + TITLE_SAFE.right()) / 2.0));
    assert_eq!(p.get_cue_layout(0).unwrap().line_count(), 2);
    // The text sits inside the box by the padding: size 4.5% of the short side (16.2).
    let placed = &p.get_placed_captions_at(1.5).unwrap()[0];
    let size = 360.0 * 0.045;
    assert!(close(placed.text_x + placed.text.content_bounds().x, b.x + 0.45 * size));
    assert!(close(placed.text_y + placed.text.content_bounds().y, b.y + 0.22 * size));
    assert!(close(placed.corner_radius, 0.25 * size));
    // The wrap width: 86% of the guide's width less the padding.
    assert_eq!(placed.text.width(), 576.0 * 0.86 - 2.0 * 0.45 * size);
}

#[test]
fn by_default_captions_sit_against_the_frame_kept_the_margin_from_its_edges() {
    let Some(p) = presenter(None, CaptionPlacement::Frame) else { return };
    let size = 360.0 * 0.045;
    let b = p.get_placed_captions_at(1.5).unwrap()[0].box_bounds;
    assert!(close(b.bottom(), 360.0 - size), "{b:?}");
    assert!(close((b.x + b.right()) / 2.0, 320.0));
    assert_eq!(p.get_cue_layout(0).unwrap().width(), (640.0 - 2.0 * size) * 0.86 - 2.0 * 0.45 * size);
    // A margin of 0 puts the box on the frame's bottom edge (what C# does with no guide).
    let Some(flush) = presenter(Some(CaptionStyle { margin: 0.0, ..CaptionStyle::default() }), CaptionPlacement::Frame) else { return };
    assert!(close(flush.get_placed_captions_at(1.5).unwrap()[0].box_bounds.bottom(), 360.0));
}

#[test]
fn overlapping_cues_stack_with_the_earliest_on_top() {
    let Some(p) = presenter(None, CaptionPlacement::SafeAreaGuide(TITLE_SAFE)) else { return };
    let both = p.get_placed_captions_at(2.5).unwrap();
    assert_eq!((both[0].cue_index, both[1].cue_index), (0, 1));
    assert!(both[0].box_bounds.bottom() < both[1].box_bounds.y);
    let gap = both[1].box_bounds.y - both[0].box_bounds.bottom();
    assert!(close(gap, 0.3 * 360.0 * 0.045));
    assert!(close(both[1].box_bounds.bottom(), TITLE_SAFE.bottom()));
}

#[test]
fn the_style_controls_size_box_and_outline() {
    let style = CaptionStyle { size: 30.0, background_color: None, outline_color: Some(Color::BLACK), max_lines: 1, ..CaptionStyle::default() };
    let Some(p) = presenter(Some(style), CaptionPlacement::SafeAreaGuide(TITLE_SAFE)) else { return };
    let placed = &p.get_placed_captions_at(1.5).unwrap()[0];
    assert!(!placed.has_box());
    let shapes = placed.create_text_shapes().unwrap();
    assert_eq!(shapes.len(), 2);
    assert!(shapes[0].is_stroked() && !shapes[0].is_filled());
    assert_eq!(shapes[0].stroke_style().unwrap().width(), 2.0 * 0.08 * 30.0);
    assert_eq!(shapes[0].color(), Some(Color::BLACK));
    assert!(shapes[1].is_filled());
    assert_eq!(shapes[1].color(), Some(Color::WHITE));
    assert_eq!(p.get_cue_layout(0).unwrap().line_count(), 1);
    assert!(p.get_cue_layout(0).unwrap().has_ellipsis());
    assert_eq!(p.style().size, 30.0);
}

#[test]
fn a_cue_position_places_it_by_itself() {
    let Some(fonts) = fonts() else { return };
    let top = LayoutPosition::new(LayoutAnchor::TOP_CENTER, LayoutPositionOptions { reference: LayoutReference::SafeArea, ..LayoutPositionOptions::default() }).unwrap();
    let track = CaptionTrack::new(vec![CaptionCue::new(0.0, 2.0, "Top", None).unwrap().with_position(top), CaptionCue::new(0.0, 2.0, "Bottom", None).unwrap()]);
    let p = CaptionPresenter::new(track.clone(), &fonts, DrawingLayout::new((640, 360)).unwrap(), None).unwrap().with_placement(CaptionPlacement::SafeAreaGuide(TITLE_SAFE));
    let placed = p.get_placed_captions_at(1.0).unwrap();
    assert!(close(placed[0].box_bounds.y, TITLE_SAFE.y));
    assert!(close(placed[1].box_bounds.bottom(), TITLE_SAFE.bottom()));
    // By default, the frame less the margin.
    let p = CaptionPresenter::new(track, &fonts, DrawingLayout::new((640, 360)).unwrap(), None).unwrap();
    assert!(close(p.get_placed_captions_at(1.0).unwrap()[0].box_bounds.y, 360.0 * 0.045));
}

#[test]
fn a_web_vtt_line_setting_is_measured_against_the_frame() {
    let Some(fonts) = fonts() else { return };
    let track = CaptionTrack::parse_web_vtt("WEBVTT\n\n00:00.000 --> 00:02.000 line:10% position:25%\nHi\n").unwrap();
    let p = CaptionPresenter::new(track, &fonts, DrawingLayout::new((640, 360)).unwrap(), None).unwrap();
    let placed = &p.get_placed_captions_at(1.0).unwrap()[0];
    // The box's top at 10% of the frame's height; its own anchor x = 0.25 at 25% of the width.
    assert!(close(placed.box_bounds.y, 36.0));
    assert!(close(placed.box_bounds.x + 0.25 * placed.box_bounds.width, 160.0));
}

#[test]
fn invalid_arguments_are_refused_with_what_to_do() {
    let Some(fonts) = fonts() else { return };
    let track = CaptionTrack::parse_srt(PRESENTER_SRT).unwrap();
    let error = CaptionPresenter::new(track.clone(), &fonts, DrawingLayout::new((640, 360)).unwrap(), Some(CaptionStyle { max_width_fraction: 0.0, ..CaptionStyle::default() })).unwrap_err();
    assert!(error.contains("max_width_fraction"));
    let p = CaptionPresenter::new(track, &fonts, DrawingLayout::new((640, 360)).unwrap(), Some(CaptionStyle { family_name: Some("Nope".into()), ..CaptionStyle::default() })).unwrap();
    assert!(p.get_placed_captions_at(1.5).unwrap_err().contains("'Nope'"));
    assert!(p.get_cue_layout(9).is_err());
}
