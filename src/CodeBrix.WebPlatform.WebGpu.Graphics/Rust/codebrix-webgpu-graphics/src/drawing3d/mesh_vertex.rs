//! [`MeshVertex`]: one vertex of a [`Mesh`](crate::Mesh) - position, normal, texture coordinate
//! and colour, 48 bytes (C#: `MeshVertex`).

use glam::{Vec2, Vec3, Vec4};

use crate::common::Color4;
use crate::pipelines::{VertexLayout, VertexLayoutOptions, VertexType};

/// One vertex of a mesh (48 bytes, C#: `MeshVertex`): the position and the normal in model space
/// (the normal need not be unit length), the texture coordinate (0,0 = the texture's top-left
/// corner) and the vertex colour, which multiplies the material's base colour (white for none).
/// The lit shader reads it at locations 0 to 3.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshVertex {
    /// The position in model space (C#: `Position`).
    pub position: Vec3,
    /// The normal in model space (C#: `Normal`).
    pub normal: Vec3,
    /// The texture coordinate (C#: `TexCoord`).
    pub tex_coord: Vec2,
    /// The colour, linear RGBA (C#: `Color`).
    pub color: Vec4,
}

const _: () = assert!(std::mem::size_of::<MeshVertex>() == 48);

// SAFETY: #[repr(C)]; 12 + 12 + 8 bytes put the Vec4 at offset 32, a multiple of its alignment,
// and the size is 48 (asserted above): there is no padding, and every bit pattern of f32 fields
// is a valid value.
unsafe impl bytemuck::Zeroable for MeshVertex {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for MeshVertex {}

impl MeshVertex {
    /// A vertex with a colour (C#: `new MeshVertex(position, normal, texCoord, color)`).
    pub fn new(position: Vec3, normal: Vec3, tex_coord: Vec2, color: Color4) -> MeshVertex {
        MeshVertex { position, normal, tex_coord, color: color.to_vector4() }
    }

    /// A white vertex (C#: `new MeshVertex(position, normal, texCoord)`).
    pub fn white(position: Vec3, normal: Vec3, tex_coord: Vec2) -> MeshVertex {
        MeshVertex { position, normal, tex_coord, color: Vec4::ONE }
    }
}

impl VertexType for MeshVertex {
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
        VertexLayout::describe(options)
            .add(wgpu::VertexFormat::Float32x3)
            .add(wgpu::VertexFormat::Float32x3)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Float32x4)
            .build(48)
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec2, Vec3, Vec4};

    use super::MeshVertex;
    use crate::common::Color4;
    use crate::pipelines::{VertexLayoutOptions, VertexType};

    #[test]
    fn the_layout_is_the_csharp_one() {
        let layout = MeshVertex::vertex_layout(VertexLayoutOptions::default());
        let offsets: Vec<u64> = layout.elements().iter().map(|e| e.offset).collect();
        assert_eq!(offsets, vec![0, 12, 24, 32]);
        assert_eq!(layout.array_stride(), 48);
        let vertex = MeshVertex::new(Vec3::X, Vec3::Y, Vec2::ONE, Color4::RED);
        assert_eq!(vertex.color, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(bytemuck::bytes_of(&vertex).len(), 48);
        assert_eq!(MeshVertex::white(Vec3::X, Vec3::Y, Vec2::ONE).color, Vec4::ONE);
    }
}
