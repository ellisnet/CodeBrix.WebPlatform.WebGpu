//! [`ModelAnimation`], [`ModelAnimationChannel`], [`AnimationPath`] and
//! [`AnimationInterpolation`]: a model's decoded animation clips (C#: the types of the same
//! names). Immutable data; an [`AnimationPlayer`](crate::AnimationPlayer) plays them.

/// Which node property an animation channel drives (C#: `AnimationPath`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimationPath {
    /// The translation, 3 floats per key.
    Translation,
    /// The rotation, 4 floats per key (x, y, z, w).
    Rotation,
    /// The scale, 3 floats per key.
    Scale,
    /// The morph weights, one float per morph target per key.
    Weights,
}

/// How an animation channel interpolates between keys (C#: `AnimationInterpolation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimationInterpolation {
    /// Hold each key's value until the next key.
    Step,
    /// Linear (rotations: spherical linear).
    Linear,
    /// Cubic Hermite spline; each key stores in-tangent, value and out-tangent.
    CubicSpline,
}

/// One decoded animation channel (C#: `ModelAnimationChannel`): the target node and property, the
/// key times and the packed key values - [`ModelAnimationChannel::component_count`] floats per
/// key (x3 for cubic splines: in-tangent, value, out-tangent of each key in turn).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelAnimationChannel {
    /// The target node (C#: `Node`).
    pub node: usize,
    /// The animated property (C#: `Path`).
    pub path: AnimationPath,
    /// The interpolation (C#: `Interpolation`).
    pub interpolation: AnimationInterpolation,
    /// The key times in seconds, ascending (C#: `Times`).
    pub times: Vec<f32>,
    /// The packed key values (C#: `Values`).
    pub values: Vec<f32>,
    /// Floats per value: 3 translation and scale, 4 rotation, the morph-target count for weights
    /// (C#: `ComponentCount`).
    pub component_count: usize,
}

impl ModelAnimationChannel {
    /// The number of keys (C#: `KeyCount`).
    pub fn key_count(&self) -> usize {
        self.times.len()
    }

    /// The time of the last key (C#: `EndTime`).
    pub fn end_time(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }

    /// Evaluates the channel at `time` seconds into the first
    /// [`ModelAnimationChannel::component_count`] floats of `destination` (C#: `Sample`): STEP
    /// holds each key, LINEAR interpolates (rotations by slerp on the shorter arc), CUBICSPLINE
    /// is glTF's Hermite spline (rotations normalised); before the first key the first value,
    /// after the last key the last value. Err when `destination` is shorter than the component
    /// count (C#'s ArgumentException).
    pub fn sample(&self, time: f32, destination: &mut [f32]) -> Result<(), crate::GraphicsException> {
        if destination.len() < self.component_count {
            return Err(crate::GraphicsException::new(format!("The destination holds {} float(s), but the channel's values have {} components. Pass a slice of at least {} floats.", destination.len(), self.component_count, self.component_count)));
        }
        let mut cursor = 0;
        crate::animation::animation_sampling::sample(self, time, &mut cursor, destination);
        Ok(())
    }
}

/// A glTF animation clip: a name, a duration and its channels (C#: `ModelAnimation`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelAnimation {
    /// The index in the model's animations (C#: `Index`).
    pub index: usize,
    /// The name, or None (C#: `Name`).
    pub name: Option<String>,
    /// The channels (C#: `Channels`).
    pub channels: Vec<ModelAnimationChannel>,
}

impl ModelAnimation {
    /// The length in seconds: the last key time of any channel (C#: `Duration`).
    pub fn duration(&self) -> f32 {
        self.channels.iter().map(ModelAnimationChannel::end_time).fold(0.0, f32::max)
    }
}

impl std::fmt::Display for ModelAnimation {
    /// For example `ModelAnimation 0 'walk' 1.25 s` (C#: `ToString`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = format!("{:.3}", self.duration());
        let duration = duration.trim_end_matches('0').trim_end_matches('.');
        match &self.name {
            Some(name) => write!(f, "ModelAnimation {} '{}' {} s", self.index, name, duration),
            None => write!(f, "ModelAnimation {} {} s", self.index, duration),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel};

    #[test]
    fn the_duration_is_the_last_key_of_any_channel() {
        let channel = |times: Vec<f32>| ModelAnimationChannel { node: 0, path: AnimationPath::Translation, interpolation: AnimationInterpolation::Linear, values: vec![0.0; times.len() * 3], times, component_count: 3 };
        let clip = ModelAnimation { index: 0, name: Some("walk".to_string()), channels: vec![channel(vec![0.0, 1.0]), channel(vec![0.0, 1.25])] };
        assert_eq!(clip.duration(), 1.25);
        assert_eq!(clip.to_string(), "ModelAnimation 0 'walk' 1.25 s");
        assert_eq!(clip.channels[0].key_count(), 2);
    }
}
