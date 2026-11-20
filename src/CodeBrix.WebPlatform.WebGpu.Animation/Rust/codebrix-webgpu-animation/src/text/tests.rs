//! Host tests of the text step: the registry's rules and messages, shaping results for fixed
//! strings (values computed with the same fonts), tracking, alignment, bounds, the three
//! missing-character modes and the reports, and the debug overlay's lines.
//!
//! THE FONTS ARE NOT IN THIS CRATE (OFL fonts are allowed in tests and samples only, never in a
//! package or a crate): they are the test project's, tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/
//! Fonts/ (Roboto and Open Sans, each with its OFL.txt). The folder is given by the environment
//! variable CODEBRIX_ANIMATION_TEST_FONTS (the .NET test that runs these tests sets it); without
//! it the tests look for that folder from this crate's place in the repository, and a test that
//! finds no font says so and passes without checking.

use std::path::PathBuf;

use crate::animation::{AnimationTimeline, EasingKind};
use crate::rendering::{DebugOverlay, DebugOverlayCorner, VectorScene};
use crate::shapes::{DrawingShape, ShapeKind, TextShape, TextShapeOptions};
use crate::text::font_face::TOFU_KEY;
use crate::text::{FontRegistry, MeasureTextOptions, MissingCharacter, MissingCharacterMode, TextAlign};
use crate::{Color, DrawingSession, DrawingSessionOptions};

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

fn roboto() -> Option<FontRegistry> {
    let fonts = FontRegistry::new();
    fonts.register_font(font_bytes("Roboto", "Roboto-Regular.ttf")?, None).unwrap();
    Some(fonts)
}

fn both() -> Option<FontRegistry> {
    let fonts = roboto()?;
    fonts.register_font(font_bytes("OpenSans", "OpenSans-Regular.ttf")?, None).unwrap();
    Some(fonts)
}

fn opts() -> TextShapeOptions {
    TextShapeOptions::default()
}

#[test]
fn an_empty_registry_refuses_text_with_csharps_message() {
    //Arrange
    let fonts = FontRegistry::new();

    //Act
    let error = TextShape::new("Hi", 0.0, 0.0, 10.0, &fonts, opts()).unwrap_err();

    //Assert
    assert_eq!(error, "No font is registered: register a font file before drawing text (system fonts are never used).");
    assert_eq!(fonts.count(), 0);
    assert!(!fonts.contains(None));
    assert!(fonts.register_font(Vec::new(), None).unwrap_err().contains("could not be read as a font"));
    assert!(fonts.register_font(vec![1, 2, 3, 4, 5, 6, 7, 8], None).unwrap_err().contains("could not be read as a font"));
}

#[test]
fn fonts_are_known_by_their_family_name_or_override_without_regard_to_case() {
    let Some(fonts) = both() else { return };
    assert_eq!(fonts.get_registered_family_names(), vec!["Roboto".to_string(), "Open Sans".to_string()]);
    let alias = fonts.register_font(font_bytes("Roboto", "Roboto-Regular.ttf").unwrap(), Some("  Headline  ")).unwrap();
    assert_eq!(alias, "Headline");
    assert!(fonts.contains(Some("roboto")));
    assert!(fonts.contains(Some("OPEN SANS")));
    assert!(fonts.contains(Some("headline")));
    let shape = TextShape::new("x", 0.0, 0.0, 10.0, &fonts, TextShapeOptions { family_name: Some("open sans".into()), ..opts() }).unwrap();
    assert_eq!(shape.family_name(), "Open Sans");
    let first = TextShape::new("x", 0.0, 0.0, 10.0, &fonts, opts()).unwrap();
    assert_eq!(first.family_name(), "Roboto");
    let error = TextShape::new("x", 0.0, 0.0, 10.0, &fonts, TextShapeOptions { family_name: Some("Comic".into()), ..opts() }).unwrap_err();
    assert!(error.starts_with("No font is registered under the family name 'Comic'"), "{error}");
}

#[test]
fn a_shared_registry_handle_sees_fonts_registered_later() {
    let Some(bytes) = font_bytes("Roboto", "Roboto-Regular.ttf") else { return };
    let fonts = FontRegistry::new();
    let session = DrawingSession::new(DrawingSessionOptions { fonts: Some(fonts.clone()), ..DrawingSessionOptions::default() });
    assert!(!session.fonts().contains(None));
    fonts.register_font(bytes, None).unwrap();
    assert!(session.fonts().contains(Some("Roboto")));
    assert_eq!(session.fonts(), &fonts);
    assert_ne!(&FontRegistry::new(), &fonts);
}

#[test]
fn kerning_is_applied_and_can_be_switched_off() {
    let Some(fonts) = roboto() else { return };
    // Roboto has 2048 units per em: at size 2048 one calibrated unit is one font unit. "AV" is
    // kerned to 2553 units (plain advances 1320 + 1320); "HH" has no kerning pair (2922 both).
    let kerned = fonts.measure_text("AV", 2048.0, &MeasureTextOptions::default()).unwrap();
    let plain = fonts.measure_text("AV", 2048.0, &MeasureTextOptions { kerning: false, ..MeasureTextOptions::default() }).unwrap();
    assert_eq!(kerned, 2553.0);
    assert_eq!(plain, 2640.0);
    assert_eq!(fonts.measure_text("HH", 2048.0, &MeasureTextOptions::default()).unwrap(), 2922.0);
    assert_eq!(fonts.measure_text("HH", 2048.0, &MeasureTextOptions { kerning: false, ..MeasureTextOptions::default() }).unwrap(), 2922.0);
    let shape = TextShape::new("AV", 0.0, 0.0, 2048.0, &fonts, TextShapeOptions { kerning: false, ..opts() }).unwrap();
    assert_eq!(shape.width(), 2640.0);
    assert!(!shape.kerning());
}

#[test]
fn a_ligature_is_one_cluster_with_utf8_byte_offsets() {
    let Some(fonts) = roboto() else { return };
    // Roboto's "ffi" ligature: "office" is o, ffi, c, e.
    let shape = TextShape::new("office", 0.0, 0.0, 20.0, &fonts, opts()).unwrap();
    let c = shape.clusters();
    assert_eq!(c.len(), 4);
    assert_eq!((c[1].text_index, c[1].text_length), (1, 3));
    assert_eq!((c[2].text_index, c[3].text_index), (4, 5));
    // Byte offsets: "é" is two bytes.
    let accented = TextShape::new("éa", 0.0, 0.0, 20.0, &fonts, opts()).unwrap();
    assert_eq!(accented.clusters()[1].text_index, 2);
    assert!(!accented.has_missing_characters());
}

#[test]
fn tracking_adds_space_after_every_cluster_but_the_last() {
    let Some(fonts) = roboto() else { return };
    let plain = TextShape::new("Animate", 0.0, 0.0, 60.0, &fonts, opts()).unwrap();
    let tracked = TextShape::new("Animate", 0.0, 0.0, 60.0, &fonts, TextShapeOptions { tracking: 0.25, ..opts() }).unwrap();
    let measured = fonts.measure_text("Animate", 60.0, &MeasureTextOptions { tracking: 0.25, ..MeasureTextOptions::default() }).unwrap();
    assert!((tracked.width() - (plain.width() + 6.0 * 0.25 * 60.0)).abs() < 1e-3);
    assert_eq!(tracked.width(), measured);
    assert!((tracked.clusters()[3].x - (plain.clusters()[3].x + 3.0 * 15.0)).abs() < 1e-3);
    assert!(TextShape::new("x", 0.0, 0.0, 10.0, &fonts, TextShapeOptions { tracking: f32::NAN, ..opts() }).is_err());
    assert!(TextShape::new("x", 0.0, 0.0, 0.0, &fonts, opts()).is_err());
    assert!(TextShape::new("x", 0.0, 0.0, 10.0, &fonts, TextShapeOptions { align: TextAlign::Justify, ..opts() }).is_err());
}

#[test]
fn alignment_places_the_line_about_its_anchor_and_bounds_follow() {
    let Some(fonts) = roboto() else { return };
    let left = TextShape::new("Animate", 640.0, 190.0, 60.0, &fonts, opts()).unwrap();
    let center = TextShape::new("Animate", 640.0, 190.0, 60.0, &fonts, TextShapeOptions { align: TextAlign::Center, ..opts() }).unwrap();
    let right = TextShape::new("Animate", 640.0, 190.0, 60.0, &fonts, TextShapeOptions { align: TextAlign::Right, ..opts() }).unwrap();
    assert_eq!(left.left(), 640.0);
    assert!((center.left() - (640.0 - left.width() / 2.0)).abs() < 1e-3);
    assert!((right.left() - (640.0 - left.width())).abs() < 1e-3);
    let b = left.get_bounds();
    // Capitals sit on the baseline and rise about 0.71 em (Roboto's cap height is 1456 units).
    assert!(b.y < 190.0 - 0.6 * 60.0 && b.y > 190.0 - 0.8 * 60.0, "{b:?}");
    assert!((b.bottom() - 190.0).abs() < 1.5, "{b:?}");
    assert!(b.x >= 640.0 && b.right() <= 640.0 + left.width() + 1.0, "{b:?}");
    let rb = right.get_bounds();
    assert!((rb.x - (b.x - left.width())).abs() < 1e-3);
    // The outline is the drawn geometry; a space has none.
    assert!(!left.get_outline().is_empty());
    let space = TextShape::new("   ", 10.0, 20.0, 30.0, &fonts, opts()).unwrap();
    assert!(space.get_outline().is_empty());
    assert_eq!(space.get_bounds(), crate::RectangleF::new(10.0, 20.0, 0.0, 0.0));
    assert!(space.width() > 0.0);
}

#[test]
fn a_missing_character_is_reported_and_keeps_a_sensible_advance() {
    let Some(fonts) = both() else { return };
    // "日" is in neither font. Roboto's .notdef is a box with an advance of 908 units.
    let shape = TextShape::new("A日B", 0.0, 0.0, 2048.0, &fonts, opts()).unwrap();
    assert_eq!(shape.missing_characters(), &[MissingCharacter { text_index: 1, character: '日' }]);
    assert!(shape.has_missing_characters());
    let c = shape.clusters();
    assert_eq!(c.len(), 3);
    assert!(c[1].is_missing && !c[0].is_missing && !c[2].is_missing);
    assert_eq!(c[1].advance, 908.0);
    assert_eq!(fonts.get_missing_characters("A日B שלום", None).unwrap().iter().map(|m| m.character).collect::<String>(), "日שלום");
    assert!(fonts.get_missing_characters("Hello, world", None).unwrap().is_empty());
    // A zero-width non-joiner is invisible, never "missing".
    assert!(fonts.get_missing_characters("of\u{200C}fice", None).unwrap().is_empty());
    // Open Sans's .notdef has an advance too; the missing advance is never 0.
    let open = TextShape::new("A日B", 0.0, 0.0, 100.0, &fonts, TextShapeOptions { family_name: Some("Open Sans".into()), ..opts() }).unwrap();
    assert!(open.clusters()[1].advance > 10.0);
}

fn glyph_keys(shape: &TextShape) -> Vec<u32> {
    shape.glyph_keys()
}

#[test]
fn the_three_missing_character_modes_draw_what_the_owner_ruled() {
    let Some(fonts) = both() else { return };
    let mode = |family: &str, m: MissingCharacterMode| TextShape::new("A日B", 0.0, 100.0, 100.0, &fonts, TextShapeOptions { family_name: Some(family.into()), missing_characters: m, ..opts() }).unwrap();
    // Roboto: its .notdef has ink, so ShowTofu and FontDefault both draw glyph 0 (the font's box).
    let r_tofu = mode("Roboto", MissingCharacterMode::ShowTofu);
    let r_default = mode("Roboto", MissingCharacterMode::FontDefault);
    let r_blank = mode("Roboto", MissingCharacterMode::Blank);
    assert_eq!(glyph_keys(&r_tofu).len(), 3);
    assert!(glyph_keys(&r_tofu).contains(&0));
    assert_eq!(glyph_keys(&r_default), glyph_keys(&r_tofu));
    assert_eq!(glyph_keys(&r_blank).len(), 2);
    assert!(!glyph_keys(&r_blank).contains(&0));
    // Open Sans: its .notdef has NO ink - ShowTofu draws the crate's hollow box, FontDefault
    // draws the font's own (nothing visible), Blank nothing.
    let o_tofu = mode("Open Sans", MissingCharacterMode::ShowTofu);
    let o_default = mode("Open Sans", MissingCharacterMode::FontDefault);
    let o_blank = mode("Open Sans", MissingCharacterMode::Blank);
    assert!(glyph_keys(&o_tofu).contains(&TOFU_KEY));
    assert_eq!(glyph_keys(&o_tofu).len(), 3);
    assert_eq!(glyph_keys(&o_default).len(), 2);
    assert_eq!(glyph_keys(&o_blank).len(), 2);
    // Every mode keeps the same advance and the same report.
    for s in [&r_tofu, &r_default, &r_blank] {
        assert_eq!(s.width(), r_tofu.width());
        assert_eq!(s.missing_characters().len(), 1);
    }
    for s in [&o_tofu, &o_default, &o_blank] {
        assert_eq!(s.width(), o_tofu.width());
        assert_eq!(s.missing_characters().len(), 1);
    }
    // The hollow box sits inside the missing advance, from the baseline up, and is two contours.
    let a_width = mode("Open Sans", MissingCharacterMode::Blank).clusters()[0].advance;
    let box_bounds = o_tofu.get_bounds();
    let blank_bounds = o_blank.get_bounds();
    assert!(box_bounds.width >= blank_bounds.width);
    let tofu_cluster = o_tofu.clusters()[1];
    assert!(tofu_cluster.x >= a_width - 1e-3);
    let outline = o_tofu.get_outline();
    let moves = outline.bez_path().elements().iter().filter(|e| matches!(e, kurbo::PathEl::MoveTo(_))).count();
    assert!(moves >= 2 + 2, "A, B and the box's two contours");
}

#[test]
fn a_text_shape_records_one_fill_per_visible_glyph_with_the_glyph_geometry_key() {
    let Some(fonts) = roboto() else { return };
    let mut session = DrawingSession::new(DrawingSessionOptions { fonts: Some(fonts.clone()), layer_opacity: 255, ..DrawingSessionOptions::default() });
    session.add_layer("Text", Color::WHITE);
    let shape = session.draw_text("A A", 10.0, 50.0, 30.0, TextShapeOptions { color: Some(Color::from_rgb(1, 2, 3)), ..opts() }).unwrap();
    assert_eq!(shape.text(), "A A");
    let layer = session.get_layer("Text").unwrap();
    assert!(matches!(layer.get_elements()[0].kind(), ShapeKind::Text(_)));
    let mut scene = VectorScene::new();
    session.render(&mut scene, 1000, 1000);
    // Two "A"s (the space draws nothing), with the same geometry key.
    assert_eq!(scene.draw_count(), 2);
    let keys = scene.fill_keys();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], keys[1]);
    let _ = DrawingShape::from(shape);
}

#[test]
fn a_right_to_left_line_is_shaped_right_to_left() {
    let Some(fonts) = roboto() else { return };
    // Roboto has no Hebrew; the line is still shaped right to left (and reported missing).
    let shape = TextShape::new("שלום", 0.0, 0.0, 20.0, &fonts, opts()).unwrap();
    assert!(shape.is_right_to_left());
    assert_eq!(shape.missing_characters().len(), 4);
    // Drawing order is visual: the last character's cluster comes first.
    assert_eq!(shape.clusters()[0].text_index, 6);
    assert!(!TextShape::new("abc", 0.0, 0.0, 20.0, &fonts, opts()).unwrap().is_right_to_left());
}

#[test]
fn the_debug_overlay_shows_timecode_frame_and_cues_and_fails_soft() {
    //Arrange
    let mut timeline = AnimationTimeline::new(10.0, 30.0).unwrap();
    timeline.add_cue("intro", 0.0, 2.0, EasingKind::Linear).unwrap();
    let mut overlay = DebugOverlay::new(None, Some(timeline));
    overlay.show_cues = true;
    overlay.corner = DebugOverlayCorner::BottomRight;

    //Act
    let lines = overlay.get_lines(1.5);

    //Assert
    assert_eq!(lines, vec!["00:00:01:15   1.500 s".to_string(), "frame 45 / 300".to_string(), "cues: intro".to_string()]);
    assert_eq!(overlay.get_lines(3.0)[2], "cues: none");
    assert_eq!(DebugOverlay::new(None, None).get_lines(2.0), vec!["2.000 s".to_string()]);
    assert!(!overlay.can_draw_text());
    let mut scene = VectorScene::new();
    overlay.render(&mut scene, 1280, 720, 1.5);
    assert!(scene.is_empty(), "no font: nothing at all");
    overlay.fonts = Some(FontRegistry::new());
    overlay.render(&mut scene, 1280, 720, 1.5);
    assert!(scene.is_empty(), "an empty registry: nothing at all");
    assert!(overlay.set_text_size(-1.0).is_err());
    assert!(overlay.set_text_size(0.0).is_ok());
    let Some(fonts) = roboto() else { return };
    overlay.fonts = Some(fonts);
    overlay.render(&mut scene, 1280, 720, 1.5);
    assert!(scene.draw_count() > 10, "a plate and the glyphs");
    overlay.family_name = Some("Unknown".into());
    assert!(!overlay.can_draw_text());
}
