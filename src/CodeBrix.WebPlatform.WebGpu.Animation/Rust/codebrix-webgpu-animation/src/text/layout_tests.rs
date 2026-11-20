//! Host tests of the text layout engine, its line breaker and the text block's reveal - ported
//! from CodeBrix.Imaging.Drawing's TextLayoutEngineTests and TextBlockShapeTests (their
//! expected values where they state them: Roboto's ascender 1900, descender -500, no line gap,
//! 2048 units per em), plus checks against values computed from NoSkia's formulas with the
//! shaped advances of the same fonts, and the Rust-only rules (UTF-8 byte offsets with non-ASCII
//! text, the missing-character modes in a block, right-to-left runs placed right to left).
//!
//! The fonts are the test project's (see the text tests' header): CODEBRIX_ANIMATION_TEST_FONTS,
//! or a test that finds no font says so and passes without checking.

use std::path::PathBuf;
use std::rc::Rc;

use crate::paths::PathGeometry;
use crate::shapes::{TextBlockShape, TextBlockStrokeOptions};
use crate::text::font_face::TOFU_KEY;
use crate::text::text_layout_engine::{is_ideograph, is_no_break_space};
use crate::text::{FontRegistry, MeasureTextOptions, MissingCharacterMode, TextAlign, TextLayoutEngine, TextLayoutOptions, TextLayoutResult, TextRunDescriptor, TextTrimming, TextUnit, TextVerticalAlign};
use crate::Color;

const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog while the patient owl watches from the old oak tree.";

fn font_bytes(folder: &str, file: &str) -> Option<Vec<u8>> {
    let root = match std::env::var("CODEBRIX_ANIMATION_TEST_FONTS") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/Fonts"),
    };
    let path = root.join(folder).join(file);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(_) if std::env::var("CODEBRIX_ANIMATION_TEST_FONTS").is_ok() => panic!("CODEBRIX_ANIMATION_TEST_FONTS is set, but {} cannot be read", path.display()),
        Err(_) => {
            eprintln!("SKIPPED: no test font at {} (set CODEBRIX_ANIMATION_TEST_FONTS)", path.display());
            None
        }
    }
}

/// Roboto first, then Open Sans (C#'s CreateFonts).
pub(crate) fn fonts() -> Option<FontRegistry> {
    let fonts = FontRegistry::new();
    fonts.register_font(font_bytes("Roboto", "Roboto-Regular.ttf")?, None).unwrap();
    fonts.register_font(font_bytes("OpenSans", "OpenSans-Regular.ttf")?, None).unwrap();
    Some(fonts)
}

fn lay(fonts: &FontRegistry, text: &str, size: f32, options: TextLayoutOptions) -> Rc<TextLayoutResult> {
    TextLayoutEngine::layout(fonts, text, size, None, Some(&options)).unwrap()
}

fn width_of(fonts: &FontRegistry, text: &str, size: f32) -> f32 {
    fonts.measure_text(text, size, &MeasureTextOptions::default()).unwrap()
}

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
fn layout_wraps_to_the_width_without_starting_a_line_with_a_space() {
    let Some(fonts) = fonts() else { return };

    //Act
    let layout = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(420.0), ..TextLayoutOptions::default() });

    //Assert
    assert!(layout.line_count() > 2);
    assert_eq!(layout.width(), 420.0);
    for line in layout.lines() {
        assert!(line.width() <= 420.0, "{line}");
        assert_ne!(layout.text().as_bytes()[line.text_index()], b' ');
    }
    assert!(layout.lines().last().unwrap().ends_paragraph());
    assert!(!layout.lines()[0].ends_paragraph());
}

#[test]
fn greedy_wrapping_matches_noskias_rule_with_the_shaped_widths() {
    let Some(fonts) = fonts() else { return };

    //Act
    let layout = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), ..TextLayoutOptions::default() });

    //Assert - each line's width is the shaped width of its words (kerning across the run: the
    // line is part of one shaped paragraph, so compare within half a unit), and the next line's
    // first word would not have fitted after it.
    for (i, line) in layout.lines().iter().enumerate() {
        let text = layout.text()[line.text_index()..line.text_index() + line.text_length()].trim_end();
        assert!(close(line.width(), width_of(&fonts, text, 30.0), 0.5), "{line} '{text}'");
        if let Some(next) = layout.lines().get(i + 1) {
            let next_word = layout.text()[next.text_index()..].split(' ').next().unwrap();
            let joined = format!("{text} {next_word}");
            assert!(width_of(&fonts, &joined, 30.0) > 300.0, "'{joined}' would have fitted");
        }
    }
}

#[test]
fn lines_cover_the_whole_text_in_order() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), ..TextLayoutOptions::default() });
    let mut expected = 0;
    for line in layout.lines() {
        assert_eq!(line.text_index(), expected);
        expected += line.text_length();
    }
    assert_eq!(expected, PARAGRAPH.len());
    let all: String = layout.clusters().iter().map(|c| c.text()).collect();
    assert_eq!(all, PARAGRAPH);
}

#[test]
fn explicit_line_breaks_start_new_lines_and_a_trailing_break_leaves_an_empty_line() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "one\ntwo\r\nthree\n", 20.0, TextLayoutOptions::default());
    assert_eq!(layout.line_count(), 4);
    assert_eq!(layout.lines()[0].text_index(), 0);
    assert_eq!(layout.lines()[1].text_index(), 4);
    assert_eq!(layout.lines()[2].text_index(), 9);
    assert_eq!(layout.lines()[3].cluster_count(), 0);
    assert_eq!(layout.cluster_count(), 11);
    // NEL, LS and PS break too, and their UTF-8 lengths (2 and 3 bytes) are what the next line
    // starts after.
    let unicode = lay(&fonts, "a\u{0085}b\u{2028}c\u{2029}d", 20.0, TextLayoutOptions::default());
    let starts: Vec<usize> = unicode.lines().iter().map(|l| l.text_index()).collect();
    assert_eq!(starts, vec![0, 3, 7, 11]);
}

#[test]
fn line_spacing_and_explicit_line_height_set_the_line_pitch() {
    let Some(fonts) = fonts() else { return };

    //Act
    let natural = lay(&fonts, "a\nb", 100.0, TextLayoutOptions::default());
    let spaced = lay(&fonts, "a\nb", 100.0, TextLayoutOptions { line_spacing: 1.5, ..TextLayoutOptions::default() });
    let fixed = lay(&fonts, "a\nb", 100.0, TextLayoutOptions { line_height: 90.0, ..TextLayoutOptions::default() });

    //Assert - Roboto: ascender 1900, descender -500, no line gap, 2048 units per em
    assert!(close(natural.lines()[0].height(), 2400.0 / 2048.0 * 100.0, 0.001));
    assert!(close(natural.lines()[0].baseline_offset(), 1900.0 / 2048.0 * 100.0, 0.001));
    assert!(close(spaced.lines()[1].top(), natural.lines()[0].height() * 1.5, 0.001));
    assert_eq!(fixed.lines()[1].top(), 90.0);
    assert_eq!(fixed.height(), 180.0);
    // NoSkia's baseline formula with a fixed height: top + (height - (ascent + descent)) / 2 + ascent.
    let (a, d) = (1900.0 / 2048.0 * 100.0, 500.0 / 2048.0 * 100.0);
    assert!(close(fixed.lines()[0].baseline(), (90.0 - (a + d)) / 2.0 + a, 0.001));
}

#[test]
fn alignment_places_each_line_within_the_width() {
    let Some(fonts) = fonts() else { return };
    for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
        let layout = lay(&fonts, PARAGRAPH, 28.0, TextLayoutOptions { max_width: Some(500.0), align, ..TextLayoutOptions::default() });
        for line in layout.lines() {
            let expected = match align {
                TextAlign::Center => (500.0 - line.width()) / 2.0,
                TextAlign::Right => 500.0 - line.width(),
                _ => 0.0,
            };
            assert!(close(line.left(), expected, 0.0001));
            assert!(close(layout.clusters()[line.first_cluster()].origin().x, expected, 0.0001));
        }
    }
}

#[test]
fn justify_stretches_every_line_but_the_paragraph_end() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, PARAGRAPH, 28.0, TextLayoutOptions { max_width: Some(500.0), align: TextAlign::Justify, ..TextLayoutOptions::default() });
    for line in &layout.lines()[..layout.line_count() - 1] {
        assert_eq!(line.left(), 0.0);
        assert!(close(line.width(), 500.0, 0.01), "{line}");
    }
    assert!(layout.lines().last().unwrap().width() < 500.0);
}

#[test]
fn without_a_width_lines_align_to_the_widest() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "a long first line\nshort", 40.0, TextLayoutOptions { align: TextAlign::Center, ..TextLayoutOptions::default() });
    assert_eq!(layout.width(), layout.lines()[0].width());
    assert_eq!(layout.lines()[0].left(), 0.0);
    assert!(close(layout.lines()[1].left(), (layout.lines()[0].width() - layout.lines()[1].width()) / 2.0, 0.0001));
}

#[test]
fn vertical_alignment_places_the_lines_in_the_box() {
    let Some(fonts) = fonts() else { return };
    for align in [TextVerticalAlign::Top, TextVerticalAlign::Center, TextVerticalAlign::Bottom] {
        let layout = lay(&fonts, "one\ntwo", 50.0, TextLayoutOptions { max_height: Some(400.0), vertical_align: align, ..TextLayoutOptions::default() });
        let content = layout.content_bounds().height;
        let expected = match align {
            TextVerticalAlign::Center => (400.0 - content) / 2.0,
            TextVerticalAlign::Bottom => 400.0 - content,
            TextVerticalAlign::Top => 0.0,
        };
        assert_eq!(layout.height(), 400.0);
        assert!(close(layout.lines()[0].top(), expected, 0.0001));
    }
}

#[test]
fn character_ellipsis_cuts_the_last_kept_line_to_fit() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), max_lines: 2, trimming: TextTrimming::CharacterEllipsis, ..TextLayoutOptions::default() });
    assert_eq!(layout.line_count(), 2);
    assert!(layout.is_truncated());
    assert!(layout.has_ellipsis());
    assert_eq!(layout.clusters().last().unwrap().text(), "\u{2026}");
    assert!(layout.lines()[1].width() <= 300.0);
    assert!(layout.shown_text_length() < PARAGRAPH.len());
    assert_eq!(layout.clusters().last().unwrap().text_index(), layout.shown_text_length());
    // NoSkia's rule: the kept characters plus the ellipsis fit, one more character would not.
    let line = &layout.lines()[1];
    let kept = &PARAGRAPH[line.text_index()..layout.shown_text_length()];
    assert!(width_of(&fonts, &format!("{kept}\u{2026}"), 30.0) <= 300.0 + 0.5);
}

#[test]
fn word_ellipsis_cuts_after_a_whole_word() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), max_lines: 2, trimming: TextTrimming::WordEllipsis, ellipsis: "...".to_string(), ..TextLayoutOptions::default() });
    assert!(layout.has_ellipsis());
    assert_eq!(layout.clusters().last().unwrap().text(), "...");
    assert_eq!(PARAGRAPH.as_bytes()[layout.shown_text_length()], b' ');
    assert!(layout.lines()[1].width() <= 300.0);
}

#[test]
fn without_trimming_extra_lines_are_dropped_silently_and_max_height_limits_lines() {
    let Some(fonts) = fonts() else { return };
    let line_height = 2400.0 / 2048.0 * 30.0;
    let by_lines = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), max_lines: 1, ..TextLayoutOptions::default() });
    let by_height = lay(&fonts, PARAGRAPH, 30.0, TextLayoutOptions { max_width: Some(300.0), max_height: Some(line_height * 2.5), ..TextLayoutOptions::default() });
    assert_eq!(by_lines.line_count(), 1);
    assert!(by_lines.is_truncated());
    assert!(!by_lines.has_ellipsis());
    assert_eq!(by_height.line_count(), 2);
    assert!(by_height.is_truncated());
}

#[test]
fn words_and_lines_are_reported_as_pieces() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Hello brave new\nworld", 40.0, TextLayoutOptions::default());
    let words = layout.get_pieces(TextUnit::Word);
    let lines = layout.get_pieces(TextUnit::Line);
    let characters = layout.get_pieces(TextUnit::Character);
    assert_eq!(layout.word_count(), 4);
    assert_eq!(words.len(), 4);
    assert_eq!(words[1].text(), "brave");
    assert_eq!(words[1].text_index(), 6);
    assert_eq!(words[3].line_index(), 1);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].text(), "world");
    assert_eq!(characters.len(), layout.cluster_count());
    assert!(characters[5].is_white_space());
    assert!(!words[0].get_outline().is_empty());
    assert!(close(words[0].center().x, words[0].bounds().x + words[0].bounds().width / 2.0, 0.0001));
}

#[test]
fn clusters_carry_outlines_boxes_and_lookups() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Ab c", 60.0, TextLayoutOptions::default());
    let c = layout.clusters();
    assert!(c[0].has_ink());
    assert!(c[2].is_white_space());
    assert!(!c[2].has_ink());
    assert_eq!(c[2].word_index(), None);
    assert_eq!(c[3].word_index(), Some(1));
    assert_eq!(c[1].bounds().x, c[1].origin().x);
    assert_eq!(c[1].bounds().height, layout.lines()[0].height());
    assert_eq!(layout.get_rect_for_index(1).unwrap(), c[1].bounds());
    assert_eq!(layout.get_rect_for_index(4).unwrap().width, 0.0);
    assert_eq!(layout.get_line_index_at(3), Some(0));
    assert!(layout.get_rect_for_index(5).is_err());
    let outline = layout.get_outline().get_bounds();
    let ink = layout.ink_bounds().unwrap();
    assert!(outline.x >= ink.x - 0.001 && outline.right() <= ink.right() + 0.001);
}

#[test]
fn runs_mix_sizes_and_families_on_one_line() {
    let Some(fonts) = fonts() else { return };
    let runs = [TextRunDescriptor::new("Big ", 80.0, Some("Roboto")).unwrap(), TextRunDescriptor::new("small", 20.0, Some("Open Sans")).unwrap().with_color(Color::from_rgb(255, 0, 0))];
    let layout = TextLayoutEngine::layout_runs(&fonts, &runs, None).unwrap();
    assert_eq!(layout.text(), "Big small");
    assert_eq!(layout.line_count(), 1);
    assert!(close(layout.lines()[0].ascent(), 1900.0 / 2048.0 * 80.0, 0.001));
    assert_eq!(layout.clusters()[4].run_index(), 1);
    assert_eq!(layout.clusters()[0].origin().y, layout.clusters()[4].origin().y);
    assert_eq!(layout.runs()[1].color(), Some(Color::from_rgb(255, 0, 0)));
    // The second run starts where the first's advance (and tracking) ends.
    let first_run = width_of(&fonts, "Big ", 80.0);
    assert!(close(layout.clusters()[4].origin().x, first_run, 0.01));
}

#[test]
fn tracking_adds_its_space_after_every_cluster_but_the_last() {
    let Some(fonts) = fonts() else { return };
    let plain = TextLayoutEngine::layout_runs(&fonts, &[TextRunDescriptor::new("abcd", 40.0, None).unwrap()], None).unwrap();
    let tracked = TextLayoutEngine::layout_runs(&fonts, &[TextRunDescriptor::new("abcd", 40.0, None).unwrap().with_tracking(0.25).unwrap()], None).unwrap();
    assert!(close(tracked.lines()[0].width(), plain.lines()[0].width() + 3.0 * 10.0, 0.001));
    assert!(close(tracked.clusters()[2].origin().x, plain.clusters()[2].origin().x + 20.0, 0.001));
}

#[test]
fn empty_text_lays_out_one_empty_line() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "", 30.0, TextLayoutOptions::default());
    assert_eq!(layout.line_count(), 1);
    assert_eq!(layout.cluster_count(), 0);
    assert_eq!(layout.ink_bounds(), None);
    assert!(layout.get_outline().is_empty());
    assert!(layout.height() > 0.0);
}

#[test]
fn layout_is_deterministic() {
    let Some(fonts) = fonts() else { return };
    let options = TextLayoutOptions { max_width: Some(333.0), align: TextAlign::Justify, line_spacing: 1.2, ..TextLayoutOptions::default() };
    let first = lay(&fonts, PARAGRAPH, 31.0, options.clone()).get_outline();
    let second = lay(&fonts, PARAGRAPH, 31.0, options).get_outline();
    assert_eq!(first.bez_path().elements(), second.bez_path().elements());
}

#[test]
fn text_along_a_straight_path_lands_on_its_baseline() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Path", 40.0, TextLayoutOptions::default());
    let baseline = layout.lines()[0].baseline();
    let along = layout.get_outline_along_path(&PathGeometry::parse("M100 300 L700 300").unwrap(), 20.0, 0.0).unwrap();
    let off = layout.get_outline_along_path(&PathGeometry::parse("M100 300 L110 300").unwrap(), 20.0, 0.0).unwrap();
    let moved = along.get_bounds();
    let original = layout.get_outline().get_bounds();
    assert!(close(moved.x, original.x + 120.0, 0.01), "{moved:?} {original:?}");
    assert!(close(moved.y, original.y + 300.0 - baseline, 0.01));
    assert!(off.is_empty());
    // A vertical offset moves the baseline below the path.
    let below = layout.get_outline_along_path(&PathGeometry::parse("M100 300 L700 300").unwrap(), 20.0, 15.0).unwrap().get_bounds();
    assert!(close(below.y, moved.y + 15.0, 0.01));
}

#[test]
fn text_along_a_vertical_path_turns_with_it() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "IIII", 40.0, TextLayoutOptions::default());
    let glyphs = layout.get_cluster_outlines_along_path(&PathGeometry::parse("M200 0 L200 800").unwrap(), 0.0, 0.0).unwrap();
    let first = glyphs[0].get_bounds();
    assert!(first.x >= 199.9, "{first:?}");
    assert!(first.height < first.width);
    assert!(glyphs[1].get_bounds().y > first.y);
}

#[test]
fn text_around_a_closed_path_wraps_round() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Round", 40.0, TextLayoutOptions::default());
    let circle = PathGeometry::circle(crate::PointF::new(0.0, 0.0), 10.0);
    let transforms = layout.get_cluster_transforms_along_path(&circle, 1000.0, 0.0).unwrap();
    assert!(transforms.iter().all(|t| t.is_some()));
    assert!(layout.get_cluster_transforms_along_path(&circle, f32::NAN, 0.0).is_err());
}

#[test]
fn invalid_arguments_are_refused_with_what_to_do() {
    let Some(fonts) = fonts() else { return };
    assert!(TextLayoutEngine::layout_runs(&fonts, &[], None).unwrap_err().contains("At least one run"));
    assert!(TextLayoutEngine::layout(&fonts, "a", 10.0, None, Some(&TextLayoutOptions { max_width: Some(0.0), ..TextLayoutOptions::default() })).unwrap_err().contains("max_width"));
    assert!(TextLayoutEngine::layout(&fonts, "a", 10.0, None, Some(&TextLayoutOptions { line_spacing: f32::NAN, ..TextLayoutOptions::default() })).unwrap_err().contains("line_spacing"));
    assert!(TextLayoutEngine::layout(&fonts, "a", 10.0, Some("Nope"), None).unwrap_err().contains("'Nope'"));
    assert!(TextRunDescriptor::new("a", 0.0, None).is_err());
    assert!(TextLayoutEngine::layout(&FontRegistry::new(), "a", 10.0, None, None).unwrap_err().starts_with("No font is registered"));
}

#[test]
fn a_word_wider_than_the_line_is_broken_between_characters() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Supercalifragilistic", 40.0, TextLayoutOptions { max_width: Some(150.0), ..TextLayoutOptions::default() });
    assert!(layout.line_count() > 1);
    for line in layout.lines() {
        assert!(line.width() <= 150.0);
    }
}

#[test]
fn the_line_breaker_breaks_after_hyphens_inside_words_zero_width_spaces_and_around_ideographs() {
    let Some(fonts) = fonts() else { return };
    // Every break opportunity is used when the next word does not fit: the hyphen inside
    // "well-known" is one; a no-break space is none.
    let wide = |text: &str, width: f32| -> Vec<String> {
        let layout = lay(&fonts, text, 40.0, TextLayoutOptions { max_width: Some(width), ..TextLayoutOptions::default() });
        layout.lines().iter().map(|l| layout.text()[l.text_index()..l.text_index() + l.text_length()].to_string()).collect()
    };
    let w = width_of(&fonts, "well-", 40.0).max(width_of(&fonts, "known", 40.0)) + 1.0;
    assert_eq!(wide("well-known", w), vec!["well-", "known"]);
    // U+2010 HYPHEN (3 bytes) works the same, and byte offsets stay on character boundaries.
    let w = width_of(&fonts, "well\u{2010}", 40.0).max(width_of(&fonts, "known", 40.0)) + 1.0;
    assert_eq!(wide("well\u{2010}known", w), vec!["well\u{2010}", "known"]);
    // After a zero width space.
    let w = width_of(&fonts, "abc", 40.0) + 1.0;
    assert_eq!(wide("abc\u{200B}def", w), vec!["abc\u{200B}", "def"]);
    // A no-break space is no opportunity: "ab\u{A0}cd" breaks between characters instead.
    let lines = wide("ab\u{00A0}cd ef", width_of(&fonts, "ab\u{00A0}c", 40.0) + 1.0);
    assert_eq!(lines[0], "ab\u{00A0}c");
    assert!(is_no_break_space('\u{00A0}') && is_no_break_space('\u{202F}') && !is_no_break_space(' '));
    // Before or after an ideograph (Roboto lacks them: each is a tofu box of .notdef's advance).
    let w = width_of(&fonts, "ab\u{65E5}", 40.0) + 1.0;
    assert_eq!(wide("ab\u{65E5}\u{672C}cd", w), vec!["ab\u{65E5}", "\u{672C}cd"]);
    assert!(is_ideograph(0x65E5) && is_ideograph(0x3042) && is_ideograph(0xAC00) && !is_ideograph(0x41));
}

#[test]
fn text_positions_are_utf8_byte_offsets() {
    let Some(fonts) = fonts() else { return };
    let text = "Größe über 東京";
    let layout = lay(&fonts, text, 30.0, TextLayoutOptions::default());
    for c in layout.clusters() {
        assert_eq!(&text[c.text_index()..c.text_index() + c.text_length()], c.text());
    }
    let u = text.find('ü').unwrap();
    let cluster = layout.get_cluster_index_at(u).unwrap();
    assert_eq!(layout.clusters()[cluster].text(), "ü");
    assert_eq!(layout.get_cluster_index_at(u + 1), Some(cluster));
    let words = layout.get_pieces(TextUnit::Word);
    assert_eq!(words[1].text(), "über");
    assert_eq!(words[1].text_index(), u);
    assert_eq!(words[1].text_length(), "über".len());
    // The two ideographs are missing in Roboto and reported at their byte offsets.
    let missing: Vec<usize> = layout.missing_characters().iter().map(|m| m.text_index).collect();
    assert_eq!(missing, vec![text.find('東').unwrap(), text.find('京').unwrap()]);
    assert!(layout.has_missing_characters());
    assert!(layout.clusters().iter().filter(|c| c.is_missing()).count() == 2);
}

#[test]
fn missing_characters_in_a_block_follow_the_three_modes() {
    let Some(fonts) = fonts() else { return };
    let keys = |family: &str, mode: MissingCharacterMode| -> (bool, Vec<u32>) {
        let options = TextLayoutOptions { missing_characters: mode, ..TextLayoutOptions::default() };
        let layout = TextLayoutEngine::layout(&fonts, "a\u{65E5}b", 30.0, Some(family), Some(&options)).unwrap();
        let c = &layout.clusters()[1];
        (c.has_ink(), c.glyphs.iter().map(|g| g.key).collect())
    };
    // Roboto's .notdef has ink (a box); Open Sans's has none.
    assert_eq!(keys("Roboto", MissingCharacterMode::ShowTofu), (true, vec![0]));
    assert_eq!(keys("Roboto", MissingCharacterMode::FontDefault), (true, vec![0]));
    assert_eq!(keys("Roboto", MissingCharacterMode::Blank), (false, vec![]));
    assert_eq!(keys("Open Sans", MissingCharacterMode::ShowTofu), (true, vec![TOFU_KEY]));
    assert_eq!(keys("Open Sans", MissingCharacterMode::FontDefault), (false, vec![]));
    assert_eq!(keys("Open Sans", MissingCharacterMode::Blank), (false, vec![]));
    // The advance is the same in every mode (the line never collapses).
    let widths: Vec<f32> = [MissingCharacterMode::ShowTofu, MissingCharacterMode::Blank].iter().map(|m| lay(&fonts, "a\u{65E5}b", 30.0, TextLayoutOptions { missing_characters: *m, ..TextLayoutOptions::default() }).lines()[0].width()).collect();
    assert_eq!(widths[0], widths[1]);
}

#[test]
fn a_right_to_left_run_is_placed_right_to_left_in_text_order() {
    let Some(fonts) = fonts() else { return };
    // Open Sans has Hebrew. Clusters stay in text order; their x runs right to left.
    let text = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
    let layout = TextLayoutEngine::layout(&fonts, text, 40.0, Some("Open Sans"), None).unwrap();
    let c = layout.clusters();
    assert_eq!(c.len(), 4);
    assert!(c.windows(2).all(|w| w[0].text_index() < w[1].text_index()));
    assert!(c.windows(2).all(|w| w[0].origin().x > w[1].origin().x), "{c:?}");
    assert!(close(c[3].origin().x, 0.0, 0.001));
    assert!(close(c[0].origin().x + c[0].advance(), layout.lines()[0].width(), 0.01));
}

#[test]
fn a_text_block_reveals_units_by_fraction_count_and_softness() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Hello brave new world", 40.0, TextLayoutOptions::default());
    let mut block = TextBlockShape::new(layout.clone(), 10.0, 20.0, None).unwrap();
    assert!(block.is_fully_revealed());
    assert_eq!(block.reveal_fraction(), 1.0);
    assert_eq!(block.get_unit_count(), 21);
    assert_eq!(block.get_revealed_clusters().len(), 18);

    // Words: 4 units; a count of 2 shows the first two words.
    block.set_reveal_unit(TextUnit::Word);
    assert_eq!(block.get_unit_count(), 4);
    block.set_reveal_count(2.0).unwrap();
    let shown: String = block.get_revealed_clusters().iter().map(|s| layout.clusters()[s.cluster].text().to_string()).collect();
    assert_eq!(shown, "Hellobrave");
    assert!(!block.is_fully_revealed());
    assert_eq!(block.reveal_fraction(), 0.5);

    // Softness 2 at count 2.5: NoSkia's AlphaOf = clamp((reveal - (unit + 1)) / softness + 1).
    block.set_reveal_softness(2.0).unwrap();
    block.set_reveal_count(2.5).unwrap();
    let alphas: Vec<f32> = block.get_revealed_clusters().iter().map(|s| s.alpha).collect();
    let expect = |unit: f32| (((2.5 - (unit + 1.0)) / 2.0) + 1.0).clamp(0.0, 1.0);
    assert_eq!(alphas[0], expect(0.0));
    assert_eq!(alphas[5], expect(1.0));
    assert_eq!(alphas[9], expect(1.0));
    assert_eq!(alphas[10], expect(2.0));
    assert_eq!(*alphas.last().unwrap(), expect(3.0));
    assert_eq!(alphas.len(), 18);

    // Fractions, and what a binding does with a track's value.
    block.set_reveal_fraction(0.25).unwrap();
    assert_eq!(block.reveal_count(), 1.0);
    assert!(block.set_reveal_fraction(1.5).is_err() && block.set_reveal_count(-1.0).is_err() && block.set_reveal_softness(f32::NAN).is_err());
    // With the soft edge, a count of 0 still fades the first unit in by half; without it, nothing.
    block.apply_reveal_fraction(f32::NAN);
    assert_eq!(block.get_revealed_clusters()[0].alpha, 0.5);
    block.set_reveal_softness(0.0).unwrap();
    assert_eq!(block.reveal_count(), 0.0);
    assert!(block.get_revealed_clusters().is_empty());
    assert_eq!(block.get_bounds(), crate::RectangleF::new(10.0, 20.0, 0.0, 0.0));
    block.apply_reveal_fraction(7.0);
    assert!(block.is_fully_revealed());
    block.apply_reveal_count(-3.0);
    assert_eq!(block.reveal_count(), 0.0);
}

#[test]
fn a_text_block_splits_into_pieces_and_path_pieces() {
    let Some(fonts) = fonts() else { return };
    let layout = lay(&fonts, "Hello brave\nworld", 40.0, TextLayoutOptions::default());
    let block = TextBlockShape::with_stroke(layout.clone(), 5.0, 7.0, Some(crate::paths::PathStrokeStyle::new(2.0, Default::default()).unwrap()), TextBlockStrokeOptions { fill_color: Some(Color::WHITE), ..TextBlockStrokeOptions::default() }).unwrap();
    assert!(block.is_stroked() && block.is_filled());
    assert!(TextBlockShape::with_stroke(layout.clone(), 0.0, 0.0, None, TextBlockStrokeOptions::default()).is_err());
    let words = block.create_pieces(TextUnit::Word);
    assert_eq!(words.len(), 3);
    assert_eq!((words[1].first_cluster(), words[1].cluster_count()), (6, 5));
    assert!(words[1].is_stroked());
    let characters = block.create_pieces(TextUnit::Character);
    assert_eq!(characters.len(), layout.cluster_count());
    let path = block.create_path_pieces(TextUnit::Character);
    assert_eq!(path.len(), layout.cluster_count() - 1);
    assert!(close(path.last().unwrap().baseline, 7.0 + layout.lines()[1].baseline(), 0.0001));
    // A piece of a piece stays inside its range; the bounds are the ink widened by the stroke.
    let inner = words[1].create_pieces(TextUnit::Character);
    assert_eq!(inner.len(), 5);
    let b = words[1].get_bounds();
    let ink = layout.get_pieces(TextUnit::Word)[1].ink_bounds().unwrap();
    assert!(close(b.x, ink.x + 5.0 - 1.0, 0.001) && close(b.width, ink.width + 2.0, 0.001));
    assert_eq!(block.box_bounds(), crate::RectangleF::new(5.0, 7.0, layout.width(), layout.height()));
}

#[test]
fn ligatures_marks_and_format_characters_make_the_clusters_noskia_counts() {
    let Some(fonts) = fonts() else { return };
    // Roboto joins "ffi" into one glyph: one cluster of three bytes (C#'s heuristic folds the
    // characters a ligature covers into one cluster too).
    let office = TextLayoutEngine::layout(&fonts, "office", 40.0, Some("Roboto"), None).unwrap();
    let texts: Vec<&str> = office.clusters().iter().map(|c| c.text()).collect();
    assert_eq!(texts, vec!["o", "ffi", "c", "e"]);
    assert_eq!(office.clusters()[1].text_index(), 1);
    assert_eq!(office.get_unit_count(TextUnit::Character), 4);
    // A base with a combining mark is one cluster.
    let accented = lay(&fonts, "e\u{0301}x", 40.0, TextLayoutOptions::default());
    assert_eq!(accented.clusters()[0].text(), "e\u{0301}");
    assert_eq!(accented.cluster_count(), 2);
    // A zero width space takes no room and is not white space (it is a break opportunity).
    let zwsp = lay(&fonts, "a\u{200B}b", 40.0, TextLayoutOptions::default());
    assert_eq!(zwsp.cluster_count(), 3);
    assert_eq!(zwsp.clusters()[1].advance(), 0.0);
    assert!(!zwsp.clusters()[1].is_white_space());
    assert!(!zwsp.has_missing_characters());
}
