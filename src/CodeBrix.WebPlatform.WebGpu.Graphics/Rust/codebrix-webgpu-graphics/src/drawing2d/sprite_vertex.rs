//! [`SpriteVertex`]: one corner of a sprite-batch quad, as the built-in sprite shader reads it
//! (C#: the internal `SpriteVertex`, 24 bytes).

use crate::pipelines::{VertexLayout, VertexLayoutOptions, VertexType};

/// One vertex of the sprite batch (24 bytes): the position in batch coordinates (pixels, y
/// down, before the projection), the texture coordinate, the colour packed RGBA (red in the
/// lowest byte) and 1 to multiply by the texture or 0 for a flat colour.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SpriteVertex {
    pub(crate) position: [f32; 2],
    pub(crate) tex_coord: [f32; 2],
    pub(crate) color: u32,
    pub(crate) textured: f32,
}

// SAFETY: #[repr(C)], six 4-byte fields and no padding: every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for SpriteVertex {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for SpriteVertex {}

impl VertexType for SpriteVertex {
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
        VertexLayout::describe(options)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Unorm8x4)
            .add(wgpu::VertexFormat::Float32)
            .build(std::mem::size_of::<SpriteVertex>() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::SpriteVertex;
    use crate::pipelines::{VertexLayoutOptions, VertexType};

    #[test]
    fn the_vertex_is_24_bytes_with_four_locations() {
        let layout = SpriteVertex::vertex_layout(VertexLayoutOptions::default());
        assert_eq!(std::mem::size_of::<SpriteVertex>(), 24);
        assert_eq!(layout.array_stride(), 24);
        let offsets: Vec<u64> = layout.elements().iter().map(|e| e.offset).collect();
        assert_eq!(offsets, vec![0, 8, 16, 20]);
        assert_eq!(layout.elements()[2].format, wgpu::VertexFormat::Unorm8x4);
    }
}
