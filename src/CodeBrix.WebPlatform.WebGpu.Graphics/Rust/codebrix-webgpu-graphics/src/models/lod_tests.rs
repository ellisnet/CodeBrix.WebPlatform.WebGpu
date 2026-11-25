//! Tests of the level-of-detail choice (Not in C#): which levels at which distance, the fade
//! bands, the hysteresis, the quality scale, the cut-off, sets of instances, what is reported as
//! shown, and culling first. CPU only.

use glam::{Mat4, Vec3};

use crate::drawing3d::Camera;
use crate::models::lod_dither::{dither_coverage, dither_keeps, DITHER_INCOMING, DITHER_OUTGOING};
use crate::models::lod_instance::{choose_levels, follow_distance, lod_culled, lod_draws, union_bounds, LodChoice};
use crate::models::{LodInstance, LodLevel, LodState};
use crate::visibility::CullVolume;

/// Levels with these maximum distances and no instances (a `Model` needs a device).
fn lod(distances: &[f32]) -> LodInstance {
    LodInstance::new(Mat4::IDENTITY, distances.iter().map(|&d| LodLevel::new(Vec::new(), d)).collect()).unwrap()
}

#[test]
fn each_level_is_used_out_to_its_distance() {
    let levels = [10.0, 50.0, f32::INFINITY];
    let at = |d: f32| choose_levels(&levels, 0.0, 1.0, d);
    assert_eq!(at(0.0), LodState::Level(0));
    assert_eq!(at(9.99), LodState::Level(0));
    assert_eq!(at(10.0), LodState::Level(1));
    assert_eq!(at(49.0), LodState::Level(1));
    assert_eq!(at(50.0), LodState::Level(2));
    // No cut-off: the last level has no end.
    assert_eq!(at(1.0e9), LodState::Level(2));
}

#[test]
fn inside_a_fade_band_both_levels_are_drawn_with_complementary_patterns() {
    // Level 0 out to 100 m, band 0.1: from 90 m to 100 m level 0 fades out as level 1 fades in.
    let levels = [100.0, f32::INFINITY];
    let at = |d: f32| choose_levels(&levels, 0.1, 1.0, d);
    assert_eq!(at(89.9), LodState::Level(0));
    assert_eq!(at(90.0), LodState::Level(0));
    assert_eq!(at(95.0), LodState::Fading { from: 0, to: Some(1), progress: 0.5 });
    assert_eq!(at(92.5), LodState::Fading { from: 0, to: Some(1), progress: 0.25 });
    // Progress moves in 1/64 steps; at the last step the next level is whole, at the distance too.
    assert_eq!(at(99.999), LodState::Level(1));
    assert_eq!(at(100.0), LodState::Level(1));
    // The fade is monotonic and continuous (one step at a time) through the band.
    let mut last = 0.0f32;
    for i in 0..=1000 {
        let d = 90.0 + i as f32 * 0.01;
        let progress = match at(d) {
            LodState::Level(0) => 0.0,
            LodState::Level(1) => 1.0,
            LodState::Fading { progress, .. } => progress,
            other => panic!("{other:?} at {d}"),
        };
        assert!(progress >= last && progress - last <= 1.0 / 64.0 + 1e-6, "{d}: {last} -> {progress}");
        last = progress;
    }
    // The draws: level 0 going out, level 1 coming in, at the same step; every pixel is kept by
    // exactly one of the two, and coverages add up to one.
    let state = at(95.0);
    let draws = state.draws();
    assert_eq!(draws, vec![(0, DITHER_OUTGOING | 32), (1, DITHER_INCOMING | 32)]);
    for y in 0..16 {
        for x in 0..16 {
            assert!(dither_keeps(x, y, draws[0].1) != dither_keeps(x, y, draws[1].1));
        }
    }
    assert_eq!(dither_coverage(draws[0].1), state.coverage(0));
    assert_eq!(dither_coverage(draws[1].1), state.coverage(1));
    // Zero band: a hard cut.
    assert_eq!(choose_levels(&levels, 0.0, 1.0, 99.0), LodState::Level(0));
    assert_eq!(choose_levels(&levels, 0.0, 1.0, 100.0), LodState::Level(1));
}

#[test]
fn a_band_never_reaches_back_past_the_level_before() {
    // Level 1 runs only from 100 to 105 m: a 10 % band (94.5..105) is cut to 100..105, so level 1
    // starts whole at 100 m (no jump where level 0's fade ended) and fades out from there.
    let levels = [100.0, 105.0, f32::INFINITY];
    assert_eq!(choose_levels(&levels, 0.1, 1.0, 99.99), LodState::Level(1));
    assert_eq!(choose_levels(&levels, 0.1, 1.0, 100.0), LodState::Level(1));
    assert_eq!(choose_levels(&levels, 0.1, 1.0, 102.5), LodState::Fading { from: 1, to: Some(2), progress: 0.5 });
    // A band of 1 is the whole range of each level.
    assert_eq!(choose_levels(&[100.0, 200.0], 1.0, 1.0, 50.0), LodState::Fading { from: 0, to: Some(1), progress: 0.5 });
    assert_eq!(choose_levels(&[100.0, 200.0], 1.0, 1.0, 150.0), LodState::Fading { from: 1, to: None, progress: 0.5 });
    // Out-of-range settings: a band above 1 is 1, a NaN band is a hard cut.
    assert_eq!(choose_levels(&[100.0, 200.0], 7.0, 1.0, 50.0), choose_levels(&[100.0, 200.0], 1.0, 1.0, 50.0));
    assert_eq!(choose_levels(&[100.0, 200.0], f32::NAN, 1.0, 99.0), LodState::Level(0));
}

#[test]
fn the_last_level_is_cut_off_only_when_it_has_a_maximum_distance_and_then_fades_out() {
    assert_eq!(choose_levels(&[50.0, f32::INFINITY], 0.1, 1.0, 1.0e7), LodState::Level(1));
    let levels = [50.0, 500.0];
    let at = |d: f32| choose_levels(&levels, 0.1, 1.0, d);
    assert_eq!(at(400.0), LodState::Level(1));
    assert_eq!(at(475.0), LodState::Fading { from: 1, to: None, progress: 0.5 });
    assert_eq!(at(499.999), LodState::BeyondCutOff);
    assert_eq!(at(500.0), LodState::BeyondCutOff);
    assert_eq!(at(9000.0), LodState::BeyondCutOff);
    // The fade-out is one draw, going out; nothing comes in.
    assert_eq!(at(475.0).draws(), vec![(1, DITHER_OUTGOING | 32)]);
    assert!(at(500.0).draws().is_empty());
    // A hard cut-off.
    assert_eq!(choose_levels(&levels, 0.0, 1.0, 499.0), LodState::Level(1));
    assert_eq!(choose_levels(&levels, 0.0, 1.0, 500.0), LodState::BeyondCutOff);
}

#[test]
fn the_quality_scale_moves_every_distance() {
    let levels = [100.0, 400.0];
    // Twice the quality: level 0 out to 200 m, the cut-off at 800 m.
    assert_eq!(choose_levels(&levels, 0.0, 2.0, 150.0), LodState::Level(0));
    assert_eq!(choose_levels(&levels, 0.0, 2.0, 700.0), LodState::Level(1));
    assert_eq!(choose_levels(&levels, 0.0, 2.0, 800.0), LodState::BeyondCutOff);
    // Half: level 1 from 50 m. The band scales with the distance (45..50 m).
    assert_eq!(choose_levels(&levels, 0.0, 0.5, 60.0), LodState::Level(1));
    assert_eq!(choose_levels(&levels, 0.1, 0.5, 47.5), LodState::Fading { from: 0, to: Some(1), progress: 0.5 });
    // A scale that is not a positive number counts as 1.
    for bad in [0.0, -2.0, f32::NAN, f32::INFINITY] {
        assert_eq!(choose_levels(&levels, 0.0, bad, 150.0), LodState::Level(1));
    }
}

#[test]
fn the_hysteresis_holds_a_boundary_in_both_directions() {
    // A hard cut at 100 m, 2 % slack.
    let levels = [100.0, f32::INFINITY];
    let mut choice = LodChoice::default();
    let mut walk = |d: f32| choice.select(&levels, d, 0.0, 0.02, 1.0);
    // Outwards: the switch waits until the camera is 2 % past the boundary (100 / 0.98).
    assert_eq!(walk(95.0), LodState::Level(0));
    assert_eq!(walk(100.5), LodState::Level(0));
    assert_eq!(walk(102.0), LodState::Level(0));
    assert_eq!(walk(102.1), LodState::Level(1));
    // Hovering about the boundary changes nothing.
    for d in [100.4, 99.6, 101.0, 99.2, 100.0] {
        assert_eq!(walk(d), LodState::Level(1), "{d}");
    }
    // Inwards: the switch back waits until the camera is 2 % inside (100 / 1.02).
    assert_eq!(walk(98.1), LodState::Level(1));
    assert_eq!(walk(98.0), LodState::Level(0));
    for d in [99.0, 100.0, 101.5, 100.9] {
        assert_eq!(walk(d), LodState::Level(0), "{d}");
    }
    // The followed distance is the camera's on the first selection, and never further than the
    // slack from it.
    assert_eq!(follow_distance(None, 50.0, 0.02), 50.0);
    assert_eq!(follow_distance(Some(10.0), 50.0, 0.02), 49.0);
    assert_eq!(follow_distance(Some(90.0), 50.0, 0.02), 51.0);
    assert_eq!(follow_distance(Some(50.5), 50.0, 0.02), 50.5);
    // No slack (0), and out-of-range values clamped.
    assert_eq!(follow_distance(Some(50.5), 50.0, 0.0), 50.0);
    assert_eq!(follow_distance(Some(1.0), 50.0, 9.0), 25.0);
    assert_eq!(follow_distance(Some(50.5), 50.0, f32::NAN), 50.0);
}

#[test]
fn the_hysteresis_keeps_a_fade_continuous() {
    // In a band the followed distance lags by at most the slack, and moves smoothly: the
    // progress never jumps by more than a step per small camera move, in or out.
    let levels = [100.0, f32::INFINITY];
    let mut choice = LodChoice::default();
    let progress = |state: LodState| match state {
        LodState::Level(0) => 0.0,
        LodState::Level(1) => 1.0,
        LodState::Fading { progress, .. } => progress,
        other => panic!("{other:?}"),
    };
    let mut last = progress(choice.select(&levels, 85.0, 0.1, 0.02, 1.0));
    let path: Vec<f32> = (0..=300).map(|i| 85.0 + i as f32 * 0.1).chain((0..=300).map(|i| 115.0 - i as f32 * 0.1)).collect();
    for d in path {
        let now = progress(choice.select(&levels, d, 0.1, 0.02, 1.0));
        assert!((now - last).abs() <= 2.0 / 64.0, "{d}: {last} -> {now}");
        last = now;
    }
}

#[test]
fn a_level_made_of_a_set_of_instances_fades_as_one() {
    // Level 0: one canopy instance; level 1: three trees. In the band, every instance of both
    // levels is drawn, each with its level's code.
    let state = LodState::Fading { from: 0, to: Some(1), progress: 0.25 };
    let draws = lod_draws(&state, &[1, 3]);
    let out = DITHER_OUTGOING | 16;
    let incoming = DITHER_INCOMING | 16;
    assert_eq!(draws, vec![(0, 0, out), (1, 0, incoming), (1, 1, incoming), (1, 2, incoming)]);
    // One level whole: its instances only, no dither.
    assert_eq!(lod_draws(&LodState::Level(1), &[1, 3]), vec![(1, 0, 0), (1, 1, 0), (1, 2, 0)]);
    // An empty level (a gap) draws nothing; nothing for culled or beyond the cut-off.
    assert_eq!(lod_draws(&LodState::Level(0), &[0, 3]), Vec::new());
    for hidden in [LodState::Culled, LodState::BeyondCutOff, LodState::NotDrawn] {
        assert!(lod_draws(&hidden, &[1, 3]).is_empty());
    }
}

#[test]
fn the_state_says_which_levels_are_shown_and_how_far_through_a_fade() {
    let fading = LodState::Fading { from: 1, to: Some(2), progress: 0.25 };
    assert_eq!(fading.shown_levels(), vec![(1, 0.75), (2, 0.25)]);
    assert!(fading.is_shown(1) && fading.is_shown(2) && !fading.is_shown(0) && fading.is_fading());
    assert_eq!((fading.coverage(1), fading.coverage(2), fading.coverage(0)), (0.75, 0.25, 0.0));
    let cut = LodState::Fading { from: 2, to: None, progress: 0.5 };
    assert_eq!(cut.shown_levels(), vec![(2, 0.5)]);
    assert_eq!(LodState::Level(0).shown_levels(), vec![(0, 1.0)]);
    assert!(!LodState::Level(0).is_fading());
    for hidden in [LodState::Culled, LodState::BeyondCutOff, LodState::NotDrawn] {
        assert!(hidden.shown_levels().is_empty() && !hidden.is_shown(0));
    }
    // The object keeps the last state, and its shown levels are the ones handed out to animate.
    let mut object = lod(&[10.0, 20.0, 30.0]);
    assert_eq!(object.state(), LodState::NotDrawn);
    object.world = Mat4::from_translation(Vec3::new(0.0, 0.0, -19.5));
    let state = object.select(Vec3::ZERO, 1.0);
    assert_eq!(state, LodState::Fading { from: 1, to: Some(2), progress: 0.75 });
    assert_eq!(object.state(), state);
    assert_eq!(object.shown_levels_mut().count(), 2);
    // Moved far away: beyond the cut-off, nothing to animate.
    object.world = Mat4::from_translation(Vec3::new(0.0, 0.0, -100.0));
    assert_eq!(object.select(Vec3::ZERO, 1.0), LodState::BeyondCutOff);
    assert_eq!(object.shown_levels_mut().count(), 0);
}

#[test]
fn an_object_outside_every_view_is_left_out_before_its_distance_is_measured() {
    // The default camera at z = 5 looking at -Z; an object's bounds a 2 m cube round its origin.
    let view = CullVolume::from_camera(&Camera::default(), 1.0);
    let bounds = Some((Vec3::splat(-1.0), Vec3::splat(1.0)));
    let at = |x: f32, z: f32| Mat4::from_translation(Vec3::new(x, 0.0, z));
    assert!(!lod_culled(&view, &[], true, false, bounds, at(0.0, 0.0)));
    assert!(lod_culled(&view, &[], true, false, bounds, at(0.0, 20.0)));
    assert!(lod_culled(&view, &[], true, false, bounds, at(80.0, 0.0)));
    // Never cull, culling off.
    assert!(!lod_culled(&view, &[], true, true, bounds, at(80.0, 0.0)));
    assert!(!lod_culled(&view, &[], false, false, bounds, at(80.0, 0.0)));
    // Behind the camera but inside the sun's shadow region: kept, it may cast into view.
    let sun = crate::lighting::shadows::sun_shadow_view(Vec3::NEG_Y, Vec3::splat(-30.0), Vec3::splat(30.0), 2048).unwrap();
    assert!(!lod_culled(&view, &[&sun.volume], true, false, bounds, at(0.0, 20.0)));
    // No instance at all: nothing to draw.
    assert!(lod_culled(&view, &[], false, true, None, at(0.0, 0.0)));
    // A culled object keeps the distance it last followed (none was measured) and says Culled.
    let mut object = lod(&[100.0, f32::INFINITY]);
    object.world = at(0.0, -50.0);
    object.select(Vec3::ZERO, 1.0);
    object.mark_culled();
    assert_eq!(object.state(), LodState::Culled);
    assert_eq!(object.shown_levels_mut().count(), 0);
}

#[test]
fn the_bounds_of_the_levels_are_the_union_of_their_placed_boxes() {
    let unit = (Vec3::splat(-0.5), Vec3::splat(0.5));
    let boxes = [(unit, Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0))), (unit, Mat4::from_scale(Vec3::splat(4.0))), ((Vec3::ONE, Vec3::ZERO), Mat4::IDENTITY)];
    assert_eq!(union_bounds(boxes.into_iter()), Some((Vec3::new(-2.0, -2.0, -2.0), Vec3::new(10.5, 2.0, 2.0))));
    assert_eq!(union_bounds(std::iter::empty()), None);
    // An object without instances has no bounds.
    assert_eq!(lod(&[10.0]).bounds(), None);
}

#[test]
fn levels_must_have_growing_distances_and_only_the_last_may_have_no_end() {
    let make = |distances: &[f32]| LodInstance::new(Mat4::IDENTITY, distances.iter().map(|&d| LodLevel::new(Vec::new(), d)).collect());
    assert!(make(&[]).is_err());
    assert!(make(&[0.0]).is_err());
    assert!(make(&[f32::NAN]).is_err());
    assert!(make(&[10.0, 10.0]).is_err());
    assert!(make(&[20.0, 10.0]).is_err());
    assert!(make(&[f32::INFINITY, 10.0]).is_err());
    assert!(make(&[10.0, f32::INFINITY]).is_ok());
    assert!(make(&[10.0, 20.0]).is_ok());
    let mut object = lod(&[10.0, 20.0]);
    assert!(object.set_max_distance(0, 25.0).is_err());
    assert!(object.set_max_distance(5, 25.0).is_err());
    assert_eq!(object.levels()[0].max_distance(), 10.0);
    object.set_max_distance(1, f32::INFINITY).unwrap();
    assert_eq!(object.levels()[1].max_distance(), f32::INFINITY);
    // The defaults.
    assert_eq!((object.fade_band, object.hysteresis, object.never_cull), (LodInstance::DEFAULT_FADE_BAND, LodInstance::DEFAULT_HYSTERESIS, false));
}
