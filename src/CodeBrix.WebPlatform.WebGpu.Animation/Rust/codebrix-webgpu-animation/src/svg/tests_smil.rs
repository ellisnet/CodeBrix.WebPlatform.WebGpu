//! Host tests: SMIL timing and values against hand-computed values (test_files/smil_*.svg).

use kurbo::{Affine, Point};

use crate::animation::CubicBezierEasing;
use crate::svg::animation::smil_timing::clock;
use crate::svg::*;

fn load(text: &str) -> DrawingSvg {
    DrawingSvg::from_svg(text, &DrawingSvgOptions::default()).expect("loads")
}

fn at(svg: &DrawingSvg, id: &str, t: f64) -> SvgPartOverride {
    svg.evaluate_animations(t).get(id).cloned().unwrap_or_default()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn maps(m: Affine, p: (f64, f64), q: (f64, f64)) -> bool {
    let r = m * Point::new(p.0, p.1);
    (r.x - q.0).abs() < 1e-6 && (r.y - q.1).abs() < 1e-6
}

#[test]
fn clock_values_parse_every_form() {
    assert_eq!(clock("02:30:03"), Some(9003.0));
    assert_eq!(clock("02:33"), Some(153.0));
    assert_eq!(clock("3.2h"), Some(11520.0));
    assert_eq!(clock("45min"), Some(2700.0));
    assert_eq!(clock("30s"), Some(30.0));
    assert_eq!(clock("5ms"), Some(0.005));
    assert_eq!(clock("12.467"), Some(12.467));
    assert_eq!(clock("soon"), None);
}

#[test]
fn animate_with_key_times_begin_offset_and_freeze() {
    //Arrange
    let svg = load(include_str!("test_files/smil_basics.svg"));
    //Act / Assert: values 0;1;0.5 at keyTimes 0;0.25;1 over 4 s from 1 s.
    assert_eq!(at(&svg, "fader", 0.5).opacity, None, "before its begin it has no effect");
    assert!(near(at(&svg, "fader", 1.5).opacity.unwrap(), 0.5));
    assert!(near(at(&svg, "fader", 2.0).opacity.unwrap(), 1.0));
    assert!(near(at(&svg, "fader", 3.5).opacity.unwrap(), 0.75));
    assert!(near(at(&svg, "fader", 10.0).opacity.unwrap(), 0.5), "frozen at the last value");
}

#[test]
fn animate_transform_rotate_repeats_and_translate_sums() {
    //Arrange
    let svg = load(include_str!("test_files/smil_basics.svg"));
    //Act
    let spin = at(&svg, "spinner", 0.5).transform.unwrap();
    let spin_later = at(&svg, "spinner", 2.5).transform.unwrap();
    let slide = at(&svg, "slider", 2.0).transform.unwrap();
    //Assert: 90 degrees about (50,50) maps (40,40) to (60,40); additive sum on translate(5 0).
    assert!(maps(spin, (40.0, 40.0), (60.0, 40.0)));
    assert!(maps(spin_later, (40.0, 40.0), (60.0, 40.0)), "repeatCount indefinite");
    assert!(maps(slide, (0.0, 0.0), (25.0, 0.0)));
    assert_eq!(at(&svg, "slider", 5.0).transform, None, "fill=remove");
}

#[test]
fn set_and_colour_animate() {
    //Arrange
    let svg = load(include_str!("test_files/smil_basics.svg"));
    //Act / Assert
    assert_eq!(at(&svg, "blinker", 2.5).visibility, Some(false));
    assert_eq!(at(&svg, "blinker", 3.5).visibility, None, "set ends with its dur (fill remove)");
    assert_eq!(at(&svg, "blinker", 1.0).fill, Some(Some(crate::Color::from_rgba(128, 0, 0, 255))));
    assert_eq!(at(&svg, "blinker", 5.0).fill, Some(Some(crate::Color::from_rgba(255, 0, 0, 255))));
}

#[test]
fn by_animation_accumulates_over_repeats_on_an_element_without_id() {
    //Arrange
    let svg = load(include_str!("test_files/smil_basics.svg"));
    let id = svg.get_part_ids().into_iter().find(|i| i.starts_with(SYNTHETIC_ID_PREFIX)).expect("the crate named the target");
    //Act / Assert: base 1, by 4 per second, accumulate sum, 3 repeats.
    assert!(near(at(&svg, &id, 0.5).stroke_width.unwrap(), 3.0));
    assert!(near(at(&svg, &id, 1.5).stroke_width.unwrap(), 7.0));
    assert!(near(at(&svg, &id, 10.0).stroke_width.unwrap(), 13.0));
}

#[test]
fn event_begins_and_unsupported_attributes_are_reported() {
    //Arrange
    let svg = load(include_str!("test_files/smil_basics.svg"));
    //Act
    let report = svg.report().join("\n");
    //Assert
    assert!(report.contains("click"), "{report}");
    assert!(report.contains("the attribute 'x' cannot be overridden"), "{report}");
    assert_eq!(at(&svg, "clicked", 0.5).opacity, None);
}

#[test]
fn animate_motion_follows_a_path_with_rotate_auto() {
    //Arrange
    let svg = load(include_str!("test_files/smil_motion.svg"));
    //Act
    let quarter = at(&svg, "car", 1.0).motion.unwrap();
    let three_quarters = at(&svg, "car", 3.0).motion.unwrap();
    let end = at(&svg, "car", 10.0).motion.unwrap();
    //Assert: path length 200, paced: 50 along = (50,0) heading 0; 150 along = (100,50) heading 90.
    assert!(maps(quarter, (0.0, 0.0), (50.0, 0.0)));
    assert!(maps(quarter, (1.0, 0.0), (51.0, 0.0)));
    assert!(maps(three_quarters, (0.0, 0.0), (100.0, 50.0)));
    assert!(maps(three_quarters, (1.0, 0.0), (100.0, 51.0)), "rotate=auto turns with the path");
    assert!(maps(end, (0.0, 0.0), (100.0, 100.0)));
}

#[test]
fn animate_motion_through_values_is_linear_per_segment() {
    //Arrange
    let svg = load(include_str!("test_files/smil_motion.svg"));
    //Act / Assert: three points, two segments of 1.5 s each.
    assert!(maps(at(&svg, "ball", 0.75).motion.unwrap(), (0.0, 0.0), (5.0, 0.0)));
    assert!(maps(at(&svg, "ball", 2.25).motion.unwrap(), (0.0, 0.0), (10.0, 15.0)));
    assert_eq!(at(&svg, "ball", 4.0).motion, None);
}

#[test]
fn spline_discrete_and_paced_modes() {
    //Arrange
    let svg = load(include_str!("test_files/smil_timing.svg"));
    let ease = CubicBezierEasing::new(0.42, 0.0, 0.58, 1.0).unwrap();
    //Act / Assert
    assert!(near(at(&svg, "eased", 0.5).opacity.unwrap(), ease.evaluate(0.25) as f32));
    assert!((ease.evaluate(0.25) - 0.1291).abs() < 1e-3, "ease-in-out at 0.25 (hand value 0.129)");
    assert_eq!(at(&svg, "stepped", 0.5).fill, Some(Some(crate::Color::from_rgb(255, 0, 0))));
    assert_eq!(at(&svg, "stepped", 1.5).fill, Some(Some(crate::Color::from_rgb(0, 255, 0))));
    assert_eq!(at(&svg, "stepped", 2.9).fill, Some(Some(crate::Color::from_rgb(0, 0, 255))));
    assert_eq!(at(&svg, "stepped", 5.0).fill, Some(Some(crate::Color::from_rgb(0, 0, 255))));
    // paced 0;1;4: distances 1 and 3 over 4 s = one unit a second.
    assert!(near(at(&svg, "paced", 1.0).stroke_width.unwrap(), 1.0));
    assert!(near(at(&svg, "paced", 2.0).stroke_width.unwrap(), 2.0));
    assert!(near(at(&svg, "paced", 3.0).stroke_width.unwrap(), 3.0));
}

#[test]
fn syncbase_chains_repeat_dur_end_and_to_animations() {
    //Arrange
    let svg = load(include_str!("test_files/smil_timing.svg"));
    //Act / Assert
    assert!(near(at(&svg, "pinger", 1.25).opacity.unwrap(), 0.25), "pong began at ping.end");
    assert!(near(at(&svg, "pinger", 2.5).opacity.unwrap(), 0.5), "ping began again at pong.end");
    assert!(near(at(&svg, "pinger", 10.5).opacity.unwrap(), 0.5), "the chain keeps going");
    assert!(near(at(&svg, "limited", 2.25).opacity.unwrap(), 0.25));
    assert!(near(at(&svg, "limited", 5.0).opacity.unwrap(), 0.5), "frozen where repeatDur cut it");
    assert!(near(at(&svg, "ended", 1.0).opacity.unwrap(), 0.1));
    assert!(near(at(&svg, "ended", 5.0).opacity.unwrap(), 0.2), "frozen where end cut it");
    assert!(near(at(&svg, "toward", 2.0).opacity.unwrap(), 0.6), "to-animation from the base 0.2");
    assert_eq!(at(&svg, "toward", 4.0).opacity, None);
}

#[test]
fn file_animation_and_walk_combine() {
    //Arrange
    let svg = std::sync::Arc::new(load(include_str!("test_files/smil_basics.svg")));
    let mut shape = DrawingSvgShape::new(svg.clone(), crate::RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingSvgAspect::None);
    //Act
    let at_half = shape.draw_list(0.5);
    shape.play_document_animations = false;
    let still = shape.draw_list(0.5);
    //Assert: the spinner (fill #00ff00) turned 90 degrees about (50,50) at 0.5 s.
    let green = |l: &SvgDrawList| l.commands.iter().find_map(|c| if let SvgDrawCommand::Fill { transform, paint: SvgPaint::Solid(col), .. } = c { if *col == crate::Color::from_rgb(0, 255, 0) { Some(*transform) } else { None } } else { None }).unwrap();
    assert!(maps(green(&at_half), (40.0, 40.0), (60.0, 40.0)));
    assert!(maps(green(&still), (40.0, 40.0), (40.0, 40.0)));
}

#[test]
fn an_element_loaded_transparent_is_drawn_once_its_opacity_animates_up() {
    //Arrange
    let svg = std::sync::Arc::new(load(include_str!("test_files/smil_basics.svg")));
    let mut shape = DrawingSvgShape::new(svg, crate::RectangleF::new(0.0, 0.0, 100.0, 100.0), DrawingSvgAspect::None);
    let red = |l: &SvgDrawList| l.commands.iter().any(|c| matches!(c, SvgDrawCommand::Fill { paint: SvgPaint::Solid(col), .. } if *col == crate::Color::from_rgb(255, 0, 0)));
    //Act
    let before = shape.draw_list(0.5);
    let full = shape.draw_list(2.0);
    let half = shape.draw_list(1.5);
    //Assert
    assert!(!red(&before), "opacity 0 as loaded: nothing drawn");
    assert!(red(&full));
    assert!(!full.commands.iter().any(|c| matches!(c, SvgDrawCommand::PushGroup { alpha, .. } if *alpha < 1.0 && *alpha > 0.0)) || red(&full));
    assert!(half.commands.iter().any(|c| matches!(c, SvgDrawCommand::PushGroup { alpha, .. } if (*alpha - 0.5).abs() < 1e-4)), "drawn through a group at the animated opacity");
}

#[test]
fn animated_shapes_own_the_group_of_their_transform_and_opacity() {
    //Arrange
    let svg = load(include_str!("test_files/smil_timing.svg"));
    let styled = load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><g opacity="0.5"><rect id="r" width="5" height="5" style="fill:#000" opacity="0.4"><animate attributeName="opacity" to="1" dur="2s"/></rect></g></svg>"##);
    //Act
    let toward = svg.scene().part("toward").unwrap();
    let r = styled.scene().part("r").unwrap();
    //Assert
    let own = &svg.scene().nodes()[toward.owners[0]];
    assert!(matches!(&own.kind, SvgNodeKind::Group(g) if g.opacity == 0.2 && !g.isolate), "{own:?}");
    assert_eq!(r.base.opacity, 0.4, "the rect's own opacity, not its parent group's");
    assert!((at(&styled, "r", 1.0).opacity.unwrap() - 0.7).abs() < 1e-4);
}

#[test]
fn restart_min_end_syncbase_key_points_display_and_mixed_begins() {
    //Arrange
    let svg = load(include_str!("test_files/smil_more.svg"));
    //Act / Assert
    assert!(near(at(&svg, "once", 0.5).opacity.unwrap(), 0.5));
    assert_eq!(at(&svg, "once", 2.5).opacity, None, "restart=never: the 2 s begin is refused");
    assert!(near(at(&svg, "atleast", 1.5).opacity.unwrap(), 0.375), "min=2s keeps it active past end=1s");
    assert!(near(at(&svg, "atleast", 5.0).opacity.unwrap(), 0.5), "frozen at the 2 s minimum");
    assert!(near(at(&svg, "stopper", 1.0).opacity.unwrap(), 0.1));
    assert!(near(at(&svg, "stopper", 3.0).opacity.unwrap(), 0.15), "ended at first.end + 0.5 s = 1.5 s");
    // keyPoints 0;1;0.5 at keyTimes 0;0.5;1 along 100 units; auto-reverse faces backwards.
    assert!(maps(at(&svg, "keyed", 0.5).motion.unwrap(), (0.0, 0.0), (50.0, 0.0)));
    assert!(maps(at(&svg, "keyed", 1.5).motion.unwrap(), (0.0, 0.0), (75.0, 0.0)));
    assert!(maps(at(&svg, "keyed", 0.5).motion.unwrap(), (1.0, 0.0), (49.0, 0.0)), "rotate=auto-reverse");
    assert!(maps(at(&svg, "keyed", 9.0).motion.unwrap(), (0.0, 0.0), (50.0, 0.0)), "frozen at the last key point");
    assert_eq!(at(&svg, "flicker", 1.5).visible, Some(false));
    assert_eq!(at(&svg, "flicker", 2.5).visible, None);
    assert_eq!(at(&svg, "mixed", 1.0).opacity, None);
    assert!(near(at(&svg, "mixed", 3.5).opacity.unwrap(), 0.5), "the time-based begin of a mixed list plays");
    assert!(svg.report().iter().any(|l| l.contains("mixed.click")));
    assert_eq!(at(&svg, "discrete", 7.9).stroke_dash_array, Some(Some(vec![1.0, 1.0])));
    assert_eq!(at(&svg, "discrete", 8.5).stroke_dash_array, Some(Some(vec![2.0, 2.0])));
    assert_eq!(at(&svg, "discrete", 9.5).stroke_dash_array, Some(Some(vec![4.0, 4.0])));
}

#[test]
fn animated_colour_on_a_group_reaches_only_paths_that_do_not_set_their_own() {
    //Arrange
    let svg = std::sync::Arc::new(load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="10"><g id="g" fill="#00ff00"><rect width="5" height="5" fill="#000"/><rect x="10" width="5" height="5"/><g style="fill:#ff0000"><rect x="20" width="5" height="5"/></g><g class="own"><rect x="30" width="5" height="5"/></g><animate attributeName="fill" from="#000000" to="#0000ff" dur="1s" fill="freeze"/></g><style>.own { fill: #ffff00 }</style></svg>"##));
    let mut shape = DrawingSvgShape::new(svg, crate::RectangleF::new(0.0, 0.0, 40.0, 10.0), DrawingSvgAspect::None);
    //Act
    let list = shape.draw_list(2.0);
    let fills: Vec<crate::Color> = list.commands.iter().filter_map(|c| if let SvgDrawCommand::Fill { paint: SvgPaint::Solid(col), .. } = c { Some(*col) } else { None }).collect();
    //Assert
    assert_eq!(fills, vec![crate::Color::from_rgb(0, 0, 0), crate::Color::from_rgb(0, 0, 255), crate::Color::from_rgb(255, 0, 0), crate::Color::from_rgb(255, 255, 0)], "own fill attribute, inherited (animated), own style, own CSS rule");
}

#[test]
fn animated_visibility_on_a_group_spares_children_that_set_their_own() {
    //Arrange
    let svg = std::sync::Arc::new(load(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><g id="g"><rect width="5" height="5"/><rect x="10" width="5" height="5" visibility="visible"/><set attributeName="visibility" to="hidden" begin="1s"/></g></svg>"##));
    let mut shape = DrawingSvgShape::new(svg, crate::RectangleF::new(0.0, 0.0, 20.0, 10.0), DrawingSvgAspect::None);
    //Act
    let before = shape.draw_list(0.5).draw_count();
    let after = shape.draw_list(2.0);
    //Assert
    assert_eq!(before, 2);
    assert_eq!(after.draw_count(), 1, "the child that sets visibility=visible stays");
    assert!(after.commands.iter().any(|c| matches!(c, SvgDrawCommand::Fill { path, .. } if kurbo::Shape::bounding_box(&**path).x0 >= 9.0)));
}

#[test]
fn report_names_an_animation_whose_target_is_inside_used_content() {
    //Arrange
    let markup = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="20" height="10"><defs><g id="wheel"><circle id="spoke" cx="5" cy="5" r="4"/></g></defs><use id="w1" xlink:href="#wheel"/><animate id="spin" href="#spoke" attributeName="opacity" from="1" to="0" dur="1s"/><style>#wheel { animation: fade 1s infinite } @keyframes fade { to { opacity: 0 } }</style></svg>"##;
    //Act
    let svg = load(markup);
    let lost: Vec<&str> = svg.warnings().iter().filter(|w| w.kind == DrawingSvgWarningKind::AnimationTargetInsideUse).map(|w| w.message.as_str()).collect();
    //Assert
    assert_eq!(lost.len(), 2, "{lost:?}");
    assert!(lost[0].contains("<animate> 'spin'") && lost[0].contains("'spoke'"), "{}", lost[0]);
    assert!(lost[1].contains("CSS animation 'fade") && lost[1].contains("'wheel'"), "{}", lost[1]);
    assert!(svg.has_part("w1"), "the <use> element's own id is a part");
}
