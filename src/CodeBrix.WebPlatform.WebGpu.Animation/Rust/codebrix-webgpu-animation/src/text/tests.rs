//! Host tests of the text step: the registry's rules and messages, shaping results for fixed
//! strings (values computed with the same fonts), tracking, alignment, bounds, the three
//! missing-character modes and the reports, the debug overlay's lines, and the variable-font
//! registration (with the variable Roboto, Roboto.ttf: weight 100..900 and width 75..100 - two
//! weights differ, the default registration is the default instance, measured and drawn text
//! agree, the refusals name its real axes; and the refusal for the static fonts).
//!
//! THE FONTS ARE NOT IN THIS CRATE (OFL fonts are allowed in tests and samples only, never in a
//! package or a crate): they are the test project's, tests/CodeBrix.WebPlatform.WebGpu.Animation.Tests/
//! Fonts/ (Roboto - the static Roboto-Regular.ttf and the variable Roboto.ttf - and Open Sans, each
//! family with its OFL.txt). The folder is given by the environment variable
//! CODEBRIX_ANIMATION_TEST_FONTS (the .NET test that runs these tests sets it); without it the
//! tests look for that folder from this crate's place in the repository, and a test that finds no
//! font says so and passes without checking.

use std::path::PathBuf;

use crate::animation::{AnimationTimeline, EasingKind};
use crate::rendering::{DebugOverlay, DebugOverlayCorner, VectorScene};
use crate::shapes::{DrawingShape, ShapeKind, TextShape, TextShapeOptions};
use crate::text::font_face::{check_variations, TOFU_KEY};
use crate::text::{FontRegistry, FontVariation, FontVariationAxis, MeasureTextOptions, MissingCharacter, MissingCharacterMode, TextAlign};
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

/// The axes of a made-up variable font: weight 100..900 (default 400), width 75..100 (default
/// 100). The checks are tested against them because no variable font is among the test fonts.
fn made_up_axes() -> Vec<FontVariationAxis> {
    vec![
        FontVariationAxis { tag: *b"wght", min_value: 100.0, default_value: 400.0, max_value: 900.0, is_hidden: false },
        FontVariationAxis { tag: *b"wdth", min_value: 75.0, default_value: 100.0, max_value: 100.0, is_hidden: false },
    ]
}

#[test]
fn axis_values_inside_a_fonts_axes_pass_the_checks() {
    //Arrange
    let axes = made_up_axes();
    //Act
    let bold = check_variations(&axes, &[FontVariation::weight(700.0)]);
    let both_ends = check_variations(&axes, &[FontVariation::weight(100.0), FontVariation::width(100.0)]);
    let none = check_variations(&axes, &[]);
    //Assert
    assert!(bold.is_ok() && both_ends.is_ok() && none.is_ok());
}

#[test]
fn an_unknown_axis_a_value_outside_its_range_and_an_axis_given_twice_are_refused() {
    //Arrange
    let axes = made_up_axes();
    //Act
    let unknown = check_variations(&axes, &[FontVariation::new(*b"opsz", 12.0)]).unwrap_err();
    let too_heavy = check_variations(&axes, &[FontVariation::weight(1000.0)]).unwrap_err();
    let not_a_number = check_variations(&axes, &[FontVariation::width(f32::NAN)]).unwrap_err();
    let twice = check_variations(&axes, &[FontVariation::weight(500.0), FontVariation::weight(600.0)]).unwrap_err();
    //Assert
    assert_eq!(unknown, "The font has no 'opsz' axis; its axes are wght 100..900 (default 400), wdth 75..100 (default 100).");
    assert_eq!(too_heavy, "The value 1000 is outside the font's 'wght' axis, which runs from 100 to 900 (default 400): pass a value in that range.");
    assert!(not_a_number.contains("outside the font's 'wdth' axis"), "{not_a_number}");
    assert!(twice.starts_with("The 'wght' axis is given more than once"), "{twice}");
}

#[test]
fn an_axis_value_for_a_font_that_is_not_variable_is_refused_and_registers_nothing() {
    //Arrange
    let Some(bytes) = font_bytes("Roboto", "Roboto-Regular.ttf") else { return };
    let fonts = FontRegistry::new();
    //Act
    let error = fonts.register_font_with_variations(bytes, Some("Bold"), &[FontVariation::weight(700.0)]).unwrap_err();
    //Assert
    assert_eq!(error, "The font has no 'wght' axis; it has no variation axes (it is not a variable font): register it without axis values.");
    assert_eq!(fonts.count(), 0);
}

#[test]
fn no_axis_values_register_the_default_instance_exactly_as_register_font() {
    //Arrange
    let Some(fonts) = roboto() else { return };
    let bytes = font_bytes("Roboto", "Roboto-Regular.ttf").unwrap();
    //Act
    let name = fonts.register_font_with_variations(bytes, Some("Plain"), &[]).unwrap();
    let default_width = fonts.measure_text("AV Animate", 2048.0, &MeasureTextOptions::default()).unwrap();
    let plain_width = fonts.measure_text("AV Animate", 2048.0, &MeasureTextOptions { family_name: Some("Plain".into()), ..MeasureTextOptions::default() }).unwrap();
    let default_shape = TextShape::new("AV", 0.0, 0.0, 100.0, &fonts, opts()).unwrap();
    let plain_shape = TextShape::new("AV", 0.0, 0.0, 100.0, &fonts, TextShapeOptions { family_name: Some("Plain".into()), ..opts() }).unwrap();
    //Assert
    assert_eq!(name, "Plain");
    assert_eq!(plain_width, default_width);
    assert_eq!(plain_shape.width(), default_shape.width());
    assert_eq!(plain_shape.get_bounds(), default_shape.get_bounds());
    assert_eq!(plain_shape.clusters(), default_shape.clusters());
    assert_eq!(fonts.get_variation_axes(Some("Plain")).unwrap(), Vec::<FontVariationAxis>::new());
    assert_eq!(fonts.get_variation_axes(None).unwrap(), Vec::<FontVariationAxis>::new());
    assert!(FontRegistry::new().get_variation_axes(None).unwrap_err().starts_with("No font is registered"));
}

#[test]
fn a_variation_and_an_axis_show_their_tag_as_text() {
    //Arrange
    let axis = made_up_axes()[0];
    //Act
    let shown = axis.to_string();
    //Assert
    assert_eq!(shown, "wght 100..900 (default 400)");
    assert_eq!(FontVariation::width(90.0).tag_text(), "wdth");
    assert_eq!(FontVariation::new([b'a', 0, b'c', b'd'], 1.0).tag_text(), "a?cd");
}

/// The variable Roboto (Roboto.ttf: axes wght 100..900, default 400, and wdth 75..100, default
/// 100), from the test fonts.
fn variable_roboto() -> Option<Vec<u8>> {
    font_bytes("Roboto", "Roboto.ttf")
}

/// The options of a measurement in one family.
fn in_family(name: &str) -> MeasureTextOptions {
    MeasureTextOptions { family_name: Some(name.into()), ..MeasureTextOptions::default() }
}

#[test]
fn a_variable_font_reports_its_real_axes() {
    //Arrange
    let Some(bytes) = variable_roboto() else { return };
    let fonts = FontRegistry::new();
    fonts.register_font(bytes, None).unwrap();
    //Act
    let axes = fonts.get_variation_axes(None).unwrap();
    //Assert
    assert_eq!(axes.iter().map(|a| a.to_string()).collect::<Vec<_>>(), vec!["wght 100..900 (default 400)".to_string(), "wdth 75..100 (default 100)".to_string()]);
    assert!(axes.iter().all(|a| !a.is_hidden));
}

#[test]
fn two_weights_of_a_variable_font_give_different_advances_and_outlines() {
    //Arrange
    let Some(bytes) = variable_roboto() else { return };
    let fonts = FontRegistry::new();
    fonts.register_font_with_variations(bytes.clone(), Some("Light"), &[FontVariation::weight(300.0)]).unwrap();
    fonts.register_font_with_variations(bytes, Some("Heavy"), &[FontVariation::weight(800.0)]).unwrap();
    //Act
    let light_width = fonts.measure_text("Animate", 100.0, &in_family("Light")).unwrap();
    let heavy_width = fonts.measure_text("Animate", 100.0, &in_family("Heavy")).unwrap();
    let light = TextShape::new("o", 0.0, 0.0, 100.0, &fonts, TextShapeOptions { family_name: Some("Light".into()), ..opts() }).unwrap();
    let heavy = TextShape::new("o", 0.0, 0.0, 100.0, &fonts, TextShapeOptions { family_name: Some("Heavy".into()), ..opts() }).unwrap();
    //Assert
    assert!(heavy_width > light_width, "a heavier weight is wider: {light_width} -> {heavy_width}");
    assert_ne!(light.get_outline(), heavy.get_outline());
    assert!(heavy.get_outline().get_bounds().width > light.get_outline().get_bounds().width, "a heavier 'o' has more ink");
}

#[test]
fn the_default_registration_of_a_variable_font_is_its_default_instance() {
    //Arrange
    let Some(bytes) = variable_roboto() else { return };
    let fonts = FontRegistry::new();
    fonts.register_font(bytes.clone(), Some("Plain")).unwrap();
    fonts.register_font_with_variations(bytes.clone(), Some("Defaults"), &[FontVariation::weight(400.0), FontVariation::width(100.0)]).unwrap();
    fonts.register_font_with_variations(bytes, Some("Bold"), &[FontVariation::weight(700.0)]).unwrap();
    let shape = |family: &str| TextShape::new("Wave", 0.0, 0.0, 100.0, &fonts, TextShapeOptions { family_name: Some(family.into()), ..opts() }).unwrap();
    //Act
    let (plain, defaults, bold) = (shape("Plain"), shape("Defaults"), shape("Bold"));
    //Assert
    assert_eq!(plain.width(), defaults.width());
    assert_eq!(plain.get_outline(), defaults.get_outline());
    assert_eq!(fonts.measure_text("Wave", 100.0, &in_family("Plain")).unwrap(), fonts.measure_text("Wave", 100.0, &in_family("Defaults")).unwrap());
    assert_ne!(plain.get_outline(), bold.get_outline(), "an axis value does change the font");
}

#[test]
fn text_measured_and_drawn_uses_the_same_instance() {
    //Arrange
    let Some(bytes) = variable_roboto() else { return };
    let fonts = FontRegistry::new();
    fonts.register_font(bytes.clone(), Some("Plain")).unwrap();
    fonts.register_font_with_variations(bytes, Some("Narrow Bold"), &[FontVariation::weight(700.0), FontVariation::width(75.0)]).unwrap();
    let options = MeasureTextOptions { tracking: 0.05, ..in_family("Narrow Bold") };
    //Act
    let measured = fonts.measure_text("Animate", 60.0, &options).unwrap();
    let drawn = TextShape::new("Animate", 0.0, 0.0, 60.0, &fonts, TextShapeOptions { family_name: Some("Narrow Bold".into()), tracking: 0.05, ..opts() }).unwrap();
    let plain = TextShape::new("Animate", 0.0, 0.0, 60.0, &fonts, TextShapeOptions { family_name: Some("Plain".into()), tracking: 0.05, ..opts() }).unwrap();
    //Assert
    assert_eq!(drawn.width(), measured);
    assert_ne!(drawn.width(), plain.width());
    let last = drawn.clusters().last().unwrap();
    assert!(drawn.get_outline().get_bounds().right() <= drawn.left() + last.x + last.advance + 60.0 * 0.05, "the drawn outlines stay within the measured advances");
}

#[test]
fn an_unknown_axis_and_a_value_outside_a_real_axis_are_refused_with_the_fonts_axes() {
    //Arrange
    let Some(bytes) = variable_roboto() else { return };
    let fonts = FontRegistry::new();
    //Act
    let unknown = fonts.register_font_with_variations(bytes.clone(), Some("Optical"), &[FontVariation::new(*b"opsz", 14.0)]).unwrap_err();
    let too_light = fonts.register_font_with_variations(bytes.clone(), Some("Hairline"), &[FontVariation::weight(50.0)]).unwrap_err();
    let too_wide = fonts.register_font_with_variations(bytes, Some("Wide"), &[FontVariation::width(125.0)]).unwrap_err();
    //Assert
    assert_eq!(unknown, "The font has no 'opsz' axis; its axes are wght 100..900 (default 400), wdth 75..100 (default 100).");
    assert_eq!(too_light, "The value 50 is outside the font's 'wght' axis, which runs from 100 to 900 (default 400): pass a value in that range.");
    assert_eq!(too_wide, "The value 125 is outside the font's 'wdth' axis, which runs from 75 to 100 (default 100): pass a value in that range.");
    assert_eq!(fonts.count(), 0, "nothing is registered when a value is refused");
}
