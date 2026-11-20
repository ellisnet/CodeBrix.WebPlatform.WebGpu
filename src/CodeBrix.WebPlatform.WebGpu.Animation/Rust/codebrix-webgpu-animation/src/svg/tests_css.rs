//! Host tests: the CSS @keyframes subset against hand-computed values (test_files/css_keyframes.svg).

use kurbo::Point;

use crate::svg::*;

fn load() -> DrawingSvg {
    DrawingSvg::from_svg(include_str!("test_files/css_keyframes.svg"), &DrawingSvgOptions::default()).expect("loads")
}

fn at(svg: &DrawingSvg, id: &str, t: f64) -> SvgPartOverride {
    svg.evaluate_animations(t).get(id).cloned().unwrap_or_default()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn maps(m: kurbo::Affine, p: (f64, f64), q: (f64, f64)) -> bool {
    let r = m * Point::new(p.0, p.1);
    (r.x - q.0).abs() < 1e-6 && (r.y - q.1).abs() < 1e-6
}

#[test]
fn class_selected_spin_turns_about_its_fill_box_centre() {
    //Arrange
    let svg = load();
    let id = svg.get_part_ids().into_iter().find(|i| i.starts_with(SYNTHETIC_ID_PREFIX)).expect("the .spin rect was named");
    //Act
    let m = at(&svg, &id, 0.5).transform.unwrap();
    let later = at(&svg, &id, 4.5).transform.unwrap();
    //Assert: 90 degrees about (20,20) maps (10,10) to (30,10); infinite.
    assert!(maps(m, (10.0, 10.0), (30.0, 10.0)));
    assert!(maps(later, (10.0, 10.0), (30.0, 10.0)));
}

#[test]
fn steps_alternate_and_fill_forwards() {
    //Arrange
    let svg = load();
    //Act / Assert: steps(4, end) - 0.3 -> 0.25; alternate second run 0.7 -> 0.5; ends at 0%.
    assert!(near(at(&svg, "pulse", 0.3).opacity.unwrap(), 0.25));
    assert!(near(at(&svg, "pulse", 1.3).opacity.unwrap(), 0.5));
    assert!(near(at(&svg, "pulse", 5.0).opacity.unwrap(), 0.0));
}

#[test]
fn child_combinator_cubic_bezier_delay_and_fill_both() {
    //Arrange
    let svg = load();
    let id = svg.get_part_ids().into_iter().filter(|i| i.starts_with(SYNTHETIC_ID_PREFIX)).nth(1).expect("the .draw path was named");
    //Act / Assert
    assert!(near(at(&svg, &id, 0.0).stroke_dash_offset.unwrap(), 100.0), "backwards fill during the delay");
    assert!(near(at(&svg, &id, 3.0).stroke_dash_offset.unwrap(), 50.0), "ease-in-out is 0.5 at the half");
    assert_eq!(at(&svg, &id, 3.0).stroke, Some(Some(crate::Color::from_rgb(128, 0, 0))), "from the static #000 to #ff0000");
    assert!(near(at(&svg, &id, 10.0).stroke_dash_offset.unwrap(), 0.0));
}

#[test]
fn missing_end_keyframes_use_the_static_value() {
    //Arrange
    let svg = load();
    //Act / Assert: only a 50% keyframe: none -> translate(10,20) -> none, linear over 2 s.
    assert!(maps(at(&svg, "slide", 0.5).transform.unwrap(), (0.0, 0.0), (5.0, 10.0)));
    assert!(maps(at(&svg, "slide", 1.0).transform.unwrap(), (0.0, 0.0), (10.0, 20.0)));
    assert!(maps(at(&svg, "slide", 1.5).transform.unwrap(), (0.0, 0.0), (5.0, 10.0)));
    assert_eq!(at(&svg, "slide", 3.0).transform, None);
}

#[test]
fn unsupported_css_is_reported() {
    //Arrange
    let svg = load();
    //Act
    let report = svg.report().join("\n");
    //Assert
    assert!(report.contains("@media"), "{report}");
    assert!(report.contains("[data-x]"), "{report}");
    assert_eq!(svg.animations().css_count(), 4);
}

#[test]
fn decomposition_round_trips_and_mismatched_lists_interpolate_through_it() {
    use crate::svg::animation::css_animation::{decompose, lerp_transform, parse_transform, recompose};
    //Arrange
    let m = kurbo::Affine::translate((7.0, -3.0)) * kurbo::Affine::rotate(0.6) * kurbo::Affine::skew(0.3, 0.0) * kurbo::Affine::scale_non_uniform(2.0, 0.5);
    //Act
    let back = recompose(&decompose(m));
    let a = parse_transform("translate(10px, 0) rotate(0deg)", (0.0, 0.0)).unwrap();
    let b = parse_transform("rotate(90deg) translate(10px, 0)", (0.0, 0.0)).unwrap();
    let half = lerp_transform(&a, &b, 0.5);
    //Assert
    for (x, y) in m.as_coeffs().iter().zip(back.as_coeffs()) {
        assert!((x - y).abs() < 1e-9, "{m:?} vs {back:?}");
    }
    // a = translate(10,0); b = rotate(90) then translate = the point (10,0) turned to (0,10), facing 90.
    // Halfway through the decomposition: translate (5,5), rotate 45 degrees (from 360 to 90 -> 360-..:
    // 0 is taken as 360 and the short way round is 360 -> 450 = 90; the half is 405 = 45).
    assert!(maps(half, (0.0, 0.0), (5.0, 5.0)));
    let dir = half * Point::new(1.0, 0.0) - half * Point::new(0.0, 0.0);
    assert!((dir.y.atan2(dir.x).to_degrees() - 45.0).abs() < 1e-6, "{dir:?}");
}

#[test]
fn missing_transform_keyframes_start_from_the_transform_attribute() {
    //Arrange
    let svg = DrawingSvg::from_svg(r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><style>#r { animation: up 2s linear; } @keyframes up { to { transform: translate(10px, 20px); } }</style><rect id="r" width="5" height="5" transform="translate(30 0)"/></svg>"##, &DrawingSvgOptions::default()).unwrap();
    //Act / Assert: from the attribute translate(30,0) to translate(10,20); the half is (20,10).
    assert!(maps(at(&svg, "r", 0.0).transform.unwrap(), (0.0, 0.0), (30.0, 0.0)));
    assert!(maps(at(&svg, "r", 1.0).transform.unwrap(), (0.0, 0.0), (20.0, 10.0)));
}
