//! [`ModelPoseEdit`]: an open pose edit of a [`ModelInstance`] (C#: `ModelPoseEdit`).

use crate::models::{ModelInstance, NodeTransform};

/// An open pose edit of a [`ModelInstance`] ([`ModelInstance::begin_pose_edit`]; C#:
/// `ModelPoseEdit`): write local transforms and morph weights in place, then let it go out of
/// scope - the pose counts as changed when it is dropped (C#: `Dispose`, a `using` statement).
///
/// ```ignore
/// {
///     let mut pose = instance.begin_pose_edit();
///     pose.local_transforms()[arm].rotation = Quat::from_axis_angle(Vec3::Z, angle);
///     pose.get_morph_weights(face)[0] = 1.0;
/// } // the edit ends here
/// ```
pub struct ModelPoseEdit<'a> {
    instance: &'a mut ModelInstance,
}

impl<'a> ModelPoseEdit<'a> {
    pub(crate) fn new(instance: &'a mut ModelInstance) -> ModelPoseEdit<'a> {
        ModelPoseEdit { instance }
    }

    /// The model of the instance being edited (C#: `Instance.Model`).
    pub fn model(&self) -> &crate::Model {
        self.instance.model()
    }

    /// Every node's local transform, writable in place, index = node index (C#:
    /// `LocalTransforms`).
    pub fn local_transforms(&mut self) -> &mut [NodeTransform] {
        self.instance.locals_mut()
    }

    /// A node's morph weights, writable in place; empty when the node's mesh has no morph targets
    /// (C#: `GetMorphWeights`).
    pub fn get_morph_weights(&mut self, node: usize) -> &mut [f32] {
        self.instance.morph_weights_mut(node)
    }
}

impl Drop for ModelPoseEdit<'_> {
    /// Ends the edit: the pose counts as changed (C#: `Dispose`).
    fn drop(&mut self) {
        self.instance.end_pose_edit();
    }
}
