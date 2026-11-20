//! Host tests: loading - size, timing, markers, images, errors, dotLottie, and the report.

use crate::lottie::test_support::*;
use crate::lottie::*;

const BASICS: &str = include_str!("test_files/basics.json");
const NOT_DRAWN: &str = include_str!("test_files/not_drawn.json");
const SOLID_IMAGE: &str = include_str!("test_files/solid_image.json");
const MASKS_MATTES: &str = include_str!("test_files/masks_mattes.json");
const A: &str = "A_many_shapes.json";
const B: &str = "B_painterly.json";
const C: &str = "C_busy_scene.json";

#[test]
fn from_json_reads_size_frame_rate_points_and_duration() {
    //Arrange
    let lottie = load(BASICS);
    //Act
    let (w, h, fr) = (lottie.width(), lottie.height(), lottie.frame_rate());
    //Assert
    assert_eq!((w, h, fr), (100.0, 100.0, 10.0));
    assert_eq!((lottie.in_point(), lottie.out_point(), lottie.frame_count()), (0.0, 20.0, 20.0));
    assert!(near(lottie.duration(), 2.0, 1e-12));
    assert!(lottie.report().is_empty(), "{:?}", lottie.report());
}

#[test]
fn from_json_reads_markers() {
    //Arrange
    let lottie = load(BASICS);
    //Act
    let markers = lottie.markers();
    //Assert
    assert_eq!(markers.len(), 3);
    assert_eq!(markers[1], LottieMarker { name: "second half".to_string(), frame: 10.0, duration: 10.0 });
    assert_eq!(lottie.marker("flash").map(|m| (m.frame, m.duration, m.end_frame())), Some((15.0, 0.0, 15.0)));
    assert!(lottie.marker("missing").is_none());
}

#[test]
fn the_three_invented_lotties_load_with_their_sizes_and_timing() {
    //Arrange
    let (Some(a), Some(b), Some(c)) = (trial_file(A), trial_file(B), trial_file(C)) else { return };
    let files = [(&a, 640.0, 640.0), (&b, 800.0, 600.0), (&c, 960.0, 540.0)];
    for (json, w, h) in files {
        //Act
        let lottie = load(json);
        //Assert
        assert_eq!((lottie.width(), lottie.height(), lottie.frame_rate(), lottie.in_point()), (w, h, 30.0, 0.0));
        assert!(lottie.markers().is_empty());
    }
    assert_eq!(load(&a).duration(), 4.0);
    assert_eq!(load(&b).duration(), 5.0);
    assert_eq!(load(&c).duration(), 4.0);
}

#[test]
fn the_invented_lotties_report_only_what_they_use_and_we_do_not_draw() {
    //Arrange
    let (Some(a), Some(b), Some(c)) = (trial_file(A), trial_file(B), trial_file(C)) else { return };
    let (a, b, c) = (load(&a), load(&b), load(&c));
    //Act
    let reports = (a.report(), b.report(), c.report());
    //Assert
    assert!(reports.0.is_empty(), "A: {:?}", reports.0);
    assert!(reports.2.is_empty(), "C: {:?}", reports.2);
    // B: one mask (mode add, drawn exactly) and one alpha matte - both drawn as written.
    assert!(reports.1.is_empty(), "B: {:?}", reports.1);
}

#[test]
fn a_file_velato_refuses_or_panics_on_loads_and_reports_what_is_not_drawn() {
    //Arrange
    let raw = velato::Composition::from_slice(NOT_DRAWN.as_bytes());
    //Act
    let lottie = load(NOT_DRAWN);
    //Assert
    assert!(raw.is_err(), "velato alone refuses the text layer");
    use DrawingLottieWarningKind as K;
    for kind in [
        K::TextLayerNotDrawn,
        K::LayerTypeNotDrawn,
        K::BlendModeIgnored,
        K::StrokeDashesIgnored,
        K::EvenOddFillApproximated,
        K::ShapeModifierIgnored,
        K::UnknownShapeRemoved,
        K::EffectIgnored,
        K::MotionBlurIgnored,
        K::AutoOrientIgnored,
        K::MotionPathApproximated,
        K::ExpressionIgnored,
        K::GradientOpacityIgnored,
        K::GradientHighlightIgnored,
        K::TrimIndividuallyApproximated,
        K::SplitPositionApproximated,
        K::LayerStyleIgnored,
        K::EasingApproximated,
        K::ThreeDimensionalIgnored,
        K::TimeRemapIgnored,
    ] {
        assert!(has(&lottie, kind), "{kind:?} missing from {:#?}", lottie.report());
    }
    for kind in [K::LumaMatteApproximated, K::MaskApproximated, K::ImageNeedsPixels, K::RepeaterCopiesLimited] {
        assert!(!has(&lottie, kind), "{kind:?} reported for nothing");
    }
    assert!(lottie.report().iter().any(|l| l.starts_with("TextLayerNotDrawn: layer 'title' is a text layer")), "{:#?}", lottie.report());
    assert!(lottie.report().iter().any(|l| l.contains("'camera'") && l.contains("type 13")));
    assert!(lottie.report().iter().any(|l| l.contains("'sound'") && l.contains("removed")));
    assert!(lottie.report().iter().any(|l| l.contains("'inner multiply' of precomposition 'inner'")));
}

#[test]
fn masks_and_mattes_report_luma_and_mask_modes() {
    //Arrange
    let lottie = load(MASKS_MATTES);
    //Act
    let report = lottie.report();
    //Assert
    assert!(has(&lottie, DrawingLottieWarningKind::LumaMatteApproximated));
    let mask = report.iter().find(|l| l.starts_with("MaskApproximated")).expect("mask line");
    assert!(mask.contains("'subtracted'") && mask.contains("mode subtract") && mask.contains("inverted") && mask.contains("opacity") && mask.contains("expansion"), "{mask}");
    assert_eq!(report.len(), 2, "the add mask and the none mask are drawn as written: {report:?}");
}

#[test]
fn images_are_listed_with_their_location_and_size() {
    //Arrange
    let lottie = load(SOLID_IMAGE);
    //Act
    let images = lottie.images();
    //Assert
    assert_eq!(images, &[LottieImageAsset { id: "picture".to_string(), size: Some((64.0, 32.0)), location: "images/picture.png".to_string(), is_data_url: false }]);
    assert!(has(&lottie, DrawingLottieWarningKind::ImageNeedsPixels));
}

#[test]
fn load_reads_bytes_with_a_byte_order_mark() {
    //Arrange
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice(BASICS.as_bytes());
    //Act
    let lottie = DrawingLottie::load(&bytes, &DrawingLottieOptions::default());
    //Assert
    assert_eq!(lottie.map(|l| l.frame_count()), Ok(20.0));
}

#[test]
fn load_refuses_a_dotlottie_archive_and_says_what_to_do() {
    //Arrange
    let zip = b"PK\x03\x04\x14\x00\x00\x00manifest.json";
    //Act
    let error = DrawingLottie::load(zip, &DrawingLottieOptions::default()).unwrap_err();
    //Assert
    assert!(error.contains("dotLottie") && error.contains("unpack it on the backend"), "{error}");
}

#[test]
fn errors_say_what_is_wrong() {
    //Arrange
    let options = DrawingLottieOptions::default();
    let cases = [
        ("{ not json", "not valid JSON"),
        ("[1, 2]", "not an object"),
        (r#"{"ip":0,"op":10,"w":10,"h":10,"layers":[]}"#, "no number \"fr\""),
        (r#"{"fr":0,"ip":0,"op":10,"w":10,"h":10,"layers":[]}"#, "must be greater than 0"),
        (r#"{"fr":30,"ip":10,"op":10,"w":10,"h":10,"layers":[]}"#, "is not after its in point"),
        (r#"{"fr":30,"ip":0,"op":10,"w":10.5,"h":10,"layers":[]}"#, "whole numbers"),
        (r#"{"fr":30,"ip":0,"op":10,"w":10,"h":10}"#, "no \"layers\" array"),
        (r#"{"fr":30,"ip":0,"op":10,"w":10,"h":10,"layers":[{"ty":4,"ks":{},"shapes":[]}]}"#, "could not be read"),
    ];
    for (json, expected) in cases {
        //Act
        let error = DrawingLottie::from_json(json, &options).unwrap_err();
        //Assert
        assert!(error.contains(expected), "{json}: {error}");
    }
}

#[test]
fn repeaters_are_limited_by_the_load_options() {
    //Arrange
    let json = include_str!("test_files/repeater.json").replace(r#""c":{"a":0,"k":3}"#, r#""c":{"a":0,"k":5000}"#);
    let options = DrawingLottieOptions { max_repeater_copies: 100, ..DrawingLottieOptions::default() };
    //Act
    let lottie = DrawingLottie::from_json(&json, &options).unwrap();
    let list = lottie.draw_list(0.0, kurbo::Affine::IDENTITY, 1.0, true);
    //Assert
    assert!(has(&lottie, DrawingLottieWarningKind::RepeaterCopiesLimited));
    assert_eq!(list.draw_count(), 100);
}

#[test]
fn resolve_range_clamps_frames_and_finds_markers() {
    //Arrange
    let lottie = load(BASICS);
    //Act
    let whole = lottie.resolve_range(&LottieRange::Whole);
    let frames = lottie.resolve_range(&LottieRange::Frames { start: -5.0, end: 12.0 });
    let span = lottie.resolve_range(&LottieRange::Marker("second half".to_string()));
    let point = lottie.resolve_range(&LottieRange::Marker("flash".to_string()));
    let unknown = lottie.resolve_range(&LottieRange::Marker("outro".to_string())).unwrap_err();
    let empty = lottie.resolve_range(&LottieRange::Frames { start: 8.0, end: 8.0 }).unwrap_err();
    //Assert
    assert_eq!((whole, frames, span, point), (Ok((0.0, 20.0)), Ok((0.0, 12.0)), Ok((10.0, 20.0)), Ok((15.0, 20.0))));
    assert!(unknown.contains("no marker named 'outro'") && unknown.contains("intro, second half, flash"), "{unknown}");
    assert!(empty.contains("holds no frames"), "{empty}");
}

#[test]
fn documents_velato_would_refuse_or_panic_on_still_load() {
    //Arrange - whole numbers written as floats (velato reads "ty", "ind", "w" as integers), a
    // shape group rotation split into x / y / z, a first keyframe with an empty value, and a
    // layer transform without a rotation (velato's importer panics on the last three)
    let json = r#"{"fr":30.0,"ip":0.0,"op":30.0,"w":100.0,"h":50.0,"layers":[
      {"ty":4.0,"ind":1.0,"ip":0,"op":30,"st":0,"ks":{"p":{"a":0,"k":[0,0]},
        "o":{"a":1,"k":[{"t":0,"s":[]},{"t":10,"s":[50]},{"t":20,"s":[100]}]}},
       "shapes":[{"ty":"gr","it":[
         {"ty":"rc","p":{"a":0,"k":[10,10]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
         {"ty":"fl","c":{"a":0,"k":[0,0,0,1]},"o":{"a":0,"k":100},"r":1.0},
         {"ty":"tr","p":{"a":0,"k":[0,0]},"r":{"x":{"a":0,"k":0},"y":{"a":0,"k":0},"z":{"a":0,"k":90}}}]}]}]}"#;
    //Act
    let lottie = DrawingLottie::from_json(json, &DrawingLottieOptions::default()).unwrap();
    let list = lottie.draw_list(10.0, kurbo::Affine::IDENTITY, 1.0, true);
    //Assert
    assert_eq!((lottie.width(), lottie.height()), (100.0, 50.0));
    let LottieDrawCommand::Fill { transform, opacity, .. } = draws(&list)[0] else { panic!("fill") };
    assert!(at(*transform * kurbo::Point::new(1.0, 0.0), 0.0, 1.0), "the group turned by its z rotation: {transform:?}");
    assert!(near(*opacity as f64, 0.5, 1e-6), "the empty first keyframe dropped: 50 at frame 10");
}

#[test]
fn velato_alone_panics_where_the_check_steps_in() {
    //Arrange - three small documents Lottie allows: no layer rotation, the "add" blend mode,
    // per-dimension easing handles on a scalar keyframe
    let layer = |extra: &str, ks: &str| format!(r#"{{"fr":30,"ip":0,"op":30,"w":10,"h":10,"layers":[{{"ty":4,"ip":0,"op":30,"st":0{extra},"ks":{{"p":{{"a":0,"k":[0,0]}}{ks}}},"shapes":[]}}]}}"#);
    let docs = [
        layer("", ""),
        layer(r#","bm":16"#, r#","r":{"a":0,"k":0}"#),
        layer("", r#","r":{"a":1,"k":[{"t":0,"s":[0],"o":{"x":[0.1,0.2],"y":[0,0]},"i":{"x":[0.9,0.8],"y":[1,1]}},{"t":30,"s":[90]}]}"#),
    ];
    for doc in docs {
        //Act
        let velato_alone = std::panic::catch_unwind(|| velato::Composition::from_slice(doc.as_bytes()).is_ok());
        let ours = DrawingLottie::from_json(&doc, &DrawingLottieOptions::default());
        //Assert
        assert!(velato_alone.is_err(), "velato 0.12.0 panics on {doc}");
        assert!(ours.is_ok(), "{doc}: {:?}", ours.err());
    }
}
