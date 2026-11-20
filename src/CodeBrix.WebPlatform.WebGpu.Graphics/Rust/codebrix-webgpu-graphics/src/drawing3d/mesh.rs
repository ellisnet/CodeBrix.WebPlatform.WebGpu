//! [`Mesh`]: a mesh on the GPU - a vertex buffer of [`MeshVertex`] and a 32-bit index buffer -
//! and the context's factories for it (C#: `Mesh`, `GraphicsContext.CreateMesh`).

use glam::Vec3;

use crate::context::{GraphicsContext, GraphicsException};
use crate::drawing3d::mesh_data::bounds;
use crate::drawing3d::{MeshData, MeshVertex};
use crate::resources::GpuBuffer;

/// A mesh on the GPU (C#: `Mesh`): a vertex buffer of [`MeshVertex`] and a 32-bit index buffer
/// (a triangle list). Made by [`GraphicsContext::create_mesh`]; drawn by a
/// [`MeshRenderer`](crate::MeshRenderer); dropped to release it.
pub struct Mesh {
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    vertex_count: usize,
    index_count: usize,
    bounds_min: Vec3,
    bounds_max: Vec3,
}

impl Mesh {
    /// The vertex buffer (C#: `VertexBuffer`).
    pub fn vertex_buffer(&self) -> &GpuBuffer {
        &self.vertex_buffer
    }

    /// The index buffer, 32-bit indices (C#: `IndexBuffer`).
    pub fn index_buffer(&self) -> &GpuBuffer {
        &self.index_buffer
    }

    /// The number of vertices (C#: `VertexCount`).
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    /// The number of indices (C#: `IndexCount`).
    pub fn index_count(&self) -> usize {
        self.index_count
    }

    /// The minimum corner of the model-space bounds (C#: `BoundsMin`).
    pub fn bounds_min(&self) -> Vec3 {
        self.bounds_min
    }

    /// The maximum corner of the model-space bounds (C#: `BoundsMax`).
    pub fn bounds_max(&self) -> Vec3 {
        self.bounds_max
    }
}

impl GraphicsContext {
    /// A mesh from mesh data (C#: `CreateMesh(MeshData data, string label = null)`).
    pub fn create_mesh(&self, data: &MeshData, label: Option<&str>) -> Result<Mesh, GraphicsException> {
        self.create_mesh_from(&data.vertices, &data.indices, label)
    }

    /// A mesh from vertices and triangle-list indices (C#: `CreateMesh(vertices, indices,
    /// label)`). An error for no vertices, an index count that is 0 or not a multiple of 3, or an
    /// index outside the vertices.
    pub fn create_mesh_from(&self, vertices: &[MeshVertex], indices: &[u32], label: Option<&str>) -> Result<Mesh, GraphicsException> {
        check_mesh(vertices.len(), indices)?;
        let (bounds_min, bounds_max) = bounds(vertices);
        let vertex_label = label.map(|l| format!("{l} vertices"));
        let index_label = label.map(|l| format!("{l} indices"));
        let vertex_buffer = self.create_vertex_buffer(vertices, vertex_label.as_deref())?;
        let index_buffer = self.create_index_buffer_u32(indices, index_label.as_deref())?;
        Ok(Mesh { vertex_buffer, index_buffer, vertex_count: vertices.len(), index_count: indices.len(), bounds_min, bounds_max })
    }
}

/// C#'s checks of a mesh's vertices and indices.
pub(crate) fn check_mesh(vertex_count: usize, indices: &[u32]) -> Result<(), GraphicsException> {
    if vertex_count == 0 {
        return Err(GraphicsException::new("A mesh needs at least one vertex; give it the vertices of its triangles."));
    }
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err(GraphicsException::new(format!("A mesh needs a non-empty triangle list (a multiple of 3 indices), not {} indices; give three indices per triangle.", indices.len())));
    }
    if let Some(index) = indices.iter().find(|&&index| index as usize >= vertex_count) {
        return Err(GraphicsException::new(format!("Index {index} is out of range for {vertex_count} vertices; every index must name one of the mesh's vertices.")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_mesh;

    #[test]
    fn a_mesh_needs_vertices_and_whole_triangles_in_range() {
        assert!(check_mesh(3, &[0, 1, 2]).is_ok());
        assert!(check_mesh(0, &[0, 1, 2]).unwrap_err().to_string().contains("at least one vertex"));
        assert!(check_mesh(3, &[0, 1]).unwrap_err().to_string().contains("multiple of 3"));
        assert!(check_mesh(3, &[]).is_err());
        assert!(check_mesh(3, &[0, 1, 3]).unwrap_err().to_string().contains("Index 3 is out of range for 3 vertices"));
    }
}
