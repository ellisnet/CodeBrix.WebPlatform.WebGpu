//! Host tests for the animation vocabulary. The expected values were computed from NoSkia's own
//! formulas (Animation/Easing.cs, CubicBezierEasing.cs, ColorRamp.cs, AnimationTimeline.cs,
//! PlaybackController.cs of CodeBrix.Imaging.Drawing), evaluated independently in double
//! precision; transcendental functions may differ in the last bits, so curves compare within
//! 1e-12.

use super::*;
use crate::color::Color;
use crate::fills::ColorStop;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12
}

#[test]
fn every_curve_matches_noskia_formulas() {
    const TS: [f64; 5] = [0.1, 0.25, 0.5, 0.7, 0.9];
    const EXPECTED: [(&str, fn(f64) -> f64, [f64; 5]); 33] = [
        ("linear", Easing::linear, [0.1, 0.25, 0.5, 0.7, 0.9]),
        ("smooth_step", Easing::smooth_step, [0.028000000000000004, 0.15625, 0.5, 0.7839999999999999, 0.972]),
        ("smoother_step", Easing::smoother_step, [0.008560000000000003, 0.103515625, 0.5, 0.8369199999999999, 0.9914399999999998]),
        ("quad_in", Easing::quad_in, [0.010000000000000002, 0.0625, 0.25, 0.48999999999999994, 0.81]),
        ("quad_out", Easing::quad_out, [0.18999999999999995, 0.4375, 0.75, 0.9099999999999999, 0.99]),
        ("quad_in_out", Easing::quad_in_out, [0.020000000000000004, 0.125, 0.5, 0.82, 0.98]),
        ("cubic_in", Easing::cubic_in, [0.0010000000000000002, 0.015625, 0.125, 0.3429999999999999, 0.7290000000000001]),
        ("cubic_out", Easing::cubic_out, [0.2709999999999999, 0.578125, 0.875, 0.973, 0.999]),
        ("cubic_in_out", Easing::cubic_in_out, [0.004000000000000001, 0.0625, 0.5, 0.8919999999999999, 0.996]),
        ("quart_in", Easing::quart_in, [0.00010000000000000002, 0.00390625, 0.0625, 0.24009999999999995, 0.6561]),
        ("quart_out", Easing::quart_out, [0.3439, 0.68359375, 0.9375, 0.9919, 0.9999]),
        ("quart_in_out", Easing::quart_in_out, [0.0008000000000000001, 0.03125, 0.5, 0.9351999999999999, 0.9992]),
        ("quint_in", Easing::quint_in, [1.0000000000000003e-05, 0.0009765625, 0.03125, 0.16806999999999994, 0.5904900000000001]),
        ("quint_out", Easing::quint_out, [0.40950999999999993, 0.7626953125, 0.96875, 0.99757, 0.99999]),
        ("quint_in_out", Easing::quint_in_out, [0.00016000000000000004, 0.015625, 0.5, 0.96112, 0.99984]),
        ("sine_in", Easing::sine_in, [0.01231165940486223, 0.07612046748871326, 0.2928932188134524, 0.5460095002604533, 0.843565534959769]),
        ("sine_out", Easing::sine_out, [0.15643446504023087, 0.3826834323650898, 0.7071067811865475, 0.8910065241883678, 0.9876883405951378]),
        ("sine_in_out", Easing::sine_in_out, [0.024471741852423234, 0.1464466094067262, 0.49999999999999994, 0.7938926261462365, 0.9755282581475768]),
        ("expo_in", Easing::expo_in, [0.001953125, 0.005524271728019903, 0.03125, 0.125, 0.5]),
        ("expo_out", Easing::expo_out, [0.5, 0.8232233047033631, 0.96875, 0.9921875, 0.998046875]),
        ("expo_in_out", Easing::expo_in_out, [0.001953125, 0.015625, 0.5, 0.96875, 0.998046875]),
        ("circ_in", Easing::circ_in, [0.005012562893380035, 0.031754163448145745, 0.1339745962155614, 0.285857157145715, 0.5641101056459328]),
        ("circ_out", Easing::circ_out, [0.4358898943540673, 0.6614378277661477, 0.8660254037844386, 0.9539392014169457, 0.99498743710662]),
        ("circ_in_out", Easing::circ_in_out, [0.010102051443364402, 0.0669872981077807, 0.5, 0.8999999999999999, 0.9898979485566356]),
        ("back_in", Easing::back_in, [-0.014314220000000004, -0.06413656250000001, -0.08769750000000004, 0.09286773999999987, 0.5911720200000001]),
        ("back_out", Easing::back_out, [0.40882797999999987, 0.8174096875000002, 1.0876975, 1.08019954, 1.01431422]),
        ("back_in_out", Easing::back_in_out, [-0.037518552000000004, -0.09968184375, 0.5, 1.078833484, 1.0375185519999999]),
        ("elastic_in", Easing::elastic_in, [0.001953125, -0.005524271728019903, -0.015625000000000045, 0.125, -0.24999999999999986]),
        ("elastic_out", Easing::elastic_out, [1.25, 0.9116116523516816, 1.015625, 1.00390625, 0.998046875]),
        ("elastic_in_out", Easing::elastic_in_out, [0.000339156597005722, 0.011969444423734044, 0.5, 0.9760611111525319, 0.9996608434029943]),
        ("bounce_in", Easing::bounce_in, [0.01187500000000008, 0.02734375, 0.234375, 0.31937499999999985, 0.9243750000000001]),
        ("bounce_out", Easing::bounce_out, [0.07562500000000001, 0.47265625, 0.765625, 0.930625, 0.9881249999999999]),
        ("bounce_in_out", Easing::bounce_in_out, [0.030000000000000027, 0.1171875, 0.5, 0.9550000000000001, 0.97]),
    ];
    for (name, curve, values) in EXPECTED {
        for (t, expected) in TS.iter().zip(values) {
            let actual = curve(*t);
            assert!(close(actual, expected), "{name}({t}) = {actual}, NoSkia's formula gives {expected}");
        }
    }
}

#[test]
fn every_kind_reaches_its_ends_and_clamps() {
    for kind in EasingKind::ALL {
        assert!(close(Easing::evaluate(kind, 0.0), 0.0), "{} at 0", kind.name());
        assert!(close(Easing::evaluate(kind, 1.0), 1.0), "{} at 1", kind.name());
        assert_eq!(Easing::evaluate(kind, -3.0), Easing::evaluate(kind, 0.0));
        assert_eq!(Easing::evaluate(kind, 7.0), Easing::evaluate(kind, 1.0));
        assert_eq!(Easing::evaluate(kind, f64::NAN), Easing::evaluate(kind, 0.0));
    }
}

#[test]
fn get_function_and_evaluate_name_the_same_curve() {
    assert_eq!(Easing::evaluate(EasingKind::BounceOut, 0.5), Easing::bounce_out(0.5));
    assert_eq!((Easing::get_function(EasingKind::ElasticInOut))(0.3), Easing::elastic_in_out(0.3));
    assert_eq!(EasingKind::QuadInOut as i32, 12);
    assert_eq!(EasingKind::BounceInOut as i32, 102);
    assert_eq!(EasingKind::ALL.len(), 33);
}

#[test]
fn steps_follow_css_jump_terms() {
    // steps(4, End) at 0.3: floor(1.2) = 1 -> 1/4; Start: 2/4; None (3 jumps): 1/3; steps(3, Both) (4 jumps): floor(0.9) + 1 = 1 -> 1/4.
    assert_eq!(Easing::steps(0.3, 4, StepPosition::End), Some(0.25));
    assert_eq!(Easing::steps(0.3, 4, StepPosition::Start), Some(0.5));
    assert_eq!(Easing::steps(0.3, 4, StepPosition::None), Some(1.0 / 3.0));
    assert_eq!(Easing::steps(0.3, 3, StepPosition::Both), Some(0.25));
    assert_eq!(Easing::steps(1.0, 4, StepPosition::End), Some(1.0));
    assert_eq!(Easing::steps(0.5, 0, StepPosition::End), None);
    assert_eq!(Easing::steps(0.5, 1, StepPosition::None), None);
}

#[test]
fn css_keyword_curves_match_noskia_solver() {
    let ts = [0.1, 0.25, 0.5, 0.7, 0.9];
    let ease = [0.09479630571510597, 0.40851059301560033, 0.8024033876954126, 0.9407646142979404, 0.9943164775095211];
    let ease_in_out = [0.01972244726385564, 0.12916190056878776, 0.5, 0.8126041088420908, 0.9802775527361445];
    for i in 0..5 {
        assert!(close(CubicBezierEasing::EASE.evaluate(ts[i]), ease[i]), "ease({})", ts[i]);
        assert!(close(CubicBezierEasing::EASE_IN_OUT.evaluate(ts[i]), ease_in_out[i]), "ease-in-out({})", ts[i]);
    }
    assert!(CubicBezierEasing::new(1.5, 0.0, 0.5, 1.0).is_none());
    assert!(CubicBezierEasing::new(0.5, f64::INFINITY, 0.5, 1.0).is_none());
    assert_eq!(CubicBezierEasing::EASE.to_string(), "cubic-bezier(0.25, 0.1, 0.25, 1)");
    assert_eq!(Easing::cubic_bezier(0.5, 0.42, 0.0, 0.58, 1.0), Some(0.5));
}

#[test]
fn lerp_is_unclamped() {
    assert_eq!(Easing::lerp(10.0, 20.0, 1.5), 25.0);
    assert_eq!(Easing::lerp_f32(0.0, 8.0, 0.25), 2.0);
    assert_eq!(Easing::smooth_step_between(2.0, 2.0, 1.0), None);
    assert_eq!(Easing::smooth_step_between(0.0, 2.0, 1.0), Some(0.5));
}

#[test]
fn color_ramps_match_noskia_rounding() {
    let venus = ColorRamp::from_colors(&[Color::from_rgb(246, 112, 168), Color::from_rgb(232, 146, 92), Color::from_rgb(156, 122, 248)]).unwrap();
    let sunset = ColorRamp::from_colors(&[Color::from_rgb(20, 24, 82), Color::from_rgb(120, 40, 140), Color::from_rgb(232, 72, 96), Color::from_rgb(255, 160, 64), Color::from_rgb(255, 236, 160)]).unwrap();
    let cases: [(f64, [u8; 4], [u8; 4]); 6] = [
        (0.0, [246, 112, 168, 255], [20, 24, 82, 255]),
        (0.2, [240, 126, 138, 255], [100, 37, 128, 255]),
        (0.37, [236, 137, 112, 255], [174, 55, 119, 255]),
        (0.5, [232, 146, 92, 255], [232, 72, 96, 255]),
        (0.81, [185, 131, 189, 255], [255, 178, 87, 255]),
        (1.0, [156, 122, 248, 255], [255, 236, 160, 255]),
    ];
    for (t, v, s) in cases {
        assert_eq!(venus.at(t).to_rgba32(), v, "venus at {t}");
        assert_eq!(sunset.at(t).to_rgba32(), s, "sunset at {t}");
    }
    assert_eq!(venus.at_with_alpha(0.5, 7).to_rgba32(), [232, 146, 92, 7]);
    assert_eq!(venus.stop_count(), 3);
}

#[test]
fn a_shared_stop_position_resolves_to_the_later_stop() {
    let red = Color::from_rgb(230, 60, 60);
    let yellow = Color::from_rgb(240, 200, 40);
    let ramp = ColorRamp::new(&[ColorStop::new(0.0, red).unwrap(), ColorStop::new(0.5, red).unwrap(), ColorStop::new(0.5, yellow).unwrap(), ColorStop::new(1.0, yellow).unwrap()]).unwrap();
    assert_eq!(ramp.at(0.49), red);
    assert_eq!(ramp.at(0.5), yellow);
    assert!(ColorRamp::new(&[ColorStop::new(0.0, red).unwrap()]).is_err());
    assert!(ColorRamp::new(&[ColorStop::new(0.6, red).unwrap(), ColorStop::new(0.2, red).unwrap()]).is_err());
    assert!(ColorStop::new(1.5, red).is_none());
}

#[test]
fn timeline_frames_and_timecodes_follow_noskia() {
    let timeline = AnimationTimeline::new(396.0, 30.0).unwrap();
    assert_eq!(timeline.frame_count(), 11880);
    assert_eq!(timeline.frame_at(1.0), 30);
    assert_eq!(timeline.frame_at(0.9999999999), 30);
    assert_eq!(timeline.frame_at(1e9), 11879);
    assert_eq!(timeline.format_timecode(3661.5), "01:01:01:15");
    assert_eq!(AnimationTimeline::new(10.0, 120.0).unwrap().format_timecode(1.0 / 120.0 * 7.0), "00:00:00:007");
    assert!(AnimationTimeline::new(0.0, 30.0).is_err());
}

#[test]
fn cues_progress_and_activity() {
    let mut timeline = AnimationTimeline::new(10.0, 30.0).unwrap();
    timeline.add_cue("move", 2.0, 4.0, EasingKind::QuadIn).unwrap();
    timeline.add_marker("hit", 5.0).unwrap();
    assert!(timeline.add_marker("hit", 6.0).is_err());
    assert_eq!(timeline.progress("move", 3.0), Some(0.25));
    assert_eq!(timeline.progress("hit", 4.9), Some(0.0));
    assert_eq!(timeline.progress("hit", 5.0), Some(1.0));
    assert_eq!(timeline.progress("nothing", 5.0), None);
    let names: Vec<&str> = timeline.get_active_cues(5.01).iter().map(|c| c.name()).collect();
    assert_eq!(names, vec!["hit"]);
    let custom = TimelineCue::with_curve("x", 0.0, 2.0, |t| t * t).unwrap();
    assert!(custom.has_custom_curve() && close(custom.progress(1.0), 0.25));
    assert_eq!(custom.to_string(), "x [0, 2)");
}

#[test]
fn a_marker_can_come_from_a_named_point_of_the_host_clock() {
    let clock = codebrix_webgpu::CanvasClock::new();
    clock.define_point("title in", 1.5);
    let mut timeline = AnimationTimeline::new(10.0, 30.0).unwrap();
    assert_eq!(timeline.add_marker_at_point("title", &clock.points(), "title in").unwrap().start_time(), 1.5);
    assert!(timeline.add_marker_at_point("other", &clock.points(), "missing").unwrap_err().contains("define_point"));
}

#[test]
fn playback_advances_only_by_the_time_it_is_given() {
    let mut playback = PlaybackController::with_duration(2.0, 30.0).unwrap();
    assert!(!playback.advance(0.05));
    playback.play();
    assert!(!playback.advance(0.05));
    assert!(close(playback.time(), 0.05));
    // A slow frame is capped at the largest step.
    playback.advance(1.0);
    assert!(close(playback.time(), 0.15));
    // A paused host clock hands a delta of 0: nothing moves.
    assert!(!playback.advance(0.0));
    assert!(close(playback.time(), 0.15));
    playback.seek(1.95);
    assert!(playback.advance(0.1));
    assert_eq!(playback.time(), 2.0);
    assert!(!playback.is_playing());
    playback.play();
    assert_eq!(playback.time(), 0.0);
    playback.set_is_looping(true);
    playback.seek(1.95);
    assert!(playback.advance(0.1));
    assert!(close(playback.time(), 0.05));
    assert!(playback.set_speed(-1.0));
    playback.advance(0.1);
    assert!(close(playback.time(), 1.95));
    assert!(!playback.set_speed(f64::NAN));
    playback.step(3);
    assert!(!playback.is_playing());
    assert_eq!(playback.frame(), 59);
    playback.restart();
    assert!(playback.is_playing() && playback.time() == 0.0);
}
