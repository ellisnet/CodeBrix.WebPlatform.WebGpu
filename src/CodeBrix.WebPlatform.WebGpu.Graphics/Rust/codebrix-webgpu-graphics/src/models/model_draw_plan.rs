//! The CPU half of [`ModelRenderer`](crate::ModelRenderer)'s finishing step: which instance
//! records, matrices and weights are uploaded, which instanced draws the opaque parts become and in
//! which order the blended parts are drawn - a blended double-sided part twice, back faces then
//! front faces - which instances culling leaves out, what each shadow pass draws, and which draws
//! carry a level-of-detail fade's dither (C#: the
//! packing and sorting of `PbrModelRenderPath`, with the additions marked "Not in C#"). Tested on
//! the host.

use glam::{Mat4, Vec3, Vec4};

use crate::models::pbr_data::{determinant3, PbrInstance, INSTANCE_RECEIVES_SHADOWS};
use crate::shaders::PbrFeatures;
use crate::visibility::CullVolume;

/// True when [`ModelRenderer::draw`](crate::ModelRenderer::draw) leaves an instance out (Not in
/// C#): culling is on, the instance does not ask never to be culled, and its model's culling
/// bounds (`bounds`, model space), placed by its world matrix, are entirely outside `volume`. A
/// culled instance is not queued: no pose upload, no draw.
pub(crate) fn is_culled(volume: &CullVolume, culling: bool, never_cull: bool, bounds: (Vec3, Vec3), world: Mat4) -> bool {
    culling && !never_cull && !volume.intersects_transformed_box(bounds.0, bounds.1, world)
}

/// Where an instance given to `draw` is drawn (Not in C#): `in_view` - in the camera's pass (not
/// culled against `view`); `shadow_mask` bit k - in shadow pass k (it casts, and is not culled
/// against `shadow_volumes[k]`, the light's own volume). Each pass culls against its own volume,
/// so a caster outside the camera's view (behind it, say) still casts. An instance in no pass is
/// left out entirely.
pub(crate) fn cull_passes(view: &CullVolume, shadow_volumes: &[&CullVolume], culling: bool, never_cull: bool, casts: bool, bounds: (Vec3, Vec3), world: Mat4) -> (bool, u8) {
    let in_view = !is_culled(view, culling, never_cull, bounds, world);
    let mut shadow_mask = 0u8;
    if casts {
        for (pass, volume) in shadow_volumes.iter().enumerate().take(8) {
            if !is_culled(volume, culling, never_cull, bounds, world) {
                shadow_mask |= 1 << pass;
            }
        }
    }
    (in_view, shadow_mask)
}

/// Which faces a draw draws (Not in C#): all the material allows, or - for a part that is both
/// blended and double-sided, drawn twice - only the back faces first, then only the front faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaceSide {
    /// As the material says (one draw).
    Both,
    /// The first draw of a two-pass part: front faces culled.
    Back,
    /// The second draw of a two-pass part: back faces culled.
    Front,
}

/// One draw item of a model as the renderer draws it (C#: `PbrDraw`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PbrDraw {
    /// The material's index, None for the default material.
    pub(crate) material: Option<usize>,
    /// The shader features of the draw (material, skinning, morphing).
    pub(crate) features: PbrFeatures,
    /// The mesh node the draw item belongs to.
    pub(crate) node: usize,
    /// True when the primitive is skinned (its vertices are in model space, moved by the joints).
    pub(crate) skinned: bool,
    /// (first joint in the instance's palette, joint count) of the skin, when skinned.
    pub(crate) skin_palette: (usize, usize),
    /// True for an alpha-blended material.
    pub(crate) blend: bool,
    /// The centre of the primitive's bounds as stored (default morph weights, rest pose).
    pub(crate) center: Vec3,
    /// (first shift in the model's centre-shift list, first weight in the instance's weights,
    /// target count) when the primitive's morph targets move positions; count 0 for none. Not in C#.
    pub(crate) morph: (usize, usize, usize),
    /// The primitive's first index.
    pub(crate) first_index: u32,
    /// The primitive's index count.
    pub(crate) index_count: u32,
    /// The primitive's first vertex.
    pub(crate) base_vertex: u32,
}

/// What one instance looks like beyond its pose (Not in C#): the tint (rgb) and alpha (a), the
/// emissive strength, whether it receives shadows, and the dither code of a level-of-detail fade
/// (0: none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InstanceLook {
    pub(crate) tint: Vec4,
    pub(crate) emissive: f32,
    pub(crate) receives_shadows: bool,
    pub(crate) dither: u32,
}

impl InstanceLook {
    /// The look of an instance that changes nothing.
    #[cfg(test)]
    pub(crate) const PLAIN: InstanceLook = InstanceLook { tint: Vec4::ONE, emissive: 1.0, receives_shadows: true, dither: 0 };

    /// True when the instance is drawn with blending whatever its materials (alpha below 1).
    pub(crate) fn faded(&self) -> bool {
        self.tint.w < 1.0
    }

    /// True when the instance is a fading level of detail: its draws use the dither variants.
    pub(crate) fn dithered(&self) -> bool {
        self.dither != 0
    }
}

/// The instances of one model queued between begin and end, copied at `draw`.
pub(crate) struct Bucket {
    pub(crate) model_id: u64,
    pub(crate) worlds: Vec<Mat4>,
    pub(crate) node_matrices: Vec<Mat4>,
    pub(crate) palettes: Vec<Mat4>,
    /// The morph weights minus the weights baked into the vertex buffer.
    pub(crate) weights: Vec<f32>,
    pub(crate) looks: Vec<InstanceLook>,
    /// Per instance: None when every node shows, else the HIDDEN flag of every node (a hidden
    /// node's descendants included). Not in C#.
    pub(crate) hidden: Vec<Option<Box<[bool]>>>,
    /// Per instance: true when the camera's pass draws it (false: only shadow passes do). Not in C#.
    pub(crate) in_view: Vec<bool>,
    /// Per instance: bit k set when shadow pass k draws it ([`cull_passes`]). Not in C#.
    pub(crate) shadow_mask: Vec<u8>,
}

impl Bucket {
    /// An empty bucket for a model.
    pub(crate) fn new(model_id: u64) -> Bucket {
        Bucket { model_id, worlds: Vec::new(), node_matrices: Vec::new(), palettes: Vec::new(), weights: Vec::new(), looks: Vec::new(), hidden: Vec::new(), in_view: Vec::new(), shadow_mask: Vec::new() }
    }
}

/// What the plan needs to know of a bucket's model.
pub(crate) struct PlanModel<'a> {
    pub(crate) draws: &'a [PbrDraw],
    pub(crate) node_count: usize,
    pub(crate) palette_size: usize,
    pub(crate) weight_count: usize,
    /// How far each morph target moves a primitive's centre at weight 1 (see
    /// [`PbrDraw::morph`]).
    pub(crate) morph_shifts: &'a [Vec3],
}

/// One draw call of the plan: draw item `draw` of bucket `bucket`'s model, for `count` instance
/// records from `first_instance` (counted from the plan's first record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlannedDraw {
    pub(crate) bucket: usize,
    pub(crate) draw: usize,
    pub(crate) first_instance: u32,
    pub(crate) count: u32,
    /// The draw's model matrix mirrors (clockwise front faces).
    pub(crate) mirrored: bool,
    /// An opaque or alpha-tested part drawn blended because its instance's alpha is below 1.
    pub(crate) faded: bool,
    /// The faces it draws (Not in C#).
    pub(crate) side: FaceSide,
    /// The instances are a fading level of detail: drawn with the dither variant (Not in C#).
    pub(crate) dithered: bool,
}

/// The finishing step worked out on the CPU: what to upload and what to draw.
pub(crate) struct DrawPlan {
    pub(crate) instances: Vec<PbrInstance>,
    pub(crate) matrices: Vec<Mat4>,
    pub(crate) weights: Vec<f32>,
    /// The opaque and alpha-tested parts: one instanced draw per draw item and run of instances
    /// that draw it alike (normally ONE per draw item for all instances of a model).
    pub(crate) opaque: Vec<PlannedDraw>,
    /// The blended parts (and the parts of faded instances), one instance each, back to front; a
    /// blended double-sided part as two neighbouring draws, back faces then front faces, when the
    /// plan was made with `two_pass`.
    pub(crate) blended: Vec<PlannedDraw>,
    /// Per shadow pass: the draws of the parts that cast into it - opaque and alpha-tested parts
    /// only (blended parts and faded instances do not cast), both faces, one instanced draw per
    /// draw item and run of casting instances (Not in C#).
    pub(crate) shadows: Vec<Vec<PlannedDraw>>,
}

/// Works out the upload and the draws of the queued buckets (`models[i]` describes bucket i's
/// model). Instances of a model whose world mirrors go after the others, and instances with a
/// hidden node or alpha below 1 after the plain ones, so the plain instances of a model stay ONE
/// run per draw item. With `two_pass`, a part whose material is both blended and double-sided is
/// drawn twice, back faces then front faces, at the same place in the back-to-front order (Not in
/// C#; a faded instance's opaque double-sided parts are not: only the material's blending counts).
/// Instances outside the camera's view (kept for a shadow pass) go after all the others and are
/// drawn only by the `shadow_passes` shadow passes (Not in C#). Instances of a fading level of
/// detail (a dither code) form runs of their own, in the camera's pass and in the shadow passes,
/// and their records carry the code (Not in C#).
pub(crate) fn plan(buckets: &[Bucket], models: &[PlanModel<'_>], camera_position: Vec3, camera_forward: Vec3, two_pass: bool, shadow_passes: usize) -> DrawPlan {
    let mut instances: Vec<PbrInstance> = Vec::new();
    let mut matrices: Vec<Mat4> = Vec::new();
    let mut weights: Vec<f32> = Vec::new();
    let mut opaque: Vec<PlannedDraw> = Vec::new();
    let mut blends: Vec<(f32, PlannedDraw)> = Vec::new();
    let mut shadows: Vec<Vec<PlannedDraw>> = vec![Vec::new(); shadow_passes];
    for (bucket_index, (bucket, model)) in buckets.iter().zip(models).enumerate() {
        let nodes = model.node_count;
        let palette = model.palette_size;
        let weight_count = model.weight_count;
        let draw_count = model.draws.len();
        let first = instances.len();
        let count = bucket.worlds.len();
        let in_view = |i: usize| bucket.in_view.get(i).copied().unwrap_or(true);
        let key = |i: usize| (!in_view(i), determinant3(&bucket.worlds[i]) < 0.0, bucket.looks[i].dithered(), bucket.looks[i].faded() || bucket.hidden[i].is_some());
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by_key(|&i| key(i));
        // Per (instance position, draw item): Some((mirrored, dithered)) when the opaque pass draws it.
        let mut opaque_flags: Vec<Option<(bool, bool)>> = Vec::with_capacity(count * draw_count);
        for (position, &index) in order.iter().enumerate() {
            let world = bucket.worlds[index];
            let world_mirrored = determinant3(&world) < 0.0;
            let node_matrices = &bucket.node_matrices[index * nodes..(index + 1) * nodes];
            let palettes = &bucket.palettes[index * palette..(index + 1) * palette];
            let instance_weights = &bucket.weights[index * weight_count..(index + 1) * weight_count];
            let look = bucket.looks[index];
            let faded = look.faded();
            let dithered = look.dithered();
            let hidden = bucket.hidden[index].as_deref();
            let flags = [if look.receives_shadows { INSTANCE_RECEIVES_SHADOWS } else { 0 }, look.dither, 0, 0];
            instances.push(PbrInstance { world, node_base: matrices.len() as u32, joint_base: (matrices.len() + nodes) as u32, weight_base: weights.len() as u32, emissive: look.emissive, tint: look.tint, flags });
            matrices.extend_from_slice(node_matrices);
            matrices.extend_from_slice(palettes);
            weights.extend_from_slice(instance_weights);
            let viewed = in_view(index);
            for (draw_index, draw) in model.draws.iter().enumerate() {
                if !viewed || hidden.is_some_and(|h| h.get(draw.node).copied().unwrap_or(false)) {
                    opaque_flags.push(None);
                    continue;
                }
                let mirrored = if draw.skinned { world_mirrored } else { world_mirrored != (determinant3(&node_matrices[draw.node]) < 0.0) };
                if draw.blend || faded {
                    opaque_flags.push(None);
                    let center = world.transform_point3(blend_center(draw, node_matrices, palettes, instance_weights, model.morph_shifts));
                    let depth = (center - camera_position).dot(camera_forward);
                    let planned = PlannedDraw { bucket: bucket_index, draw: draw_index, first_instance: (first + position) as u32, count: 1, mirrored, faded: faded && !draw.blend, side: FaceSide::Both, dithered };
                    if two_pass && draw.blend && draw.features.contains(PbrFeatures::DOUBLE_SIDED) {
                        // The far side first; the stable sort keeps the two together, in order.
                        blends.push((-depth, PlannedDraw { side: FaceSide::Back, ..planned }));
                        blends.push((-depth, PlannedDraw { side: FaceSide::Front, ..planned }));
                    } else {
                        blends.push((-depth, planned));
                    }
                } else {
                    opaque_flags.push(Some((mirrored, dithered)));
                }
            }
        }
        for draw_index in 0..draw_count {
            let flags = (0..count).map(|position| opaque_flags[position * draw_count + draw_index]);
            for (start, end, flag) in runs(flags) {
                if let Some((mirrored, dithered)) = flag {
                    opaque.push(PlannedDraw { bucket: bucket_index, draw: draw_index, first_instance: (first + start) as u32, count: (end - start) as u32, mirrored, faded: false, side: FaceSide::Both, dithered });
                }
            }
        }
        // The shadow passes: the casting instances' parts that are neither blended nor hidden
        // (both faces - the pipelines cull nothing - so mirroring does not split a run; a fading
        // level of detail does: it casts through the dither).
        for (pass, draws) in shadows.iter_mut().enumerate() {
            for (draw_index, draw) in model.draws.iter().enumerate() {
                if draw.blend {
                    continue;
                }
                let flags = order.iter().map(|&index| {
                    let casts = bucket.shadow_mask.get(index).is_some_and(|mask| mask & (1 << pass) != 0) && !bucket.looks[index].faded();
                    let drawn = casts && !bucket.hidden[index].as_deref().is_some_and(|h| h.get(draw.node).copied().unwrap_or(false));
                    drawn.then(|| bucket.looks[index].dithered())
                });
                for (start, end, flag) in runs(flags) {
                    if let Some(dithered) = flag {
                        draws.push(PlannedDraw { bucket: bucket_index, draw: draw_index, first_instance: (first + start) as u32, count: (end - start) as u32, mirrored: false, faded: false, side: FaceSide::Both, dithered });
                    }
                }
            }
        }
    }
    // Back to front; equal depths keep their queue order.
    blends.sort_by(|a, b| a.0.total_cmp(&b.0));
    DrawPlan { instances, matrices, weights, opaque, blended: blends.into_iter().map(|(_, draw)| draw).collect(), shadows }
}

/// Where a blended part is now, in model space (the sort key; Not in C#, which sorts by the
/// rest-pose, default-morph centre): the stored centre, moved by the morph targets' centre shifts
/// scaled by the instance's weights, then by the node's matrix - or, for a skinned part, by the
/// AVERAGE of its skin's current joint matrices (the rest-pose palette is the identity, so an
/// unanimated skin leaves the centre where it is).
pub(crate) fn blend_center(draw: &PbrDraw, node_matrices: &[Mat4], palettes: &[Mat4], weights: &[f32], shifts: &[Vec3]) -> Vec3 {
    let (shift_start, weight_start, targets) = draw.morph;
    let mut center = draw.center;
    for target in 0..targets {
        if let (Some(weight), Some(shift)) = (weights.get(weight_start + target), shifts.get(shift_start + target)) {
            center += *shift * *weight;
        }
    }
    if draw.skinned {
        let (offset, count) = draw.skin_palette;
        match palettes.get(offset..offset + count) {
            Some(joints) if count > 0 => average(joints).transform_point3(center),
            _ => center,
        }
    } else {
        node_matrices.get(draw.node).map_or(center, |m| m.transform_point3(center))
    }
}

/// The element-wise average of matrices.
fn average(matrices: &[Mat4]) -> Mat4 {
    let sum = matrices.iter().fold(Mat4::ZERO, |sum, m| sum + *m);
    sum * (1.0 / matrices.len() as f32)
}

/// The runs of equal values in `flags`: (start, end, value) for each run (C#: one draw per run of
/// instances with the same winding).
pub(crate) fn runs<T: PartialEq + Copy>(flags: impl Iterator<Item = T>) -> Vec<(usize, usize, T)> {
    let mut result: Vec<(usize, usize, T)> = Vec::new();
    for (index, flag) in flags.enumerate() {
        match result.last_mut() {
            Some(last) if last.2 == flag => last.1 = index + 1,
            _ => result.push((index, index + 1, flag)),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3, Vec4};

    use super::{blend_center, cull_passes, is_culled, plan, runs, Bucket, FaceSide, InstanceLook, PbrDraw, PlanModel, PlannedDraw};
    use crate::drawing3d::Camera;
    use crate::visibility::CullVolume;
    use crate::shaders::PbrFeatures;

    fn part(node: usize, blend: bool, center: Vec3) -> PbrDraw {
        PbrDraw { material: None, features: if blend { PbrFeatures::ALPHA_BLEND } else { PbrFeatures::NONE }, node, skinned: false, skin_palette: (0, 0), blend, center, morph: (0, 0, 0), first_index: 0, index_count: 3, base_vertex: 0 }
    }

    /// A bucket of `count` instances of a model with `nodes` nodes, all at `world`, no skin, no
    /// morph weights.
    fn bucket(model_id: u64, nodes: usize, worlds: &[Mat4]) -> Bucket {
        let mut bucket = Bucket::new(model_id);
        for world in worlds {
            bucket.worlds.push(*world);
            bucket.node_matrices.extend(std::iter::repeat_n(Mat4::IDENTITY, nodes));
            bucket.looks.push(InstanceLook::PLAIN);
            bucket.hidden.push(None);
            bucket.in_view.push(true);
            bucket.shadow_mask.push(0);
        }
        bucket
    }

    fn model<'a>(draws: &'a [PbrDraw], nodes: usize) -> PlanModel<'a> {
        PlanModel { draws, node_count: nodes, palette_size: 0, weight_count: 0, morph_shifts: &[] }
    }

    /// The camera at the origin looking down -Z: depth = -z.
    fn camera() -> (Vec3, Vec3) {
        (Vec3::ZERO, Vec3::NEG_Z)
    }

    #[test]
    fn plain_opaque_instances_of_a_model_are_one_draw_per_primitive() {
        let draws = [part(0, false, Vec3::ZERO), part(1, false, Vec3::ZERO)];
        let worlds: Vec<Mat4> = (0..5).map(|i| Mat4::from_translation(Vec3::new(i as f32, 0.0, -5.0))).collect();
        let buckets = [bucket(1, 2, &worlds)];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 2)], position, forward, true, 0);
        assert_eq!(plan.instances.len(), 5);
        assert_eq!(plan.opaque.len(), 2);
        assert_eq!(plan.opaque[0], PlannedDraw { bucket: 0, draw: 0, first_instance: 0, count: 5, mirrored: false, faded: false, side: FaceSide::Both, dithered: false });
        assert!(plan.blended.is_empty());
    }

    #[test]
    fn a_model_that_mixes_both_has_opaque_and_blended_draws() {
        let draws = [part(0, false, Vec3::ZERO), part(0, true, Vec3::ZERO)];
        let buckets = [bucket(1, 1, &[Mat4::IDENTITY, Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0))])];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(plan.opaque.len(), 1);
        assert_eq!(plan.opaque[0].draw, 0);
        assert_eq!(plan.opaque[0].count, 2);
        assert_eq!(plan.blended.len(), 2);
        assert!(plan.blended.iter().all(|d| d.draw == 1 && d.count == 1 && !d.faded));
        // Back to front: the instance at z = -3 first.
        assert_eq!(plan.blended[0].first_instance, 1);
    }

    #[test]
    fn a_hidden_node_drops_its_draws_and_its_descendants_for_that_instance_only() {
        let draws = [part(0, false, Vec3::ZERO), part(1, false, Vec3::ZERO), part(1, true, Vec3::ZERO)];
        let mut buckets = [bucket(1, 2, &[Mat4::IDENTITY, Mat4::IDENTITY, Mat4::IDENTITY])];
        // Instance 0 hides node 1 (the mask holds the effective flags, descendants included).
        buckets[0].hidden[0] = Some(vec![false, true].into_boxed_slice());
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 2)], position, forward, true, 0);
        // Node 0's part: all three instances in one run; node 1's: the two others, still one run
        // (the instance with a hidden node goes last).
        let node0: Vec<&PlannedDraw> = plan.opaque.iter().filter(|d| d.draw == 0).collect();
        let node1: Vec<&PlannedDraw> = plan.opaque.iter().filter(|d| d.draw == 1).collect();
        assert_eq!(node0.len(), 1);
        assert_eq!(node0[0].count, 3);
        assert_eq!(node1.len(), 1);
        assert_eq!((node1[0].first_instance, node1[0].count), (0, 2));
        assert_eq!(plan.blended.len(), 2);
        assert!(plan.blended.iter().all(|d| d.first_instance != 2));
    }

    #[test]
    fn an_instance_below_alpha_one_is_drawn_with_the_blended_parts() {
        let draws = [part(0, false, Vec3::ZERO)];
        let mut buckets = [bucket(1, 1, &[Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0)), Mat4::from_translation(Vec3::new(0.0, 0.0, -9.0))])];
        buckets[0].looks[0] = InstanceLook { tint: Vec4::new(1.0, 1.0, 1.0, 0.5), emissive: 2.0, receives_shadows: true, dither: 0 };
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(plan.opaque.len(), 1);
        assert_eq!(plan.opaque[0].count, 1);
        assert_eq!(plan.blended.len(), 1);
        assert!(plan.blended[0].faded);
        // The faded instance went after the plain one; its record carries the look.
        let record = plan.instances[plan.blended[0].first_instance as usize];
        assert_eq!(record.tint.w, 0.5);
        assert_eq!(record.emissive, 2.0);
    }

    #[test]
    fn mirrored_instances_run_after_the_others() {
        let draws = [part(0, false, Vec3::ZERO)];
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let buckets = [bucket(1, 1, &[mirror, Mat4::IDENTITY, mirror, Mat4::IDENTITY])];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(plan.opaque, vec![PlannedDraw { bucket: 0, draw: 0, first_instance: 0, count: 2, mirrored: false, faded: false, side: FaceSide::Both, dithered: false }, PlannedDraw { bucket: 0, draw: 0, first_instance: 2, count: 2, mirrored: true, faded: false, side: FaceSide::Both, dithered: false }]);
    }

    #[test]
    fn a_morph_that_moves_a_part_changes_the_draw_order() {
        // Two blended parts of one model at z = -5 (A) and z = -4 (B): A is further, so A first.
        // A's one morph target moves its centre 3 m towards the camera at weight 1.
        let mut a = part(0, true, Vec3::new(0.0, 0.0, -5.0));
        a.morph = (0, 0, 1);
        let b = part(0, true, Vec3::new(0.0, 0.0, -4.0));
        let draws = [a, b];
        let shifts = [Vec3::new(0.0, 0.0, 3.0)];
        let order = |weight: f32| {
            let mut buckets = [bucket(1, 1, &[Mat4::IDENTITY])];
            buckets[0].weights = vec![weight];
            let models = [PlanModel { draws: &draws, node_count: 1, palette_size: 0, weight_count: 1, morph_shifts: &shifts }];
            let (position, forward) = camera();
            plan(&buckets, &models, position, forward, true, 0).blended.iter().map(|d| d.draw).collect::<Vec<usize>>()
        };
        assert_eq!(order(0.0), vec![0, 1]);
        // At weight 1 A sits at z = -2, in front of B.
        assert_eq!(order(1.0), vec![1, 0]);
    }

    #[test]
    fn a_skinned_part_follows_the_average_of_its_joints() {
        // A skinned blended part (rest centre z = -5) and a plain blended part at z = -4.
        let mut skinned = part(0, true, Vec3::new(0.0, 0.0, -5.0));
        skinned.skinned = true;
        skinned.skin_palette = (0, 2);
        let plain = part(0, true, Vec3::new(0.0, 0.0, -4.0));
        let draws = [skinned, plain];
        let order = |palette: [Mat4; 2]| {
            let mut buckets = [bucket(1, 1, &[Mat4::IDENTITY])];
            buckets[0].palettes = palette.to_vec();
            let models = [PlanModel { draws: &draws, node_count: 1, palette_size: 2, weight_count: 0, morph_shifts: &[] }];
            let (position, forward) = camera();
            plan(&buckets, &models, position, forward, true, 0).blended.iter().map(|d| d.draw).collect::<Vec<usize>>()
        };
        // The rest pose (identity palette): the skinned part is further, so it is drawn first.
        assert_eq!(order([Mat4::IDENTITY; 2]), vec![0, 1]);
        // Both joints moved 4 m towards the camera: the average moves the centre to z = -1.
        let moved = Mat4::from_translation(Vec3::new(0.0, 0.0, 4.0));
        assert_eq!(order([moved, moved]), vec![1, 0]);
        // One joint moved 4 m, the other not: the average moves it 2 m, to z = -3, still in front.
        assert_eq!(order([moved, Mat4::IDENTITY]), vec![1, 0]);
    }

    #[test]
    fn the_blend_centre_combines_morph_shift_and_node_matrix() {
        let mut draw = part(1, true, Vec3::new(1.0, 0.0, 0.0));
        draw.morph = (1, 2, 2);
        let nodes = [Mat4::IDENTITY, Mat4::from_translation(Vec3::new(0.0, 10.0, 0.0))];
        let shifts = [Vec3::splat(100.0), Vec3::X, Vec3::Z];
        let weights = [9.0, 9.0, 0.5, 2.0];
        assert_eq!(blend_center(&draw, &nodes, &[], &weights, &shifts), Vec3::new(1.5, 10.0, 2.0));
    }

    /// A blended, double-sided part.
    fn two_sided(node: usize, center: Vec3) -> PbrDraw {
        let mut draw = part(node, true, center);
        draw.features = PbrFeatures::ALPHA_BLEND | PbrFeatures::DOUBLE_SIDED;
        draw
    }

    fn sides(plan: &super::DrawPlan) -> Vec<(usize, FaceSide)> {
        plan.blended.iter().map(|d| (d.draw, d.side)).collect()
    }

    #[test]
    fn a_blended_double_sided_part_is_drawn_back_faces_then_front_faces_in_its_place() {
        // Far (z = -9) and near (z = -1) two-sided parts, a one-sided blended part between them,
        // an opaque double-sided part (one draw, opaque pass).
        let mut opaque_two_sided = part(0, false, Vec3::ZERO);
        opaque_two_sided.features = PbrFeatures::DOUBLE_SIDED;
        let draws = [two_sided(0, Vec3::new(0.0, 0.0, -1.0)), part(0, true, Vec3::new(0.0, 0.0, -5.0)), two_sided(0, Vec3::new(0.0, 0.0, -9.0)), opaque_two_sided];
        let buckets = [bucket(1, 1, &[Mat4::IDENTITY])];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(sides(&plan), vec![(2, FaceSide::Back), (2, FaceSide::Front), (1, FaceSide::Both), (0, FaceSide::Back), (0, FaceSide::Front)]);
        assert_eq!(plan.opaque.len(), 1);
        assert_eq!(plan.opaque[0].side, FaceSide::Both);
        // The setting off: one draw each, as before.
        let one_pass = super::plan(&buckets, &[model(&draws, 1)], position, forward, false, 0);
        assert_eq!(sides(&one_pass), vec![(2, FaceSide::Both), (1, FaceSide::Both), (0, FaceSide::Both)]);
    }

    #[test]
    fn two_pass_parts_of_several_instances_keep_each_pair_together_back_to_front() {
        // Three instances of a model with one two-sided part, queued near, far, middle.
        let draws = [two_sided(0, Vec3::ZERO)];
        let worlds = [-2.0f32, -8.0, -5.0].map(|z| Mat4::from_translation(Vec3::new(0.0, 0.0, z)));
        let buckets = [bucket(1, 1, &worlds)];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        let order: Vec<(u32, FaceSide)> = plan.blended.iter().map(|d| (d.first_instance, d.side)).collect();
        assert_eq!(order, vec![(1, FaceSide::Back), (1, FaceSide::Front), (2, FaceSide::Back), (2, FaceSide::Front), (0, FaceSide::Back), (0, FaceSide::Front)]);
    }

    #[test]
    fn a_mirrored_instance_s_two_draws_are_both_mirrored_back_faces_first() {
        // The mirrored instance's draws carry `mirrored` (clockwise front faces), so the pipeline's
        // "cull front" of its first draw drops the faces nearest the camera, as for the plain one.
        let draws = [two_sided(0, Vec3::ZERO)];
        let mirror = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0)) * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        let buckets = [bucket(1, 1, &[Mat4::from_translation(Vec3::new(0.0, 0.0, -6.0)), mirror])];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        let order: Vec<(u32, bool, FaceSide)> = plan.blended.iter().map(|d| (d.first_instance, d.mirrored, d.side)).collect();
        // The mirrored instance went after the plain one (record 1) and is nearer: drawn last.
        assert_eq!(order, vec![(0, false, FaceSide::Back), (0, false, FaceSide::Front), (1, true, FaceSide::Back), (1, true, FaceSide::Front)]);
        // A mirroring NODE matrix flips it the same way.
        let mut buckets = [bucket(1, 1, &[Mat4::IDENTITY])];
        buckets[0].node_matrices[0] = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
        let plan = super::plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert!(plan.blended.iter().all(|d| d.mirrored) && plan.blended.len() == 2);
    }

    #[test]
    fn a_faded_instance_s_opaque_double_sided_part_is_not_drawn_twice() {
        let mut opaque_two_sided = part(0, false, Vec3::ZERO);
        opaque_two_sided.features = PbrFeatures::DOUBLE_SIDED;
        let draws = [opaque_two_sided];
        let mut buckets = [bucket(1, 1, &[Mat4::IDENTITY])];
        buckets[0].looks[0] = InstanceLook { tint: Vec4::new(1.0, 1.0, 1.0, 0.5), emissive: 1.0, receives_shadows: true, dither: 0 };
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(plan.blended.len(), 1);
        assert_eq!((plan.blended[0].faded, plan.blended[0].side), (true, FaceSide::Both));
    }

    #[test]
    fn culling_leaves_out_the_instances_outside_the_camera_s_view() {
        // The default camera at z = 5 looking at the origin, 60 degrees, square target; a model
        // whose culling bounds are a 1 m cube round its origin.
        let volume = CullVolume::from_camera(&Camera::default(), 1.0);
        let bounds = (Vec3::splat(-0.5), Vec3::splat(0.5));
        let at = |x: f32, y: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z));
        let culled: Vec<bool> = [at(0.0, 0.0, 0.0), at(2.0, 0.0, 0.0), at(0.0, 0.0, 7.0), at(30.0, 0.0, 0.0), at(0.0, -30.0, -10.0), at(0.0, 0.0, -2000.0)].iter().map(|world| is_culled(&volume, true, false, bounds, *world)).collect();
        // In view; in view; behind the camera; far right; far below; beyond the far plane.
        assert_eq!(culled, vec![false, false, true, true, true, true]);
        // Scaled up, the instance far to the right reaches back into the view.
        assert!(!is_culled(&volume, true, false, bounds, at(30.0, 0.0, 0.0) * Mat4::from_scale(Vec3::splat(60.0))));
        // Never cull, and culling off.
        assert!(!is_culled(&volume, true, true, bounds, at(30.0, 0.0, 0.0)));
        assert!(!is_culled(&volume, false, false, bounds, at(30.0, 0.0, 0.0)));
    }

    #[test]
    fn culling_tests_the_grown_bounds_not_the_rest_pose() {
        // The fixture model's culling bounds are wider than its rest bounds (its root moves): an
        // instance whose rest box is just out of view but whose clips bring it into view stays.
        let set = crate::models::ModelDataSet::read(include_bytes!("../../fixtures/every-section.cbmd")).unwrap();
        let model = &set.models[0];
        let (rest, grown) = (model.bounds(), model.culling_bounds());
        let volume = CullVolume::from_planes([Vec4::new(1.0, 0.0, 0.0, 0.0)]); // keeps x >= 0
        let reach = grown.1.x - rest.1.x;
        assert!(reach > 0.0, "{rest:?} {grown:?}");
        let world = Mat4::from_translation(Vec3::new(-rest.1.x - reach * 0.5, 0.0, 0.0));
        assert!(is_culled(&volume, true, false, rest, world));
        assert!(!is_culled(&volume, true, false, grown, world));
    }

    #[test]
    fn each_pass_culls_against_its_own_volume_so_a_caster_behind_the_camera_still_casts() {
        // The default camera at z = 5 looking at -Z; the sun straight down over a 40 m region
        // round the origin (which reaches behind the camera, to z = +20).
        let view = CullVolume::from_camera(&Camera::default(), 1.0);
        let sun = crate::lighting::shadows::sun_shadow_view(Vec3::NEG_Y, Vec3::splat(-20.0), Vec3::splat(20.0), 2048).unwrap();
        let volumes = [&sun.volume];
        let bounds = (Vec3::splat(-0.5), Vec3::splat(0.5));
        let at = |x: f32, y: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z));
        // In front of the camera: both passes.
        assert_eq!(cull_passes(&view, &volumes, true, false, true, bounds, at(0.0, 0.0, 0.0)), (true, 0b1));
        // Behind the camera, inside the sun's region: the shadow pass only.
        assert_eq!(cull_passes(&view, &volumes, true, false, true, bounds, at(0.0, 0.0, 12.0)), (false, 0b1));
        // High above the region, between it and the sun: still casts into it.
        assert_eq!(cull_passes(&view, &volumes, true, false, true, bounds, at(0.0, 60.0, 12.0)).1, 0b1);
        // Beside the region and behind the camera: in no pass (left out entirely).
        assert_eq!(cull_passes(&view, &volumes, true, false, true, bounds, at(300.0, 0.0, 12.0)), (false, 0));
        // An instance that does not cast is in no shadow pass, wherever it is.
        assert_eq!(cull_passes(&view, &volumes, true, false, false, bounds, at(0.0, 0.0, 0.0)), (true, 0));
        // Never cull / culling off: every pass.
        assert_eq!(cull_passes(&view, &volumes, true, true, true, bounds, at(300.0, 0.0, 12.0)), (true, 0b1));
        assert_eq!(cull_passes(&view, &volumes, false, false, true, bounds, at(300.0, 0.0, 12.0)), (true, 0b1));
        // Several passes: one bit each.
        let spot = crate::lighting::shadows::spot_shadow_view(&crate::lighting::SpotLight::new(Vec3::new(300.0, 5.0, 12.0), Vec3::NEG_Y, crate::Color4::WHITE, 1.0, 10.0, 0.3, 0.6), 1000.0, 1024).unwrap();
        assert_eq!(cull_passes(&view, &[&sun.volume, &spot.volume], true, false, true, bounds, at(300.0, 0.0, 12.0)), (false, 0b10));
    }

    #[test]
    fn the_shadow_passes_draw_the_casting_opaque_parts_and_the_camera_pass_only_what_it_sees() {
        // Parts: opaque on node 0, blended on node 0, opaque on node 1.
        let draws = [part(0, false, Vec3::ZERO), part(0, true, Vec3::ZERO), part(1, false, Vec3::ZERO)];
        let worlds = [Mat4::IDENTITY; 5];
        let mut buckets = [bucket(1, 2, &worlds)];
        let b = &mut buckets[0];
        // 0: seen and casts into both passes; 1: behind the camera, casts into pass 0 only; 2: seen,
        // casts nowhere; 3: seen, faded (alpha below 1: drawn blended, never casts); 4: seen, casts
        // into pass 1, node 1 hidden, does not receive shadows.
        b.shadow_mask = vec![0b11, 0b01, 0, 0b11, 0b10];
        b.in_view = vec![true, false, true, true, true];
        b.looks[3] = InstanceLook { tint: Vec4::new(1.0, 1.0, 1.0, 0.5), emissive: 1.0, receives_shadows: true, dither: 0 };
        b.hidden[4] = Some(vec![false, true].into_boxed_slice());
        b.looks[4].receives_shadows = false;
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 2)], position, forward, true, 2);
        // Record order: seen and plain (0, 2), seen with a fade or hidden node (3, 4), then unseen (1).
        let record = |instance: usize| [0usize, 2, 3, 4, 1].iter().position(|&i| i == instance).unwrap() as u32;
        let worlds_of = |draws: &[PlannedDraw], draw: usize| -> Vec<u32> { draws.iter().filter(|d| d.draw == draw).flat_map(|d| d.first_instance..d.first_instance + d.count).collect() };
        // The camera's pass never draws instance 1.
        assert!(plan.opaque.iter().chain(&plan.blended).all(|d| !(d.first_instance..d.first_instance + d.count).contains(&record(1))));
        // Pass 0: instances 0 and 1, both opaque parts; never the blended part, never the faded one.
        assert_eq!(worlds_of(&plan.shadows[0], 0), vec![record(0), record(1)]);
        assert_eq!(worlds_of(&plan.shadows[0], 2), vec![record(0), record(1)]);
        assert!(plan.shadows.iter().flatten().all(|d| d.draw != 1 && d.side == FaceSide::Both && !d.mirrored));
        // Pass 1: instance 0, and instance 4 without its hidden node's part.
        assert_eq!(worlds_of(&plan.shadows[1], 0), vec![record(0), record(4)]);
        assert_eq!(worlds_of(&plan.shadows[1], 2), vec![record(0)]);
        // The receive flag is in the record.
        assert_eq!(plan.instances[record(4) as usize].flags[0], 0);
        assert_eq!(plan.instances[record(0) as usize].flags[0], 1);
        // No shadow passes: no shadow draws, and the plan is what it was.
        let plain = super::plan(&buckets, &[model(&draws, 2)], position, forward, true, 0);
        assert!(plain.shadows.is_empty());
        assert_eq!(plain.opaque, plan.opaque);
    }

    #[test]
    fn a_fading_level_of_detail_draws_in_runs_of_its_own_with_its_code_in_the_record() {
        // Five instances of a model with an opaque and a blended part, all casting into one shadow
        // pass: 0, 2 and 4 plain, 1 and 3 a level fading in at step 32 (code 0x120).
        let draws = [part(0, false, Vec3::ZERO), part(0, true, Vec3::ZERO)];
        let mut buckets = [bucket(1, 1, &[Mat4::IDENTITY; 5])];
        buckets[0].looks[1].dither = 0x120;
        buckets[0].looks[3].dither = 0x120;
        buckets[0].shadow_mask = vec![1; 5];
        let (position, forward) = camera();
        let plan = plan(&buckets, &[model(&draws, 1)], position, forward, true, 1);
        // The plain instances first (records 0..3), then the fading ones (records 3, 4).
        let runs = |draws: &[PlannedDraw]| -> Vec<(u32, u32, bool)> { draws.iter().filter(|d| d.draw == 0).map(|d| (d.first_instance, d.count, d.dithered)).collect() };
        assert_eq!(runs(&plan.opaque), vec![(0, 3, false), (3, 2, true)]);
        assert_eq!(runs(&plan.shadows[0]), vec![(0, 3, false), (3, 2, true)]);
        assert_eq!(plan.instances.iter().map(|r| r.flags[1]).collect::<Vec<u32>>(), vec![0, 0, 0, 0x120, 0x120]);
        // The blended part of a fading instance carries the flag too (it fades by alpha).
        assert_eq!(plan.blended.len(), 5);
        assert!(plan.blended.iter().all(|d| d.dithered == (d.first_instance >= 3)));
        // No dither anywhere: one run, as before.
        let plain = super::plan(&[bucket(1, 1, &[Mat4::IDENTITY; 5])], &[model(&draws, 1)], position, forward, true, 0);
        assert_eq!(runs(&plain.opaque), vec![(0, 5, false)]);
    }

    #[test]
    fn runs_group_equal_values() {
        assert_eq!(runs([false, true, true, false].into_iter()), vec![(0, 1, false), (1, 3, true), (3, 4, false)]);
        assert_eq!(runs([Some(false), None, None, Some(false)].into_iter()), vec![(0, 1, Some(false)), (1, 3, None), (3, 4, Some(false))]);
    }
}
