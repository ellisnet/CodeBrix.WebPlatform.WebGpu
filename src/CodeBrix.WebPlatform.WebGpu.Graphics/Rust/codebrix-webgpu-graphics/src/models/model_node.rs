//! [`NodeTransform`], [`ModelNode`] and [`ModelSkin`]: a model's node hierarchy and skins (C#:
//! the types of the same names). Immutable data; the per-instance pose lives in
//! [`ModelInstance`](crate::ModelInstance).

use glam::{Mat4, Quat, Vec3};

/// A node's local transform as translation, rotation and scale - the form glTF animation drives
/// (C#: `NodeTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeTransform {
    /// The translation (C#: `Translation`).
    pub translation: Vec3,
    /// The rotation, a unit quaternion (C#: `Rotation`).
    pub rotation: Quat,
    /// The scale (C#: `Scale`).
    pub scale: Vec3,
}

impl NodeTransform {
    /// No translation, no rotation, unit scale (C#: `Identity`).
    pub const IDENTITY: NodeTransform = NodeTransform { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };

    /// A transform (C#: `new NodeTransform(translation, rotation, scale)`).
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> NodeTransform {
        NodeTransform { translation, rotation, scale }
    }

    /// The matrix: scale, then rotation, then translation (C#: `ToMatrix()`; the same bytes as
    /// C#'s row-vector matrix).
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for NodeTransform {
    fn default() -> NodeTransform {
        NodeTransform::IDENTITY
    }
}

/// A node of a [`Model`](crate::Model)'s hierarchy (C#: `ModelNode`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelNode {
    /// The index in the model's nodes, the glTF node index (C#: `Index`).
    pub index: usize,
    /// The name, or None (C#: `Name`).
    pub name: Option<String>,
    /// The parent node, or None for a root (C#: `Parent`, -1 for none).
    pub parent: Option<usize>,
    /// The child nodes (C#: `Children`).
    pub children: Vec<usize>,
    /// The mesh, or None (C#: `MeshIndex`).
    pub mesh: Option<usize>,
    /// The skin, or None (C#: `SkinIndex`).
    pub skin: Option<usize>,
    /// The rest pose, from the file's TRS or the decomposed matrix (C#: `DefaultTransform`).
    pub default_transform: NodeTransform,
    /// True when the file gave a matrix that does not decompose into translation, rotation and
    /// scale (a shear, say): instances then use [`ModelNode::local_matrix`] as it is (C#:
    /// `HasFixedMatrix`).
    pub has_fixed_matrix: bool,
    /// The rest-pose local matrix (C#: `LocalMatrix`).
    pub local_matrix: Mat4,
    /// The node's default morph weights - its own, else its mesh's; empty without morph targets
    /// (C#: `DefaultMorphWeights`).
    pub default_morph_weights: Vec<f32>,
    /// True when the node is in the model's scene and drawn (C#: `InScene`).
    pub in_scene: bool,
}

impl std::fmt::Display for ModelNode {
    /// The index and name, for example `ModelNode 3 'arm'` (C#: `ToString`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "ModelNode {} '{}'", self.index, name),
            None => write!(f, "ModelNode {}", self.index),
        }
    }
}

/// A glTF skin: joint nodes and inverse bind matrices (C#: `ModelSkin`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelSkin {
    /// The index in the model's skins (C#: `Index`).
    pub index: usize,
    /// The name, or None (C#: `Name`).
    pub name: Option<String>,
    /// The joint node indices; skin-vertex joint numbers index this list (C#: `Joints`).
    pub joints: Vec<usize>,
    /// One inverse bind matrix per joint (C#: `InverseBindMatrices`).
    pub inverse_bind_matrices: Vec<Mat4>,
    /// The skeleton root node, or None (C#: `Skeleton`).
    pub skeleton: Option<usize>,
}

impl ModelSkin {
    /// The number of joints (C#: `JointCount`).
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
}

#[cfg(test)]
mod tests {
    use glam::{Quat, Vec3};

    use super::NodeTransform;

    #[test]
    fn the_matrix_scales_then_rotates_then_translates() {
        let transform = NodeTransform::new(Vec3::new(1.0, 2.0, 3.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), Vec3::splat(2.0));
        let point = transform.to_matrix().transform_point3(Vec3::X);
        assert!(point.distance(Vec3::new(1.0, 2.0, 1.0)) < 1e-5);
        assert_eq!(NodeTransform::default().to_matrix(), glam::Mat4::IDENTITY);
    }
}
