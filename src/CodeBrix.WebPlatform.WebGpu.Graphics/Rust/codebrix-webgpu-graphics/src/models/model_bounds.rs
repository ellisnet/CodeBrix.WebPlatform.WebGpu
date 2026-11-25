//! The CULLING BOUNDS of a model (Not in C#): its rest-pose bounds grown AT LOAD, on purpose
//! conservatively, to hold every pose the model's own clips reach, so that a moving instance is
//! never culled while a part of it is in view. CPU only, tested on the host.
//!
//! - Node animation: every clip is sampled at its keyframes and between them (often enough that
//!   no rotation turns more than [`MAX_SAMPLE_TURN`] from one sample to the next), the node
//!   matrices of each sampled pose computed, and each part's box placed by its node's matrix.
//! - Skinned parts: for every joint, the box of the skin's vertices that joint moves, in the
//!   joint's own space (how far the skin extends from that joint); each sampled pose places every
//!   joint's box by the joint's matrix. A skinned vertex is a weighted average of its joints'
//!   placements, so it lies inside the boxes of its joints.
//! - Morph targets: every target at full weight (the weight anywhere from 0 to 1, or to the
//!   node's default or a clip's key if those reach further), all targets together.
//! - Then the box grows by [`CULLING_MARGIN`] of its diagonal on every side for the motion
//!   between samples (only for a model with clips; a model without clips keeps the exact box).
//!
//! What it does NOT cover: a pose set by hand outside the clips (`set_local_transform`, a pose
//! edit) that moves parts further than any clip does - give such an instance
//! [`ModelInstance::never_cull`](crate::ModelInstance::never_cull).

use glam::{Mat3, Mat4, Quat, Vec3};

use crate::models::model_layout::ModelLayout;
use crate::models::{AnimationInterpolation, AnimationPath, ModelAnimation, ModelAnimationChannel, ModelData, ModelMesh, ModelNode, ModelPrimitive, ModelSkin, ModelSkinVertex, ModelVertex, NodeTransform};

/// The largest turn of one rotation channel between two neighbouring samples (10 degrees).
pub(crate) const MAX_SAMPLE_TURN: f32 = 10.0 * std::f32::consts::PI / 180.0;
/// Samples per key interval of a cubic spline at least (for its overshoot).
const CUBIC_STEPS: usize = 4;
/// Samples per key interval of a linear channel at least.
const LINEAR_STEPS: usize = 2;
/// Samples per key interval at most.
const MAX_STEPS: usize = 64;
/// The fraction of the box's diagonal the culling bounds of an animated model grow by on every
/// side (the motion between samples; nested turns add up).
pub(crate) const CULLING_MARGIN: f32 = 0.02;

/// An axis-aligned box that may be empty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl Bounds {
    /// The empty box.
    pub(crate) const EMPTY: Bounds = Bounds { min: Vec3::splat(f32::MAX), max: Vec3::splat(f32::MIN) };

    /// A box from its corners.
    pub(crate) fn new(min: Vec3, max: Vec3) -> Bounds {
        Bounds { min, max }
    }

    /// True when nothing was added.
    pub(crate) fn is_empty(&self) -> bool {
        !self.min.cmple(self.max).all()
    }

    /// Grows the box to hold a point.
    pub(crate) fn add_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Grows the box to hold another (an empty one adds nothing).
    pub(crate) fn add(&mut self, other: Bounds) {
        if !other.is_empty() {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
        }
    }

    /// The axis-aligned box that holds this box placed by `matrix` (an affine transform).
    pub(crate) fn transformed(&self, matrix: &Mat4) -> Bounds {
        if self.is_empty() {
            return *self;
        }
        let center = matrix.transform_point3((self.min + self.max) * 0.5);
        let half = (self.max - self.min) * 0.5;
        let linear = Mat3::from_mat4(*matrix);
        let reach = linear.x_axis.abs() * half.x + linear.y_axis.abs() * half.y + linear.z_axis.abs() * half.z;
        Bounds { min: center - reach, max: center + reach }
    }
}

/// The model data the culling bounds are worked out from (a [`ModelData`]'s, or a model's when
/// it is made).
pub(crate) struct BoundsSource<'a> {
    pub(crate) nodes: &'a [ModelNode],
    pub(crate) meshes: &'a [ModelMesh],
    pub(crate) skins: &'a [ModelSkin],
    pub(crate) animations: &'a [ModelAnimation],
    pub(crate) vertex_bytes: &'a [u8],
    pub(crate) skin_bytes: &'a [u8],
}

impl<'a> BoundsSource<'a> {
    /// The source of a model's data.
    pub(crate) fn of(data: &'a ModelData<'a>) -> BoundsSource<'a> {
        BoundsSource { nodes: &data.nodes, meshes: &data.meshes, skins: &data.skins, animations: &data.animations, vertex_bytes: data.vertex_bytes, skin_bytes: data.skin_bytes }
    }
}

impl ModelData<'_> {
    /// The model's CULLING BOUNDS (minimum, maximum) without a GPU - what
    /// [`Model::culling_bounds`](crate::Model::culling_bounds) reports once loaded: the rest-pose
    /// [`ModelData::bounds`] grown to hold every pose the model's clips reach, every morph target
    /// at full weight and the skin around its moving joints (Not in C#). (0, 0, 0) twice for a
    /// model that draws nothing.
    pub fn culling_bounds(&self) -> (Vec3, Vec3) {
        let layout = ModelLayout::new(&self.nodes, &self.meshes, &self.skins);
        culling_bounds(&BoundsSource::of(self), &layout)
    }
}

/// Works out the culling bounds (see the module documentation).
pub(crate) fn culling_bounds(source: &BoundsSource<'_>, layout: &ModelLayout) -> (Vec3, Vec3) {
    if layout.triangle_count == 0 {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    let ranges = weight_ranges(source, layout);
    // Boxes in node space placed by the node's matrix, and in joint space by the joint's.
    let mut node_boxes: Vec<(usize, Bounds)> = Vec::new();
    let mut joint_boxes: Vec<Bounds> = vec![Bounds::EMPTY; layout.palette_size];
    for item in &layout.draw_items {
        let Some(primitive) = source.meshes.get(item.mesh).and_then(|m| m.primitives.get(item.primitive)) else {
            continue;
        };
        let node = &source.nodes[item.node];
        let morphs = morph_ranges(primitive, node, layout.morph_weight_offsets[item.node], &ranges);
        // The renderer's rule: skinned when the primitive has skin data AND its node a skin.
        match node.skin.filter(|&s| s < source.skins.len()) {
            Some(skin) if primitive.is_skinned() => add_skinned(source, primitive, &morphs, &source.skins[skin], layout.skin_palette_offsets[skin], &mut joint_boxes),
            _ => node_boxes.push((item.node, morphed_vertices(source, primitive, &morphs).unwrap_or_else(|| morphed_box(primitive, &morphs)))),
        }
    }

    let mut bounds = Bounds::new(layout.bounds_min, layout.bounds_max);
    let rest: Vec<NodeTransform> = source.nodes.iter().map(|n| n.default_transform).collect();
    let place = |bounds: &mut Bounds, locals: &[NodeTransform]| {
        let matrices = pose_matrices(source.nodes, &layout.node_order, locals);
        for (node, local) in &node_boxes {
            bounds.add(local.transformed(&matrices[*node]));
        }
        for (skin, data) in source.skins.iter().enumerate() {
            let offset = layout.skin_palette_offsets[skin];
            for (joint, &node) in data.joints.iter().enumerate() {
                if let (Some(local), Some(matrix)) = (joint_boxes.get(offset + joint), matrices.get(node)) {
                    bounds.add(local.transformed(matrix));
                }
            }
        }
    };
    place(&mut bounds, &rest);
    let mut sampled = false;
    let mut locals = rest.clone();
    for clip in source.animations {
        for time in sample_times(clip) {
            locals.copy_from_slice(&rest);
            apply_clip(clip, time, &mut locals);
            place(&mut bounds, &locals);
            sampled = true;
        }
    }
    if sampled {
        let margin = (bounds.max - bounds.min).length() * CULLING_MARGIN;
        bounds.min -= Vec3::splat(margin);
        bounds.max += Vec3::splat(margin);
    }
    (bounds.min, bounds.max)
}

/// Per morph-weight slot: the range of (weight - the weight baked into the vertex buffer) the
/// renderer can apply - the weight from 0 to 1, widened to the node's default and to every key
/// of the clips' weight channels.
fn weight_ranges(source: &BoundsSource<'_>, layout: &ModelLayout) -> Vec<(f32, f32)> {
    let mut ranges = vec![(0.0f32, 1.0f32); layout.morph_weight_count];
    let mut widen = |slot: usize, value: f32| {
        if let Some(range) = ranges.get_mut(slot).filter(|_| value.is_finite()) {
            range.0 = range.0.min(value);
            range.1 = range.1.max(value);
        }
    };
    for (index, node) in source.nodes.iter().enumerate() {
        if let Some(offset) = layout.morph_weight_offsets[index] {
            for (target, &weight) in node.default_morph_weights.iter().enumerate() {
                widen(offset + target, weight);
            }
        }
    }
    for channel in source.animations.iter().flat_map(|a| a.channels.iter()).filter(|c| c.path == AnimationPath::Weights) {
        let Some(offset) = layout.morph_weight_offsets.get(channel.node).copied().flatten() else {
            continue;
        };
        let components = channel.component_count;
        let cubic = channel.interpolation == AnimationInterpolation::CubicSpline;
        let stride = if cubic { components * 3 } else { components };
        let first = if cubic { components } else { 0 };
        for key in 0..channel.key_count() {
            for target in 0..components {
                if let Some(&value) = channel.values.get(key * stride + first + target) {
                    widen(offset + target, value);
                }
            }
        }
    }
    ranges.iter().zip(&layout.baked_morph_weights).map(|(&(low, high), &baked)| (low - baked, high - baked)).collect()
}

/// The morph targets of a primitive the renderer applies, each with its weight range (see
/// [`weight_ranges`]); empty when it applies none.
fn morph_ranges<'p>(primitive: &'p ModelPrimitive, node: &ModelNode, offset: Option<usize>, ranges: &[(f32, f32)]) -> Vec<(&'p [Vec3], (f32, f32))> {
    let Some(offset) = offset else {
        return Vec::new();
    };
    let count = primitive.morph_targets.len().min(node.default_morph_weights.len());
    primitive.morph_targets[..count]
        .iter()
        .enumerate()
        .filter(|(_, target)| !target.position_deltas.is_empty())
        .filter_map(|(target, data)| ranges.get(offset + target).map(|range| (data.position_deltas.as_slice(), *range)))
        .collect()
}

/// How far a delta can move a vertex along each axis for weights in `range`: (least, most).
fn reach(delta: Vec3, range: (f32, f32)) -> (Vec3, Vec3) {
    let (a, b) = (delta * range.0, delta * range.1);
    (a.min(b), a.max(b))
}

/// A primitive's box with every morph target anywhere in its range, vertex by vertex: each
/// vertex widened by how far every target can move IT. None when the primitive has no morph
/// targets (its stored bounds are exact) or its vertices are not in the bytes.
fn morphed_vertices(source: &BoundsSource<'_>, primitive: &ModelPrimitive, morphs: &[(&[Vec3], (f32, f32))]) -> Option<Bounds> {
    if morphs.is_empty() {
        return None;
    }
    let vertices = read_records::<ModelVertex>(source.vertex_bytes, primitive.base_vertex as usize, primitive.vertex_count as usize)?;
    let mut bounds = Bounds::EMPTY;
    for (index, vertex) in vertices.iter().enumerate() {
        let (mut least, mut most) = (vertex.position, vertex.position);
        for (deltas, range) in morphs {
            if let Some(delta) = deltas.get(index) {
                let (low, high) = reach(*delta, *range);
                least += low;
                most += high;
            }
        }
        bounds.add_point(least);
        bounds.add_point(most);
    }
    Some(bounds)
}

/// A primitive's box with every morph target anywhere in its range, without its vertices: the
/// stored bounds widened by the most any target moves any vertex, all targets added together.
fn morphed_box(primitive: &ModelPrimitive, morphs: &[(&[Vec3], (f32, f32))]) -> Bounds {
    let mut bounds = Bounds::new(primitive.bounds_min, primitive.bounds_max);
    for (deltas, range) in morphs {
        let mut least = Vec3::ZERO;
        let mut most = Vec3::ZERO;
        for delta in deltas.iter().take(primitive.vertex_count as usize) {
            let (low, high) = reach(*delta, *range);
            least = least.min(low);
            most = most.max(high);
        }
        bounds.min += least;
        bounds.max += most;
    }
    bounds
}

/// Adds a skinned primitive's vertices (each with its morph reach) to the boxes of the joints
/// that move them, in each joint's own space. Without readable vertex or skin data the whole
/// primitive's box goes to every joint of the skin.
fn add_skinned(source: &BoundsSource<'_>, primitive: &ModelPrimitive, morphs: &[(&[Vec3], (f32, f32))], skin: &ModelSkin, offset: usize, joint_boxes: &mut [Bounds]) {
    let inverse_bind = |joint: usize| skin.inverse_bind_matrices.get(joint).copied().unwrap_or(Mat4::IDENTITY);
    let vertices = read_records::<ModelVertex>(source.vertex_bytes, primitive.base_vertex as usize, primitive.vertex_count as usize);
    let influences = primitive.skin_base_vertex.and_then(|base| read_records::<ModelSkinVertex>(source.skin_bytes, base as usize, primitive.vertex_count as usize));
    let (Some(vertices), Some(influences)) = (vertices, influences) else {
        let whole = morphed_box(primitive, morphs);
        for joint in 0..skin.joint_count() {
            if let Some(local) = joint_boxes.get_mut(offset + joint) {
                local.add(whole.transformed(&inverse_bind(joint)));
            }
        }
        return;
    };
    for (index, (vertex, influence)) in vertices.iter().zip(&influences).enumerate() {
        let mut reached = Bounds::new(vertex.position, vertex.position);
        for (deltas, range) in morphs {
            if let Some(delta) = deltas.get(index) {
                let (low, high) = reach(*delta, *range);
                reached.min += low;
                reached.max += high;
            }
        }
        for (&joint, &weight) in influence.joints.iter().zip(&influence.weights) {
            let joint = joint as usize;
            if weight > 0.0 && joint < skin.joint_count() {
                if let Some(local) = joint_boxes.get_mut(offset + joint) {
                    local.add(reached.transformed(&inverse_bind(joint)));
                }
            }
        }
    }
}

/// `count` plain-data records from `first` on, None when the bytes do not hold them all.
fn read_records<T: bytemuck::Pod>(bytes: &[u8], first: usize, count: usize) -> Option<Vec<T>> {
    let size = std::mem::size_of::<T>();
    let start = first.checked_mul(size)?;
    let end = start.checked_add(count.checked_mul(size)?)?;
    Some(bytes.get(start..end)?.chunks_exact(size).map(bytemuck::pod_read_unaligned).collect())
}

/// The times a clip is sampled at: every key of every transform channel, and steps between keys,
/// at least [`LINEAR_STEPS`] (cubic splines [`CUBIC_STEPS`]), more for a rotation that turns
/// far, so that no step turns more than [`MAX_SAMPLE_TURN`]. Sorted, each once.
pub(crate) fn sample_times(clip: &ModelAnimation) -> Vec<f32> {
    let mut times: Vec<f32> = Vec::new();
    for channel in clip.channels.iter().filter(|c| c.path != AnimationPath::Weights) {
        let keys = &channel.times;
        let Some(&first) = keys.first() else {
            continue;
        };
        times.push(first);
        for key in 0..keys.len() - 1 {
            let steps = match channel.interpolation {
                AnimationInterpolation::Step => 1,
                AnimationInterpolation::Linear => LINEAR_STEPS.max(turn_steps(channel, key)),
                AnimationInterpolation::CubicSpline => CUBIC_STEPS.max(turn_steps(channel, key)),
            }
            .min(MAX_STEPS);
            let (from, to) = (keys[key], keys[key + 1]);
            times.extend((1..=steps).map(|step| from + (to - from) * step as f32 / steps as f32));
        }
    }
    times.retain(|t| t.is_finite());
    times.sort_by(f32::total_cmp);
    times.dedup();
    times
}

/// The steps a rotation channel needs between key `key` and the next so that no step turns more
/// than [`MAX_SAMPLE_TURN`]; 1 for other channels.
fn turn_steps(channel: &ModelAnimationChannel, key: usize) -> usize {
    if channel.path != AnimationPath::Rotation || channel.component_count != 4 {
        return 1;
    }
    let cubic = channel.interpolation == AnimationInterpolation::CubicSpline;
    let at = |k: usize| -> Option<Quat> {
        let start = if cubic { (k * 3 + 1) * 4 } else { k * 4 };
        let v = channel.values.get(start..start + 4)?;
        let q = Quat::from_xyzw(v[0], v[1], v[2], v[3]);
        (q.length_squared() > 0.0).then(|| q.normalize())
    };
    match (at(key), at(key + 1)) {
        (Some(a), Some(b)) => {
            let turn = 2.0 * a.dot(b).abs().min(1.0).acos();
            (turn / MAX_SAMPLE_TURN).ceil() as usize
        }
        _ => 1,
    }
}

/// Applies a clip's transform channels at `time` over the given local transforms (morph weights
/// are covered by their ranges instead).
pub(crate) fn apply_clip(clip: &ModelAnimation, time: f32, locals: &mut [NodeTransform]) {
    let mut value = [0.0f32; 4];
    for channel in clip.channels.iter().filter(|c| c.path != AnimationPath::Weights && c.component_count <= 4) {
        let Some(local) = locals.get_mut(channel.node) else {
            continue;
        };
        if channel.sample(time, &mut value).is_err() {
            continue;
        }
        match channel.path {
            AnimationPath::Translation => local.translation = Vec3::new(value[0], value[1], value[2]),
            AnimationPath::Rotation => {
                let rotation = Quat::from_xyzw(value[0], value[1], value[2], value[3]);
                if rotation.length_squared() > 0.0 {
                    local.rotation = rotation.normalize();
                }
            }
            AnimationPath::Scale => local.scale = Vec3::new(value[0], value[1], value[2]),
            AnimationPath::Weights => {}
        }
    }
}

/// Every node's model-space matrix for these local transforms, parents first (as an instance
/// computes them: a node with a fixed matrix keeps it).
pub(crate) fn pose_matrices(nodes: &[ModelNode], order: &[usize], locals: &[NodeTransform]) -> Vec<Mat4> {
    let mut matrices = vec![Mat4::IDENTITY; nodes.len()];
    for &index in order {
        let node = &nodes[index];
        let local = if node.has_fixed_matrix { node.local_matrix } else { locals[index].to_matrix() };
        matrices[index] = match node.parent {
            Some(parent) => matrices[parent] * local,
            None => local,
        };
    }
    matrices
}

#[cfg(test)]
#[path = "model_bounds_tests.rs"]
mod tests;
