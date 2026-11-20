//! Evaluating one [`ModelAnimationChannel`] at a time (C#: the internal `AnimationSampling`,
//! glTF 2.0 Appendix C): STEP holds the key value, LINEAR interpolates (rotations by spherical
//! linear interpolation on the shorter arc), CUBICSPLINE is the Hermite spline with its tangents
//! scaled by the key interval (rotations normalised). Times before the first key give the first
//! value, times after the last key the last value. The key lookup uses a cursor the caller keeps
//! (the interval found last time) and falls back to a binary search, so playback that moves
//! forward costs O(1) per channel. CPU only, allocation-free, tested on the host.

use glam::Quat;

use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimationChannel};

/// The key interval holding `time`: `Some(k)` with `times[k] <= time < times[k + 1]`, None
/// before the first key, the last index at or after the last key (C#: `FindKey`, which returns -1
/// for None). `times` is ascending and holds at least one key; `cursor` is the interval found
/// last time and is updated.
pub(crate) fn find_key(times: &[f32], time: f32, cursor: &mut usize) -> Option<usize> {
    let last = times.len() - 1;
    if time < times[0] {
        *cursor = 0;
        return None;
    }
    if time >= times[last] {
        *cursor = last;
        return Some(last);
    }
    // Here 0 <= key < last. Try the cached interval and the next two before searching.
    let mut key = *cursor;
    if key < last && times[key] <= time {
        let mut step = 0;
        while step < 3 && key < last {
            if time < times[key + 1] {
                *cursor = key;
                return Some(key);
            }
            step += 1;
            key += 1;
        }
    }
    let mut low = 0;
    let mut high = last - 1;
    while low < high {
        let middle = (low + high + 1) >> 1;
        if times[middle] <= time {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    *cursor = low;
    Some(low)
}

/// Evaluates `channel` at `time` into the first `component_count` floats of `destination`
/// (which must hold that many) (C#: `Sample`). A channel without keys, or whose values are
/// shorter than its keys need, leaves `destination` unchanged (the reader refuses such data).
pub(crate) fn sample(channel: &ModelAnimationChannel, time: f32, cursor: &mut usize, destination: &mut [f32]) {
    let times = channel.times.as_slice();
    let values = channel.values.as_slice();
    let components = channel.component_count;
    let cubic = channel.interpolation == AnimationInterpolation::CubicSpline;
    let stride = if cubic { components * 3 } else { components };
    if times.is_empty() || components == 0 || values.len() < times.len() * stride || destination.len() < components {
        return;
    }
    let value_offset = if cubic { components } else { 0 };
    let key = find_key(times, time, cursor);
    let last = times.len() - 1;
    let key = match key {
        Some(key) if key < last && channel.interpolation != AnimationInterpolation::Step => key,
        other => {
            let index = other.unwrap_or(0);
            let start = index * stride + value_offset;
            destination[..components].copy_from_slice(&values[start..start + components]);
            return;
        }
    };
    let start = times[key];
    let interval = times[key + 1] - start;
    let t = if interval > 0.0 { (time - start) / interval } else { 0.0 };
    let rotation = channel.path == AnimationPath::Rotation;
    if !cubic {
        let a = &values[key * components..(key + 1) * components];
        let b = &values[(key + 1) * components..(key + 2) * components];
        if rotation && components == 4 {
            let q = slerp(Quat::from_xyzw(a[0], a[1], a[2], a[3]), Quat::from_xyzw(b[0], b[1], b[2], b[3]), t);
            destination[0] = q.x;
            destination[1] = q.y;
            destination[2] = q.z;
            destination[3] = q.w;
            return;
        }
        for component in 0..components {
            destination[component] = a[component] + (b[component] - a[component]) * t;
        }
        return;
    }
    // Hermite: p(t) = (2t^3 - 3t^2 + 1) v_k + td (t^3 - 2t^2 + t) b_k + (-2t^3 + 3t^2) v_k+1
    // + td (t^3 - t^2) a_k+1, where b_k is key k's out-tangent and a_k+1 key k+1's in-tangent
    // (glTF layout per key: in, value, out).
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = (t3 - 2.0 * t2 + t) * interval;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = (t3 - t2) * interval;
    let first = key * stride;
    let next = (key + 1) * stride;
    for component in 0..components {
        let v0 = values[first + components + component];
        let out0 = values[first + 2 * components + component];
        let in1 = values[next + component];
        let v1 = values[next + components + component];
        destination[component] = h00 * v0 + h10 * out0 + h01 * v1 + h11 * in1;
    }
    if rotation && components == 4 {
        normalize_quaternion(&mut destination[..4]);
    }
}

/// Spherical linear interpolation on the shorter arc; normalised linear interpolation when the
/// keys are nearly equal (C#: `Slerp`).
pub(crate) fn slerp(a: Quat, b: Quat, t: f32) -> Quat {
    let mut b = b;
    let mut dot = a.dot(b);
    if dot < 0.0 {
        b = -b;
        dot = -dot;
    }
    let (wa, wb) = if dot > 0.9995 {
        (1.0 - t, t)
    } else {
        let angle = dot.acos();
        let sin = angle.sin();
        (((1.0 - t) * angle).sin() / sin, (t * angle).sin() / sin)
    };
    (a * wa + b * wb).normalize()
}

/// Normalised linear interpolation on the shorter arc (C#: `Nlerp`); `a` when the result is
/// zero.
pub(crate) fn nlerp(a: Quat, b: Quat, t: f32) -> Quat {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    let result = a * (1.0 - t) + b * t;
    let length = result.length();
    if length > 1e-12 {
        result * (1.0 / length)
    } else {
        a
    }
}

/// Normalises a quaternion stored as four floats x, y, z, w; the identity when it is zero (C#:
/// `NormalizeQuaternion`).
pub(crate) fn normalize_quaternion(value: &mut [f32]) {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3]).sqrt();
    if length <= 1e-12 {
        value[..4].copy_from_slice(&[0.0, 0.0, 0.0, 1.0]);
        return;
    }
    let scale = 1.0 / length;
    for component in value.iter_mut().take(4) {
        *component *= scale;
    }
}

#[cfg(test)]
mod tests {
    //! C#'s AnimationSamplingTests, with its values.

    use glam::{Quat, Vec3};

    use super::{find_key, slerp};
    use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimationChannel};

    fn channel(path: AnimationPath, interpolation: AnimationInterpolation, times: &[f32], values: &[f32], components: usize) -> ModelAnimationChannel {
        ModelAnimationChannel { node: 0, path, interpolation, times: times.to_vec(), values: values.to_vec(), component_count: components }
    }

    fn sample(channel: &ModelAnimationChannel, time: f32) -> Vec<f32> {
        let mut result = vec![0.0; channel.component_count];
        channel.sample(time, &mut result).unwrap();
        result
    }

    fn rotation(channel: &ModelAnimationChannel, time: f32) -> Quat {
        let v = sample(channel, time);
        Quat::from_xyzw(v[0], v[1], v[2], v[3])
    }

    fn same_rotation(actual: Quat, expected: Quat, tolerance: f32) -> bool {
        (actual.dot(expected).abs() - 1.0).abs() <= tolerance
    }

    fn q(value: Quat) -> [f32; 4] {
        [value.x, value.y, value.z, value.w]
    }

    #[test]
    fn linear_translation_interpolates_and_clamps_outside_the_keys() {
        let channel = channel(AnimationPath::Translation, AnimationInterpolation::Linear, &[1.0, 3.0], &[0.0, 0.0, 0.0, 2.0, 4.0, 6.0], 3);
        assert_eq!(sample(&channel, 2.0), vec![1.0, 2.0, 3.0]);
        assert_eq!(sample(&channel, 1.5), vec![0.5, 1.0, 1.5]);
        assert_eq!(sample(&channel, 0.0), vec![0.0, 0.0, 0.0]);
        assert_eq!(sample(&channel, 1.0), vec![0.0, 0.0, 0.0]);
        assert_eq!(sample(&channel, 3.0), vec![2.0, 4.0, 6.0]);
        assert_eq!(sample(&channel, 99.0), vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn step_holds_each_key_until_the_next_key_time() {
        let channel = channel(AnimationPath::Scale, AnimationInterpolation::Step, &[0.0, 1.0, 2.0], &[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0], 3);
        for (time, expected) in [(0.0, 1.0), (0.999, 1.0), (1.0, 2.0), (1.5, 2.0), (2.0, 3.0), (7.0, 3.0), (-1.0, 1.0)] {
            assert_eq!(sample(&channel, time)[0], expected, "time {time}");
        }
    }

    #[test]
    fn linear_rotation_is_a_slerp_on_the_shorter_arc() {
        let end = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
        let mut values = q(Quat::IDENTITY).to_vec();
        values.extend_from_slice(&q(end));
        let mut flipped = q(Quat::IDENTITY).to_vec();
        flipped.extend_from_slice(&q(-end));
        let forward = channel(AnimationPath::Rotation, AnimationInterpolation::Linear, &[0.0, 1.0], &values, 4);
        let opposite = channel(AnimationPath::Rotation, AnimationInterpolation::Linear, &[0.0, 1.0], &flipped, 4);
        // Slerp keeps constant angular speed: a quarter of the time is a quarter of the angle.
        assert!(same_rotation(rotation(&forward, 0.25), Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI / 8.0), 1e-5));
        let half = rotation(&forward, 0.5);
        assert!(same_rotation(half, Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4), 1e-5));
        assert!(same_rotation(rotation(&opposite, 0.5), Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4), 1e-5));
        assert!((half.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn slerp_handles_nearly_equal_and_nearly_opposite_keys() {
        let a = Quat::from_axis_angle(Vec3::X, 0.3);
        let b = Quat::from_axis_angle(Vec3::X, 0.3 + 1e-5);
        let c = Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI - 0.01);
        let near = slerp(a, b, 0.5);
        let wide = slerp(Quat::IDENTITY, c, 0.5);
        assert!(same_rotation(near, Quat::from_axis_angle(Vec3::X, 0.3 + 5e-6), 1e-5));
        assert!(same_rotation(wide, Quat::from_axis_angle(Vec3::Z, (std::f32::consts::PI - 0.01) / 2.0), 1e-4));
        assert!(!near.w.is_nan());
    }

    #[test]
    fn cubic_spline_reproduces_a_cubic_with_tangents_scaled_by_uneven_key_intervals() {
        // x(t) = t^3 with exact derivatives as tangents at keys 0, 2 and 3 (intervals 2 and 1).
        let times = [0.0f32, 2.0, 3.0];
        let mut values = vec![0.0f32; times.len() * 9];
        for (key, &t) in times.iter().enumerate() {
            let slope = 3.0 * t * t;
            values[key * 9] = slope;
            values[key * 9 + 3] = t * t * t;
            values[key * 9 + 6] = slope;
            values[key * 9 + 4] = 7.0;
            values[key * 9 + 5] = -2.0 * t;
            values[key * 9 + 8] = -2.0;
            values[key * 9 + 2] = -2.0;
        }
        let channel = channel(AnimationPath::Translation, AnimationInterpolation::CubicSpline, &times, &values, 3);
        // A Hermite spline with exact tangents is exact for cubics, linear functions and constants.
        for t in [0.5f32, 1.0, 1.5, 2.5, 2.9] {
            let value = sample(&channel, t);
            assert!((value[0] - t * t * t).abs() < 1e-4, "x at {t}");
            assert!((value[1] - 7.0).abs() < 1e-5, "y at {t}");
            assert!((value[2] + 2.0 * t).abs() < 1e-4, "z at {t}");
        }
        assert_eq!(sample(&channel, -1.0), vec![0.0, 7.0, -0.0]);
        assert_eq!(sample(&channel, 3.0)[0], 27.0);
        assert_eq!(sample(&channel, 10.0)[0], 27.0);
    }

    #[test]
    fn cubic_spline_rotations_are_normalised_and_hit_the_keys() {
        // Zero tangents: the spline eases between the keys; the raw blend is not unit length.
        let end = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
        let mut values = vec![0.0f32; 24];
        values[4..8].copy_from_slice(&q(Quat::IDENTITY));
        values[16..20].copy_from_slice(&q(end));
        let channel = channel(AnimationPath::Rotation, AnimationInterpolation::CubicSpline, &[0.0, 1.0], &values, 4);
        let half = rotation(&channel, 0.5);
        // With zero tangents at t = 0.5 both keys weigh 0.5: their normalised sum (45 degrees).
        assert!((half.length() - 1.0).abs() < 1e-6);
        assert!(same_rotation(half, Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4), 1e-5));
        assert!(same_rotation(rotation(&channel, 1.0), end, 1e-5));
    }

    #[test]
    fn morph_weight_channels_interpolate_every_target() {
        let linear = channel(AnimationPath::Weights, AnimationInterpolation::Linear, &[0.0, 1.0], &[0.0, 1.0, 1.0, 0.0], 2);
        let step = channel(AnimationPath::Weights, AnimationInterpolation::Step, &[0.0, 1.0], &[0.0, 1.0, 1.0, 0.0], 2);
        assert_eq!(sample(&linear, 0.25), vec![0.25, 0.75]);
        assert_eq!(sample(&step, 0.25), vec![0.0, 1.0]);
        assert_eq!(sample(&step, 1.0), vec![1.0, 0.0]);
    }

    #[test]
    fn key_cursor_finds_the_same_interval_as_a_search_in_any_order() {
        let times: Vec<f32> = (0..50).map(|index| index as f32 * 0.1 + (index % 3) as f32 * 0.01).collect();
        let mut cursor = 0;
        // A small linear congruential generator stands in for C#'s Random(7): forward steps, jumps
        // back and random seeks all agree with a plain search.
        let mut state: u64 = 7;
        for step in 0..2000 {
            let time = if step < 1000 {
                step as f32 * 0.006 - 0.5
            } else {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((state >> 11) as f64 / (1u64 << 53) as f64 * 6.0 - 0.5) as f32
            };
            let expected = times.iter().rposition(|&t| t <= time);
            assert_eq!(find_key(&times, time, &mut cursor), expected, "time {time}");
        }
    }

    #[test]
    fn sample_rejects_a_short_destination() {
        let channel = channel(AnimationPath::Rotation, AnimationInterpolation::Linear, &[0.0], &q(Quat::IDENTITY), 4);
        let error = channel.sample(0.0, &mut [0.0; 3]).unwrap_err();
        assert!(error.message().contains("4"), "{}", error.message());
    }
}
