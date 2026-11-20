//! [`MeshData`]: CPU-side mesh data - vertices and 32-bit triangle-list indices - ready for
//! [`GraphicsContext::create_mesh`](crate::GraphicsContext::create_mesh) (C#: `MeshData`).

use glam::{Mat4, Vec3};

use crate::drawing3d::MeshVertex;

/// Vertices and triangle-list indices with counter-clockwise front faces (C#: `MeshData`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    /// The vertices (C#: `Vertices`).
    pub vertices: Vec<MeshVertex>,
    /// The triangle-list indices (C#: `Indices`).
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Mesh data from its parts (C#: `new MeshData(vertices, indices)`).
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> MeshData {
        MeshData { vertices, indices }
    }

    /// The axis-aligned bounds of the positions, minimum and maximum corner; both zero for no
    /// vertices (C#: `GetBounds(out min, out max)`).
    pub fn get_bounds(&self) -> (Vec3, Vec3) {
        bounds(&self.vertices)
    }

    /// A copy with every vertex transformed: positions by the matrix, normals by its inverse
    /// transpose, renormalised (C#: `Transformed`).
    pub fn transformed(&self, transform: Mat4) -> MeshData {
        let normal_matrix = transform.inverse().transpose();
        let vertices = self
            .vertices
            .iter()
            .map(|vertex| MeshVertex { position: transform.transform_point3(vertex.position), normal: normal_matrix.transform_vector3(vertex.normal).normalize_or_zero(), ..*vertex })
            .collect();
        MeshData::new(vertices, self.indices.clone())
    }

    /// Several meshes joined into one, their indices rebased (C#: `Combine`).
    pub fn combine<'a>(parts: impl IntoIterator<Item = &'a MeshData>) -> MeshData {
        let mut combined = MeshData::default();
        for part in parts {
            let base = combined.vertices.len() as u32;
            combined.vertices.extend_from_slice(&part.vertices);
            combined.indices.extend(part.indices.iter().map(|index| index + base));
        }
        combined
    }
}

/// The bounds of some vertices (both corners zero for none).
pub(crate) fn bounds(vertices: &[MeshVertex]) -> (Vec3, Vec3) {
    if vertices.is_empty() {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    vertices.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(min, max), vertex| (min.min(vertex.position), max.max(vertex.position)))
}
