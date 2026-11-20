//! [`ModelMesh`], [`ModelPrimitive`] and [`ModelMorphTarget`]: a model's meshes, their drawable
//! parts and morph targets (C#: the types of the same names). Immutable data.

use glam::Vec3;

/// A glTF mesh of a [`Model`](crate::Model): its primitives and default morph weights (C#:
/// `ModelMesh`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelMesh {
    /// The index in the model's meshes (C#: `Index`).
    pub index: usize,
    /// The name, or None (C#: `Name`).
    pub name: Option<String>,
    /// The primitives that were loaded - triangle lists (C#: `Primitives`).
    pub primitives: Vec<ModelPrimitive>,
    /// The mesh's default morph-target weights, empty without morph targets (C#:
    /// `DefaultMorphWeights`).
    pub default_morph_weights: Vec<f32>,
}

impl ModelMesh {
    /// The number of morph targets, the same for every primitive (C#: `MorphTargetCount`).
    pub fn morph_target_count(&self) -> usize {
        self.default_morph_weights.len()
    }
}

/// One drawable part of a [`ModelMesh`]: an index range in the model's shared buffers and a
/// material (C#: `ModelPrimitive`). Indices are relative to [`ModelPrimitive::base_vertex`]; draw
/// with `draw_indexed(index_count, first_index, base_vertex)` after binding the model's vertex and
/// index buffers.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPrimitive {
    /// The index in its mesh's primitives (C#: `Index`).
    pub index: usize,
    /// The material's index in the model's materials, or None for the model's default material
    /// (C#: `Material`, an object; read it with [`Model::primitive_material`](crate::Model::primitive_material)).
    pub material: Option<usize>,
    /// The first index in the model's index buffer (C#: `FirstIndex`).
    pub first_index: u32,
    /// The number of indices, a multiple of 3 (C#: `IndexCount`).
    pub index_count: u32,
    /// The primitive's first vertex in the model's vertex buffer, added to every index (C#:
    /// `BaseVertex`).
    pub base_vertex: u32,
    /// The number of vertices (C#: `VertexCount`).
    pub vertex_count: u32,
    /// The first vertex in the model's skin buffer, or None when the primitive is not skinned
    /// (C#: `SkinBaseVertex`, -1 for none).
    pub skin_base_vertex: Option<u32>,
    /// The minimum corner of the mesh-space bounds, default morph weights applied (C#: `BoundsMin`).
    pub bounds_min: Vec3,
    /// The maximum corner of the mesh-space bounds (C#: `BoundsMax`).
    pub bounds_max: Vec3,
    /// True when the file has COLOR_0 (otherwise the colour is white) (C#: `HasVertexColors`).
    pub has_vertex_colors: bool,
    /// True when the file has TEXCOORD_1 (C#: `HasTexCoord1`).
    pub has_tex_coord1: bool,
    /// True when the file has TANGENT or tangents were generated (C#: `HasTangents`).
    pub has_tangents: bool,
    /// True when the tangents were generated when the model data was made (C#: `TangentsGenerated`).
    pub tangents_generated: bool,
    /// True when the file had no NORMAL and smooth normals were generated (C#: `NormalsGenerated`).
    pub normals_generated: bool,
    /// The morph targets; the vertex buffer holds the shape with the mesh's default weights
    /// applied, so a renderer applies `(weight - default) * delta` (C#: `MorphTargets`).
    pub morph_targets: Vec<ModelMorphTarget>,
}

impl ModelPrimitive {
    /// True when the primitive has JOINTS_0 and WEIGHTS_0 (C#: `IsSkinned`).
    pub fn is_skinned(&self) -> bool {
        self.skin_base_vertex.is_some()
    }

    /// The number of triangles (C#: `TriangleCount`).
    pub fn triangle_count(&self) -> u32 {
        self.index_count / 3
    }
}

/// One morph target of a [`ModelPrimitive`]: deltas per vertex (C#: `ModelMorphTarget`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelMorphTarget {
    /// Position deltas, one per vertex, or empty (C#: `PositionDeltas`).
    pub position_deltas: Vec<Vec3>,
    /// Normal deltas, one per vertex, or empty (C#: `NormalDeltas`).
    pub normal_deltas: Vec<Vec3>,
    /// Tangent deltas, one per vertex, or empty (C#: `TangentDeltas`).
    pub tangent_deltas: Vec<Vec3>,
}
