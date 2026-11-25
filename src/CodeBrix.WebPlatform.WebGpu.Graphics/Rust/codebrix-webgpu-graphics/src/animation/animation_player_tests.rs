//! The host tests of [`AnimationPlayer`]: C#'s AnimationPlayerTests with its model and its values
//! (the C# tests build the model as a glTF file and load it on a GPU; here the same nodes and
//! clips are made as model data, since the player needs no GPU), plus the frame-time form.

use glam::{Mat4, Quat, Vec3};

use super::{AnimationPlayer, PlayOptions, PlayerData, PoseSource, PoseTarget};
use crate::animation::AnimationLoopMode;
use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel, ModelNode, NodeTransform};

const QUAD: usize = 0;
const MOVER: usize = 1;
const SPINNER: usize = 2;

fn turned() -> Quat {
    Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)
}

fn node(index: usize, name: &str, parent: Option<usize>, translation: Vec3, weights: Vec<f32>) -> ModelNode {
    let rest = NodeTransform::new(translation, Quat::IDENTITY, Vec3::ONE);
    ModelNode { index, name: Some(name.to_string()), parent, children: Vec::new(), mesh: None, skin: None, default_transform: rest, has_fixed_matrix: false, local_matrix: rest.to_matrix(), default_morph_weights: weights, in_scene: true }
}

fn channel(node: usize, path: AnimationPath, interpolation: AnimationInterpolation, times: &[f32], values: &[f32], components: usize) -> ModelAnimationChannel {
    ModelAnimationChannel { node, path, interpolation, times: times.to_vec(), values: values.to_vec(), component_count: components }
}

fn clip(index: usize, name: &str, channels: Vec<ModelAnimationChannel>) -> ModelAnimation {
    ModelAnimation { index, name: Some(name.to_string()), channels }
}

fn q(value: Quat) -> [f32; 4] {
    [value.x, value.y, value.z, value.w]
}

/// C#'s test model: a quad with two morph targets (node 0), a "mover" (node 1) and its child
/// "spinner" (node 2, rest translation (0, 1, 0)), with clips slide (mover x 0 -> 4 over 2 s),
/// lift (mover y 0 -> 2 and spinner scale 1 -> 3 over 1 s), turn (spinner 0 -> 90 degrees about
/// Y over 1 s), still (spinner held at identity), turnNegated (spinner held at -q(90 degrees)),
/// steps (STEP mover x 0, 1, 2 at 0, 0.5, 1 s), curve (CUBICSPLINE mover x = t^3 at keys 0 and
/// 2 s) and blink (quad weights [0, 0] -> [1, 0.5] over 1 s).
struct TestInstance {
    data: PlayerData,
    nodes: Vec<ModelNode>,
    animations: Vec<ModelAnimation>,
    offsets: Vec<Option<usize>>,
    locals: Vec<NodeTransform>,
    weights: Vec<f32>,
    posed: bool,
    version: u64,
}

impl TestInstance {
    fn new() -> TestInstance {
        use AnimationInterpolation::{CubicSpline, Linear, Step};
        use AnimationPath::{Rotation, Scale, Translation, Weights};
        let nodes = vec![node(0, "quad", None, Vec3::ZERO, vec![0.0, 0.0]), node(1, "mover", None, Vec3::ZERO, Vec::new()), node(2, "spinner", Some(1), Vec3::Y, Vec::new())];
        let mut turn = q(Quat::IDENTITY).to_vec();
        turn.extend_from_slice(&q(turned()));
        let mut negated = q(-turned()).to_vec();
        negated.extend_from_slice(&q(-turned()));
        let mut still = q(Quat::IDENTITY).to_vec();
        still.extend_from_slice(&q(Quat::IDENTITY));
        let mut curve = vec![0.0f32; 9];
        curve.extend_from_slice(&[12.0, 0.0, 0.0, 8.0, 0.0, 0.0, 12.0, 0.0, 0.0]);
        let animations = vec![
            clip(0, "slide", vec![channel(MOVER, Translation, Linear, &[0.0, 2.0], &[0.0, 0.0, 0.0, 4.0, 0.0, 0.0], 3)]),
            clip(1, "lift", vec![channel(MOVER, Translation, Linear, &[0.0, 1.0], &[0.0, 0.0, 0.0, 0.0, 2.0, 0.0], 3), channel(SPINNER, Scale, Linear, &[0.0, 1.0], &[1.0, 1.0, 1.0, 3.0, 3.0, 3.0], 3)]),
            clip(2, "turn", vec![channel(SPINNER, Rotation, Linear, &[0.0, 1.0], &turn, 4)]),
            clip(3, "still", vec![channel(SPINNER, Rotation, Linear, &[0.0, 1.0], &still, 4)]),
            clip(4, "turnNegated", vec![channel(SPINNER, Rotation, Linear, &[0.0, 1.0], &negated, 4)]),
            clip(5, "steps", vec![channel(MOVER, Translation, Step, &[0.0, 0.5, 1.0], &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0], 3)]),
            clip(6, "curve", vec![channel(MOVER, Translation, CubicSpline, &[0.0, 2.0], &curve, 3)]),
            clip(7, "blink", vec![channel(QUAD, Weights, Linear, &[0.0, 1.0], &[0.0, 0.0, 1.0, 0.5], 2)]),
        ];
        let locals = nodes.iter().map(|n| n.default_transform).collect();
        TestInstance { data: PlayerData::new(&animations), nodes, animations, offsets: vec![Some(0), None, None], locals, weights: vec![0.0, 0.0], posed: true, version: 0 }
    }

    fn player(&mut self) -> AnimationPlayer<'_> {
        let source = PoseSource { nodes: &self.nodes, animations: &self.animations, morph_offsets: &self.offsets, morph_count: 2 };
        let target = PoseTarget { locals: &mut self.locals, weights: &mut self.weights, posed: &mut self.posed, version: &mut self.version };
        AnimationPlayer::new(&mut self.data, source, target)
    }

    /// The spinner's matrix in model space (C#: NodeMatrices[Spinner]).
    fn spinner_matrix(&self) -> Mat4 {
        self.locals[MOVER].to_matrix() * self.locals[SPINNER].to_matrix()
    }
}

fn close(actual: Vec3, expected: Vec3) {
    assert!((actual - expected).length() < 1e-4, "expected {expected} but found {actual}");
}

fn same_rotation(actual: Quat, expected: Quat) {
    assert!((actual.dot(expected).abs() - 1.0).abs() < 1e-4, "expected {expected} but found {actual}");
}

#[test]
fn clips_play_by_name_or_index_and_unknown_names_say_what_exists() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("lift", PlayOptions::default()).unwrap();
    let slide = player.play(0, PlayOptions::default()).unwrap();
    assert_eq!(slide.name(), Some("slide"));
    assert_eq!(slide.duration(), 2.0);
    assert_eq!(player.state_count(), 1);
    assert_eq!(player.find_state(0).map(|s| s.clip_index()), Some(0));
    let error = player.play_named("missing", PlayOptions::default()).unwrap_err();
    assert!(error.message().contains("'missing'") && error.message().contains("slide, lift"), "{}", error.message());
    assert!(player.play(8, PlayOptions::default()).unwrap_err().message().contains("8 animation(s)"));
}

#[test]
fn looping_clip_poses_the_node_and_wraps() {
    let mut test = TestInstance::new();
    test.player().play_named("slide", PlayOptions::default()).unwrap();
    test.player().update(0.5);
    close(test.locals[MOVER].translation, Vec3::new(1.0, 0.0, 0.0));
    close(test.spinner_matrix().w_axis.truncate(), Vec3::new(1.0, 1.0, 0.0));
    let mut player = test.player();
    player.update(1.75);
    let state = player.get_state(0).unwrap();
    assert!((state.time() - 0.25).abs() < 1e-5);
    assert_eq!(state.loop_count(), 1);
    assert!(state.looped_this_update());
    close(test.locals[MOVER].translation, Vec3::new(0.5, 0.0, 0.0));
    close(test.spinner_matrix().w_axis.truncate(), Vec3::new(0.5, 1.0, 0.0));
}

#[test]
fn once_holds_the_last_frame_and_reports_finishing_once() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("lift", PlayOptions::looping(AnimationLoopMode::Once)).unwrap();
    let mut finished = 0;
    player.update(0.6);
    finished += player.finished_clips().len();
    let finished_early = player.get_state(0).unwrap().is_finished();
    player.update(0.6);
    finished += player.finished_clips().len();
    let finished_now = player.get_state(0).unwrap().finished_this_update();
    player.update(0.6);
    finished += player.finished_clips().len();
    let state = player.get_state(0).unwrap();
    assert!(!finished_early);
    assert!(finished_now);
    assert!(!state.finished_this_update());
    assert!(state.is_finished());
    assert_eq!(state.time(), 1.0);
    assert_eq!(finished, 1);
    close(test.locals[MOVER].translation, Vec3::new(0.0, 2.0, 0.0));
    close(test.locals[SPINNER].scale, Vec3::splat(3.0));
}

#[test]
fn ping_pong_plays_forward_then_back() {
    let mut test = TestInstance::new();
    test.player().play_named("lift", PlayOptions::looping(AnimationLoopMode::PingPong)).unwrap();
    test.player().update(1.25);
    let backwards = test.locals[MOVER].translation.y;
    let mut player = test.player();
    player.update(1.0);
    let state = player.get_state(0).unwrap();
    assert!((backwards - 1.5).abs() < 1e-4);
    assert!((state.time() - 0.25).abs() < 1e-5);
    assert_eq!(state.loop_count(), 1);
    assert!((test.locals[MOVER].translation.y - 0.5).abs() < 1e-4);
}

#[test]
fn pause_speed_seek_and_normalized_time_control_the_playhead() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("slide", PlayOptions { speed: 0.5, ..PlayOptions::default() }).unwrap();
    player.set_speed(2.0);
    player.update(0.5);
    assert!((player.get_state(0).unwrap().time() - 0.5).abs() < 1e-6);
    player.get_state_mut(0).unwrap().paused = true;
    player.update(0.5);
    assert!((player.get_state(0).unwrap().time() - 0.5).abs() < 1e-6);
    player.get_state_mut(0).unwrap().paused = false;
    player.set_paused(true);
    player.update(0.5);
    assert!((player.get_state(0).unwrap().time() - 0.5).abs() < 1e-6);
    player.set_paused(false);
    player.get_state_mut(0).unwrap().set_normalized_time(0.5);
    player.evaluate();
    assert_eq!(player.get_state(0).unwrap().time(), 1.0);
    player.get_state_mut(0).unwrap().speed = -0.5;
    player.update(0.25);
    let state = player.get_state_mut(0).unwrap();
    assert!((state.time() - 0.75).abs() < 1e-6);
    state.set_time(5.0);
    assert!((state.time() - 1.0).abs() < 1e-6, "seeking wraps a looping clip");
    state.loop_mode = AnimationLoopMode::Once;
    state.set_time(5.0);
    assert_eq!(state.time(), 2.0);
    assert!(state.is_finished());
    player.evaluate();
    close(test.locals[MOVER].translation, Vec3::new(4.0, 0.0, 0.0));
}

#[test]
fn step_and_cubic_spline_clips_give_known_answers() {
    let mut test = TestInstance::new();
    let mut x = |clip: &str, time: f32| {
        let mut player = test.player();
        if player.find_state(player_clip(clip)).is_none() {
            player.play_named(clip, PlayOptions::looping(AnimationLoopMode::Once)).unwrap();
        }
        player.get_state_mut(0).unwrap().set_time(time);
        player.evaluate();
        test.locals[MOVER].translation.x
    };
    let stepped = [x("steps", 0.49), x("steps", 0.5), x("steps", 0.99), x("steps", 1.0)];
    let curved = [x("curve", 0.5), x("curve", 1.0), x("curve", 1.5), x("curve", 2.0)];
    assert_eq!(stepped, [0.0, 1.0, 1.0, 2.0]);
    for (actual, expected) in curved.iter().zip([0.125, 1.0, 3.375, 8.0]) {
        assert!((actual - expected).abs() < 1e-4, "{actual} for {expected}");
    }
}

fn player_clip(name: &str) -> usize {
    ["slide", "lift", "turn", "still", "turnNegated", "steps", "curve", "blink"].iter().position(|n| *n == name).unwrap()
}

#[test]
fn cross_fade_weights_ramp_linearly_and_blend_the_pose() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("slide", PlayOptions::default()).unwrap();
    player.update(1.0);
    let start_weight = player.cross_fade_named("lift", 1.0, PlayOptions::looping(AnimationLoopMode::Once)).unwrap().weight();
    player.update(0.5);
    let weights = (player.get_state(0).unwrap().weight(), player.get_state(1).unwrap().weight());
    let halfway = (test.locals[MOVER].translation, test.locals[SPINNER].scale);
    let mut player = test.player();
    player.update(0.5);
    // Slide at 1.5 s is x = 3, lift at 0.5 s is y = 1: half of each; the spinner's scale (lift
    // only) blends from its rest scale by lift's weight.
    assert_eq!(start_weight, 0.0);
    assert!((weights.0 - 0.5).abs() < 1e-5 && (weights.1 - 0.5).abs() < 1e-5);
    close(halfway.0, Vec3::new(1.5, 0.5, 0.0));
    close(halfway.1, Vec3::splat(1.5));
    assert_eq!(player.state_count(), 1);
    let lift = player.get_state(0).unwrap();
    assert_eq!(lift.name(), Some("lift"));
    assert_eq!(lift.weight(), 1.0);
    assert!(!lift.is_fading());
    close(test.locals[MOVER].translation, Vec3::new(0.0, 2.0, 0.0));
    close(test.locals[SPINNER].scale, Vec3::splat(3.0));
}

#[test]
fn higher_layer_overrides_only_its_properties_by_its_weight() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("slide", PlayOptions::default()).unwrap();
    player.update(1.0);
    player.blend_named("turn", 0.5, PlayOptions { loop_mode: AnimationLoopMode::Once, speed: 1.0, start_time: 1.0, layer: 1 }).unwrap();
    player.evaluate();
    let half = test.locals[SPINNER].rotation;
    let mut player = test.player();
    player.get_state_mut(1).unwrap().set_weight(1.0);
    player.evaluate();
    close(test.locals[MOVER].translation, Vec3::new(2.0, 0.0, 0.0));
    same_rotation(half, Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4));
    same_rotation(test.locals[SPINNER].rotation, turned());
    close(test.locals[SPINNER].translation, Vec3::Y);
}

#[test]
fn weighted_rotation_blend_corrects_the_quaternion_hemisphere() {
    // The second clip stores the 90 degree turn as -q; a naive weighted sum would give 135 degrees.
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.blend_named("still", 0.5, PlayOptions::default()).unwrap();
    player.blend_named("turnNegated", 0.5, PlayOptions::default()).unwrap();
    player.evaluate();
    let even = test.locals[SPINNER].rotation;
    let mut player = test.player();
    player.get_state_mut(0).unwrap().set_weight(0.25);
    player.get_state_mut(1).unwrap().set_weight(0.75);
    player.evaluate();
    same_rotation(even, Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4));
    assert!((even.length() - 1.0).abs() < 1e-5);
    let angle = 2.0 * test.locals[SPINNER].rotation.w.abs().min(1.0).acos();
    assert!(angle > std::f32::consts::FRAC_PI_4 && angle < std::f32::consts::FRAC_PI_2);
}

#[test]
fn stopping_returns_the_animated_properties_to_the_rest_pose() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("lift", PlayOptions::default()).unwrap();
    player.update(0.5);
    let moved = test.locals[MOVER].translation;
    let mut player = test.player();
    player.stop();
    assert!(!player.is_playing());
    close(moved, Vec3::new(0.0, 1.0, 0.0));
    close(test.locals[MOVER].translation, Vec3::ZERO);
    close(test.locals[SPINNER].scale, Vec3::ONE);
    close(test.spinner_matrix().w_axis.truncate(), Vec3::Y);
}

#[test]
fn a_state_that_stops_itself_leaves_at_the_next_call() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("lift", PlayOptions::default()).unwrap();
    player.update(0.5);
    player.get_state_mut(0).unwrap().stop();
    assert_eq!(player.state_count(), 0);
    player.update(0.1);
    assert!(!player.is_playing());
    close(test.locals[MOVER].translation, Vec3::ZERO);
}

#[test]
fn morph_weight_channels_drive_the_instance_weights() {
    let mut test = TestInstance::new();
    let version = test.version;
    let mut player = test.player();
    player.play_named("blink", PlayOptions::default()).unwrap();
    player.update(0.5);
    assert_eq!(test.weights, vec![0.5, 0.25]);
    assert!(test.version > version);
    assert!(!test.posed, "the node matrices and palettes are recomputed on their next read");
}

#[test]
fn a_finished_clip_can_start_the_next_one() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    player.play_named("lift", PlayOptions::looping(AnimationLoopMode::Once)).unwrap();
    player.update(1.5);
    if player.finished_clips() == [1] {
        player.play_named("slide", PlayOptions::default()).unwrap();
    }
    player.update(0.5);
    assert_eq!(player.state_count(), 1);
    assert_eq!(player.get_state(0).unwrap().name(), Some("slide"));
    close(test.locals[MOVER].translation, Vec3::new(1.0, 0.0, 0.0));
}

#[test]
fn the_player_refuses_a_ninth_clip_unless_one_is_fading_out() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    for _ in 0..AnimationPlayer::MAX_STATES {
        player.blend_named("slide", 0.1, PlayOptions::default()).unwrap();
    }
    let error = player.blend_named("lift", 0.1, PlayOptions::default()).unwrap_err();
    assert!(error.message().contains("stop one first"), "{}", error.message());
    player.cross_fade_named("lift", 1.0, PlayOptions::default()).unwrap();
    assert_eq!(player.state_count(), AnimationPlayer::MAX_STATES);
}

#[test]
fn update_to_follows_the_frame_time_through_pause_speed_and_seeks() {
    // The clock's time, as a host would hand it: running, paused (the same time again), double
    // speed, a seek back and a seek forward. A looping clip lands where its time says.
    let mut test = TestInstance::new();
    test.player().play_named("slide", PlayOptions { start_time: 0.25, ..PlayOptions::default() }).unwrap();
    let mut x_at = |time: f64| {
        test.player().update_to(time);
        test.locals[MOVER].translation.x
    };
    let expected = |time: f64| (((time + 0.25) % 2.0) * 2.0) as f32;
    let first = x_at(10.0);
    assert!((first - 0.5).abs() < 1e-4, "the first call only records the time: the start pose, {first}");
    assert!((x_at(10.5) - expected(0.5)).abs() < 1e-4);
    let paused = x_at(10.5);
    assert!((x_at(10.5) - paused).abs() < 1e-6, "the same time again changes nothing");
    assert!((x_at(11.25) - expected(1.25)).abs() < 1e-4);
    assert!((x_at(10.1) - expected(0.1)).abs() < 1e-4, "a seek back plays the loop back to the same place");
    assert!((x_at(17.6) - expected(7.6)).abs() < 1e-3, "a seek forward wraps the loop");
}

#[test]
fn the_clip_limit_is_per_player_and_defaults_to_eight() {
    let mut test = TestInstance::new();
    let mut player = test.player();
    assert_eq!(player.max_states(), AnimationPlayer::MAX_STATES);
    assert_eq!(AnimationPlayer::MAX_STATES, 8);
    player.set_max_states(12);
    for _ in 0..12 {
        player.blend_named("slide", 0.1, PlayOptions::default()).unwrap();
    }
    let error = player.blend_named("lift", 0.1, PlayOptions::default()).unwrap_err();
    assert!(error.message().contains("its limit is 12"), "{}", error.message());
    // A lower limit stops nothing that plays; the next start is held to it.
    player.set_max_states(2);
    assert_eq!(player.state_count(), 12);
    assert!(player.blend_named("lift", 0.1, PlayOptions::default()).is_err());
    player.stop();
    player.set_max_states(0);
    assert_eq!(player.max_states(), 1);
    player.play_named("slide", PlayOptions::default()).unwrap();
    assert!(player.blend_named("lift", 0.5, PlayOptions::default()).is_err());
    // A cross-fade makes room by fading the playing clip out, even at a limit of 1.
    player.cross_fade_named("lift", 1.0, PlayOptions::default()).unwrap();
    assert_eq!(player.state_count(), 1);
    assert_eq!(player.get_state(0).unwrap().name(), Some("lift"));
}

#[test]
fn update_to_since_zero_poses_the_clips_as_if_playing_since_time_zero() {
    // A page opened at t = 10.5: the first call already shows the pose of 10.5 s of playing.
    let mut test = TestInstance::new();
    test.player().play_named("slide", PlayOptions { start_time: 0.25, ..PlayOptions::default() }).unwrap();
    let expected = |time: f64| (((time + 0.25) % 2.0) * 2.0) as f32;
    test.player().update_to_since_zero(10.5);
    assert!((test.locals[MOVER].translation.x - expected(10.5)).abs() < 1e-4, "{}", test.locals[MOVER].translation.x);
    // Later calls advance by the difference, as update_to does.
    test.player().update_to_since_zero(11.0);
    assert!((test.locals[MOVER].translation.x - expected(11.0)).abs() < 1e-4);
    test.player().update_to(11.25);
    assert!((test.locals[MOVER].translation.x - expected(11.25)).abs() < 1e-4);

    // A once-clip is finished and reported by the first call; update_to's first call is unchanged.
    let mut once = TestInstance::new();
    once.player().play_named("lift", PlayOptions::looping(AnimationLoopMode::Once)).unwrap();
    once.player().update_to_since_zero(12.0);
    assert_eq!(once.player().finished_clips(), &[player_clip("lift")]);
    close(once.locals[MOVER].translation, Vec3::new(0.0, 2.0, 0.0));
    let mut baseline = TestInstance::new();
    baseline.player().play_named("lift", PlayOptions::looping(AnimationLoopMode::Once)).unwrap();
    baseline.player().update_to(12.0);
    close(baseline.locals[MOVER].translation, Vec3::ZERO);
}
