//! [`ModelInstance`]: one placement of a [`Model`] with its own pose (C#: `ModelInstance`).

use std::cell::{Ref, RefCell};

use glam::{Mat4, Vec3};

use crate::animation::{AnimationPlayer, PlayerData, PoseSource, PoseTarget};
use crate::models::{Model, ModelPoseEdit, NodeTransform};

/// The pose-derived data, recomputed on the next read after a change.
struct Posed {
    valid: bool,
    node_matrices: Vec<Mat4>,
    palette: Vec<Mat4>,
}

/// One placement of a [`Model`] with its own state (C#: `ModelInstance`): a world transform, every
/// node's local transform (translation, rotation, scale - what animation playback drives), the
/// node matrices computed from them, and per-node morph weights. Make any number per model
/// ([`Model::create_instance`]); an instance holds no GPU objects and is drawn by a
/// [`ModelRenderer`](crate::ModelRenderer). Pose changes go through
/// [`ModelInstance::set_local_transform`], [`ModelInstance::set_morph_weight`],
/// [`ModelInstance::begin_pose_edit`] or the instance's player ([`ModelInstance::animation`]), so
/// the derived node matrices and joint palettes are always recomputed after a change.
///
/// C#'s rule "no reading, drawing or animating while a pose edit is open" needs no check here:
/// the edit borrows the instance mutably, so Rust refuses such code when it is compiled.
pub struct ModelInstance {
    model: Model,
    /// The model-to-world matrix; default identity (C#: `World`).
    pub world: Mat4,
    /// False to skip the instance when drawing; default true (C#: `Visible`). Hide single nodes
    /// with [`ModelInstance::set_node_visible`].
    pub visible: bool,
    /// Multiplies the base colour of every part (linear RGB); default white, no change. Not in C#.
    pub tint: Vec3,
    /// Multiplies the opacity of every part, 0 to 1; default 1. Below 1 EVERY part of the instance
    /// is drawn with blending among the blended parts (by
    /// [`ModelRenderer::end_blended`](crate::ModelRenderer::end_blended), sorted back to front,
    /// depth read but not written), so the instance can fade out; 0 or less skips the instance.
    /// Not in C#.
    pub alpha: f32,
    /// Multiplies the emissive of every part (on top of the material's emissive strength); default
    /// 1. A part brightens or dims without a second material or model. Not in C#.
    pub emissive_strength: f32,
    /// True to draw the instance even when it is outside the view: the renderer's culling skips
    /// it ([`ModelRenderer::set_frustum_culling`](crate::ModelRenderer::set_frustum_culling)
    /// turns culling off for all). Default false. For an instance posed by hand further than its
    /// model's clips reach ([`Model::culling_bounds`] hold the clips' poses only), or one whose
    /// vertices a shader of the application moves. Not in C#.
    pub never_cull: bool,
    /// True (default) when the instance casts shadows - from the sun and from the spotlights that
    /// cast them ([`SceneLighting::sun_shadows`](crate::SceneLighting::sun_shadows),
    /// [`SpotLight::casts_shadows`](crate::SpotLight::casts_shadows)). Only its opaque and
    /// alpha-tested parts cast (an alpha-tested part with its cutout); blended parts never do, nor
    /// does the whole instance while its [`ModelInstance::alpha`] is below 1. A skinned or
    /// morphing instance casts in its current pose. Not in C#.
    pub casts_shadows: bool,
    /// True (default) when shadows fall on the instance. Not in C#.
    pub receives_shadows: bool,
    /// Per node: true when hidden by [`ModelInstance::set_node_visible`]; empty while none is.
    hidden_nodes: Vec<bool>,
    locals: Vec<NodeTransform>,
    morph_weights: Vec<f32>,
    posed: RefCell<Posed>,
    pose_version: u64,
    player: Option<Box<PlayerData>>,
}

impl ModelInstance {
    pub(crate) fn new(model: Model) -> ModelInstance {
        let nodes = model.nodes().len();
        let layout = model.layout();
        let mut instance = ModelInstance {
            world: Mat4::IDENTITY,
            visible: true,
            tint: Vec3::ONE,
            alpha: 1.0,
            emissive_strength: 1.0,
            never_cull: false,
            casts_shadows: true,
            receives_shadows: true,
            hidden_nodes: Vec::new(),
            locals: vec![NodeTransform::IDENTITY; nodes],
            morph_weights: vec![0.0; layout.morph_weight_count],
            posed: RefCell::new(Posed { valid: false, node_matrices: vec![Mat4::IDENTITY; nodes], palette: vec![Mat4::IDENTITY; layout.palette_size] }),
            pose_version: 0,
            player: None,
            model,
        };
        instance.reset_pose();
        instance
    }

    /// The model (C#: `Model`).
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Every node's local transform, index = node index (C#: `LocalTransforms`). Nodes with a
    /// fixed matrix ignore their entry.
    pub fn local_transforms(&self) -> &[NodeTransform] {
        &self.locals
    }

    /// A node's local transform (C#: `GetLocalTransform`).
    pub fn get_local_transform(&self, node: usize) -> NodeTransform {
        self.locals[node]
    }

    /// Sets a node's local transform (C#: `SetLocalTransform`).
    pub fn set_local_transform(&mut self, node: usize, transform: NodeTransform) {
        self.locals[node] = transform;
        self.mark_pose_changed();
    }

    /// This instance's animation player (C#: `Animation`; its playback state is made on first
    /// use, so instances that never animate pay nothing). Update it once per frame with the
    /// frame's time or delta, or many instances at once with
    /// [`AnimationPlayer::update_all_to`] / [`AnimationPlayer::update_all`].
    pub fn animation(&mut self) -> AnimationPlayer<'_> {
        let model = &self.model;
        let data = self.player.get_or_insert_with(|| Box::new(PlayerData::new(model.animations())));
        let layout = model.layout();
        let source = PoseSource { nodes: model.nodes(), animations: model.animations(), morph_offsets: &layout.morph_weight_offsets, morph_count: layout.morph_weight_count };
        let target = PoseTarget { locals: &mut self.locals, weights: &mut self.morph_weights, posed: &mut self.posed.get_mut().valid, version: &mut self.pose_version };
        AnimationPlayer::new(data, source, target)
    }

    /// True once [`ModelInstance::animation`] has been used (C#: `HasAnimationPlayer`).
    pub fn has_animation_player(&self) -> bool {
        self.player.is_some()
    }

    /// Opens a pose edit (C#: `BeginPoseEdit`): the returned editor exposes every local transform
    /// and morph weight for writing in place; the pose counts as changed when it is dropped (C#:
    /// `Dispose`). While it lives, the instance cannot be read, drawn or animated - the borrow
    /// checker sees to that.
    pub fn begin_pose_edit(&mut self) -> ModelPoseEdit<'_> {
        self.mark_pose_changed();
        ModelPoseEdit::new(self)
    }

    /// The writable local transforms and a node's writable morph weights, for a pose edit.
    pub(crate) fn locals_mut(&mut self) -> &mut [NodeTransform] {
        &mut self.locals
    }

    /// A node's writable morph weights, empty without morph targets.
    pub(crate) fn morph_weights_mut(&mut self, node: usize) -> &mut [f32] {
        match self.morph_range(node) {
            Some(range) => &mut self.morph_weights[range],
            None => &mut [],
        }
    }

    /// Ends a pose edit: the pose counts as changed.
    pub(crate) fn end_pose_edit(&mut self) {
        self.mark_pose_changed();
    }

    /// Increments whenever the pose may have changed (C#: `PoseVersion`).
    pub fn pose_version(&self) -> u64 {
        self.pose_version
    }

    /// Every node's matrix in model space (node-to-model; multiply by [`ModelInstance::world`]
    /// for world space), recomputed first when the pose changed (C#: `NodeMatrices`).
    pub fn node_matrices(&self) -> Ref<'_, [Mat4]> {
        self.ensure_posed();
        Ref::map(self.posed.borrow(), |p| p.node_matrices.as_slice())
    }

    /// A node's matrix in world space, its CURRENT place after the last pose change (C#:
    /// `GetNodeWorldMatrix`); panics for a node index out of range. By name:
    /// [`ModelInstance::get_node_world_matrix_named`].
    pub fn get_node_world_matrix(&self, node: usize) -> Mat4 {
        self.world * self.node_matrices()[node]
    }

    /// A node's matrix in world space by the node's NAME (the first node of that name), None for
    /// no such node (Not in C#). Read it after the frame's animation update to ride on an animated
    /// node (a camera, a glow, an emitter): it is the node's CURRENT place, computed from the
    /// current pose. [`Model::animated_nodes`] lists the nodes the clips move.
    pub fn get_node_world_matrix_named(&self, name: &str) -> Option<Mat4> {
        self.model.find_node(name).map(|node| self.get_node_world_matrix(node))
    }

    /// Shows or hides one node of this instance (Not in C#): a hidden node's parts are not drawn,
    /// nor those of its descendants; other instances of the model are not affected. Its pose is
    /// still computed (its children's and its joints' matrices stay valid). Out-of-range nodes are
    /// ignored. The instance's other parts stay batched with the other instances: only the hidden
    /// parts leave the shared draw.
    pub fn set_node_visible(&mut self, node: usize, visible: bool) {
        if node >= self.locals.len() || (visible && self.hidden_nodes.is_empty()) {
            return;
        }
        if self.hidden_nodes.is_empty() {
            self.hidden_nodes = vec![false; self.locals.len()];
        }
        self.hidden_nodes[node] = !visible;
        if self.hidden_nodes.iter().all(|h| !h) {
            self.hidden_nodes.clear();
        }
    }

    /// Shows or hides a node by name (the first node of that name); false when the model has no
    /// such node (Not in C#).
    pub fn set_node_visible_named(&mut self, name: &str, visible: bool) -> bool {
        match self.model.find_node(name) {
            Some(node) => {
                self.set_node_visible(node, visible);
                true
            }
            None => false,
        }
    }

    /// False when the node itself was hidden with [`ModelInstance::set_node_visible`] (a shown
    /// node under a hidden one is still not drawn) (Not in C#).
    pub fn is_node_visible(&self, node: usize) -> bool {
        !self.hidden_nodes.get(node).copied().unwrap_or(false)
    }

    /// Shows every node again (Not in C#).
    pub fn show_all_nodes(&mut self) {
        self.hidden_nodes.clear();
    }

    /// Per node, true when it is not drawn: hidden itself or under a hidden node; None while every
    /// node shows.
    pub(crate) fn hidden_node_mask(&self) -> Option<Box<[bool]>> {
        if self.hidden_nodes.is_empty() {
            return None;
        }
        let nodes = self.model.nodes();
        let mut hidden = self.hidden_nodes.clone();
        for &index in &self.model.layout().node_order {
            if let Some(parent) = nodes[index].parent {
                hidden[index] |= hidden[parent];
            }
        }
        Some(hidden.into_boxed_slice())
    }

    /// A node's current morph weights, empty when its mesh has none (C#: `GetMorphWeights`).
    pub fn get_morph_weights(&self, node: usize) -> &[f32] {
        match self.morph_range(node) {
            Some(range) => &self.morph_weights[range],
            None => &[],
        }
    }

    /// Sets one morph weight of a node; false (and nothing changes) when the node has no such
    /// morph target (C#: `SetMorphWeight`, which throws).
    pub fn set_morph_weight(&mut self, node: usize, target: usize, weight: f32) -> bool {
        let Some(range) = self.morph_range(node) else {
            return false;
        };
        if target >= range.len() {
            return false;
        }
        self.morph_weights[range.start + target] = weight;
        self.mark_pose_changed();
        true
    }

    /// Sets a node's morph weights - the first `min(weights.len(), count)` of them (C#:
    /// `SetMorphWeights`).
    pub fn set_morph_weights(&mut self, node: usize, weights: &[f32]) {
        if let Some(range) = self.morph_range(node) {
            let count = range.len().min(weights.len());
            self.morph_weights[range.start..range.start + count].copy_from_slice(&weights[..count]);
            self.mark_pose_changed();
        }
    }

    /// A skin's joint palette in model space: for joint j, `NodeMatrices[Joints[j]] *
    /// InverseBindMatrices[j]` in glam's order (C#: `GetJointPalette`); None for a skin that does
    /// not exist.
    pub fn get_joint_palette(&self, skin: usize) -> Option<Vec<Mat4>> {
        let skins = self.model.skins();
        let data = skins.get(skin)?;
        self.ensure_posed();
        let offset = self.model.layout().skin_palette_offsets[skin];
        Some(self.posed.borrow().palette[offset..offset + data.joint_count()].to_vec())
    }

    /// Returns every node to its rest pose and every morph weight to its default (C#: `ResetPose`).
    pub fn reset_pose(&mut self) {
        let model = self.model.clone();
        let layout = model.layout();
        for (index, node) in model.nodes().iter().enumerate() {
            self.locals[index] = node.default_transform;
            if let Some(offset) = layout.morph_weight_offsets[index] {
                let defaults = &node.default_morph_weights;
                let end = (offset + defaults.len()).min(self.morph_weights.len());
                self.morph_weights[offset..end].copy_from_slice(&defaults[..end - offset]);
            }
        }
        self.mark_pose_changed();
    }

    /// Recomputes the node matrices from the local transforms, parents first (C#:
    /// `UpdateNodeMatrices`). Reading them does this when needed, so a call is never required.
    pub fn update_node_matrices(&self) {
        let mut posed = self.posed.borrow_mut();
        let model = &self.model;
        for &index in &model.layout().node_order {
            let node = &model.nodes()[index];
            let local = if node.has_fixed_matrix { node.local_matrix } else { self.locals[index].to_matrix() };
            posed.node_matrices[index] = match node.parent {
                Some(parent) => posed.node_matrices[parent] * local,
                None => local,
            };
        }
        for (skin, data) in model.skins().iter().enumerate() {
            let offset = model.layout().skin_palette_offsets[skin];
            for (joint, &node) in data.joints.iter().enumerate() {
                let inverse_bind = data.inverse_bind_matrices.get(joint).copied().unwrap_or(Mat4::IDENTITY);
                posed.palette[offset + joint] = posed.node_matrices[node] * inverse_bind;
            }
        }
        posed.valid = true;
    }

    /// Every morph weight, laid out by the model's morph-weight offsets.
    pub(crate) fn morph_weight_array(&self) -> &[f32] {
        &self.morph_weights
    }

    /// Every skin's palette, skin after skin.
    pub(crate) fn joint_palettes(&self) -> Ref<'_, [Mat4]> {
        self.ensure_posed();
        Ref::map(self.posed.borrow(), |p| p.palette.as_slice())
    }

    fn ensure_posed(&self) {
        if !self.posed.borrow().valid {
            self.update_node_matrices();
        }
    }

    fn mark_pose_changed(&mut self) {
        self.posed.get_mut().valid = false;
        self.pose_version += 1;
    }

    fn morph_range(&self, node: usize) -> Option<std::ops::Range<usize>> {
        let offset = (*self.model.layout().morph_weight_offsets.get(node)?)?;
        let count = self.model.nodes()[node].default_morph_weights.len();
        Some(offset..offset + count)
    }
}
