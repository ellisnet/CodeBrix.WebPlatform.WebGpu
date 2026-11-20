//! [`ModelVertex`] and [`ModelSkinVertex`]: one vertex of a model's shared vertex buffer (64
//! bytes) and the skinning data of one vertex (24 bytes) (C#: the types of the same names). Their
//! bytes are the model-data format's: a blob's vertex range goes to the GPU unchanged.

use glam::{Vec2, Vec3, Vec4};

use crate::pipelines::{VertexLayout, VertexLayoutOptions, VertexType};

/// One vertex of a [`Model`](crate::Model)'s shared vertex buffer (64 bytes, C#: `ModelVertex`):
/// position, normal, tangent (xyz + handedness in w), two texture-coordinate sets and a
/// 16-bit-per-channel colour. A primitive's indices are relative to its
/// [`ModelPrimitive::base_vertex`](crate::ModelPrimitive::base_vertex).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModelVertex {
    /// The position in mesh space (offset 0, Float32x3; C#: `Position`).
    pub position: Vec3,
    /// The normal (offset 12, Float32x3; unit length; C#: `Normal`).
    pub normal: Vec3,
    /// The tangent: xyz along +U, w = +1 or -1 for the bitangent's sign (offset 24, Float32x4);
    /// zero when the primitive has none (C#: `Tangent`). An array, not a `Vec4`: glam's `Vec4` is
    /// 16-byte aligned and would move the field to offset 32.
    pub tangent: [f32; 4],
    /// TEXCOORD_0 (offset 40, Float32x2; 0,0 = the texture's top-left corner; C#: `TexCoord0`).
    pub tex_coord0: Vec2,
    /// TEXCOORD_1 (offset 48, Float32x2), or zero (C#: `TexCoord1`).
    pub tex_coord1: Vec2,
    /// COLOR_0 as red, green, blue, alpha (offset 56, read as Unorm16x4; 65535 = 1; C#:
    /// `ColorR`, `ColorG`, `ColorB`, `ColorA`).
    pub color: [u16; 4],
}

const _: () = assert!(std::mem::size_of::<ModelVertex>() == 64);

// SAFETY: #[repr(C)]; every field is 4- or 2-byte aligned (12 + 12 + 16 + 8 + 8 + 8 bytes): 64 in all (asserted above) with no padding; every bit
// pattern of the f32 and u16 fields is a valid value.
unsafe impl bytemuck::Zeroable for ModelVertex {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for ModelVertex {}

impl ModelVertex {
    /// The size in bytes (C#: `Size`).
    pub const SIZE: usize = 64;

    /// The colour as floats 0..1 (C#: the `Color` getter).
    pub fn color_vector(&self) -> Vec4 {
        Vec4::new(self.color[0] as f32, self.color[1] as f32, self.color[2] as f32, self.color[3] as f32) / 65535.0
    }

    /// Sets the colour from floats, clamped to 0..1 and rounded (C#: the `Color` setter).
    pub fn set_color_vector(&mut self, value: Vec4) {
        let clamped = value.clamp(Vec4::ZERO, Vec4::ONE) * 65535.0;
        self.color = [(clamped.x + 0.5) as u16, (clamped.y + 0.5) as u16, (clamped.z + 0.5) as u16, (clamped.w + 0.5) as u16];
    }
}

impl VertexType for ModelVertex {
    fn vertex_layout(options: VertexLayoutOptions) -> VertexLayout {
        VertexLayout::describe(options)
            .add(wgpu::VertexFormat::Float32x3)
            .add(wgpu::VertexFormat::Float32x3)
            .add(wgpu::VertexFormat::Float32x4)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Float32x2)
            .add(wgpu::VertexFormat::Unorm16x4)
            .build(64)
    }
}

/// The skinning data of one vertex of a skinned primitive (24 bytes, C#: `ModelSkinVertex`), in
/// the model's separate skin buffer: four joint indices (into the skin's joint list) and their
/// weights (normalised to sum to 1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModelSkinVertex {
    /// The four joints (offset 0, read as Uint16x4; C#: `Joint0` to `Joint3`).
    pub joints: [u16; 4],
    /// The four weights (offset 8, Float32x4; C#: `Weights`; an array, as glam's `Vec4` is
    /// 16-byte aligned).
    pub weights: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<ModelSkinVertex>() == 24);

// SAFETY: #[repr(C)]; 8 bytes of u16 then four f32 at offset 8: 24 bytes with no padding (asserted above); every bit pattern is a valid value.
unsafe impl bytemuck::Zeroable for ModelSkinVertex {}
// SAFETY: as above; the type is Copy and 'static.
unsafe impl bytemuck::Pod for ModelSkinVertex {}

impl ModelSkinVertex {
    /// The size in bytes (C#: `Size`).
    pub const SIZE: usize = 24;
}

#[cfg(test)]
mod tests {
    use glam::Vec4;

    use super::{ModelSkinVertex, ModelVertex};
    use crate::pipelines::{VertexLayoutOptions, VertexType};

    #[test]
    fn the_layout_matches_the_csharp_offsets() {
        let layout = ModelVertex::vertex_layout(VertexLayoutOptions::default());
        assert_eq!(layout.array_stride(), 64);
        let offsets: Vec<u64> = layout.elements().iter().map(|e| e.offset).collect();
        assert_eq!(offsets, vec![0, 12, 24, 40, 48, 56]);
        assert_eq!(std::mem::align_of::<ModelVertex>(), 4);
        assert_eq!(ModelSkinVertex::SIZE, 24);
    }

    #[test]
    fn the_colour_round_trips_through_unorm16() {
        let mut vertex = ModelVertex::default();
        vertex.set_color_vector(Vec4::new(1.0, 0.5, 0.0, 2.0));
        assert_eq!(vertex.color, [65535, 32768, 0, 65535]);
        assert!((vertex.color_vector().y - 0.5).abs() < 1e-4);
    }
}
