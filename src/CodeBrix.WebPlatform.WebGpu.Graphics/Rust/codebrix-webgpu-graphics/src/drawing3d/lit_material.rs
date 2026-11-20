//! [`LitMaterial`] and [`MaterialAlphaMode`]: the surface of a mesh drawn by a
//! [`MeshRenderer`](crate::MeshRenderer) - Blinn-Phong settings, an optional base-colour texture,
//! an alpha mode and a texture-coordinate transform (C#: `LitMaterial`, `MaterialAlphaMode`) -
//! and [`MaterialOptions`], the C# default arguments of `CreateMaterial`.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use glam::{Affine2, Vec4};

use crate::common::Color4;
use crate::context::GraphicsException;
use crate::resources::{GpuBindGroup, GpuBuffer, GpuSampler, GpuTexture};

/// How a [`LitMaterial`] treats alpha (C#: `MaterialAlphaMode`: the glTF alpha modes, plus the
/// renderer's original rule).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaterialAlphaMode {
    /// Alpha-blended when the base colour's alpha is below 1, otherwise opaque with the alpha
    /// written as computed.
    #[default]
    Auto,
    /// Opaque: alpha is ignored and written as 1.
    Opaque,
    /// Alpha-tested: fragments below the alpha cutoff are discarded, the rest are opaque.
    Mask,
    /// Alpha-blended (straight alpha), drawn after the opaque draws with depth writes off.
    Blend,
}

/// The defaulted arguments of [`MeshRenderer::create_material`](crate::MeshRenderer::create_material)
/// (C#: `baseColorTexture = null`, `doubleSided = false`, `label = null`, `sampler = null`).
#[derive(Clone, Copy, Default)]
pub struct MaterialOptions<'a> {
    /// The base-colour texture (None: the context's white texture).
    pub base_color_texture: Option<&'a GpuTexture>,
    /// True to draw back faces too, with flipped normals; false culls them.
    pub double_sided: bool,
    /// The material's label.
    pub label: Option<&'a str>,
    /// The sampler of the texture (None: the renderer's linear repeating sampler).
    pub sampler: Option<&'a GpuSampler>,
}

/// The size of the material uniform block (C#: `UniformSize`).
pub(crate) const MATERIAL_UNIFORM_SIZE: u64 = 64;

/// The surface of a mesh drawn by a [`MeshRenderer`](crate::MeshRenderer) (C#: `LitMaterial`): a
/// base colour, an optional base-colour texture, Blinn-Phong specular settings, whether both
/// faces are drawn, an alpha mode and a texture-coordinate transform. Made by
/// `MeshRenderer::create_material`. A material is a shared HANDLE (as a C# object is): its clones
/// are the same material, and a change through any of them reaches the GPU at the next draw that
/// uses it; within one frame every draw sees the material's last values.
#[derive(Clone)]
pub struct LitMaterial {
    inner: Rc<Inner>,
}

struct Inner {
    renderer_id: u64,
    uniforms: GpuBuffer,
    bind_group: GpuBindGroup,
    base_color_texture: Option<wgpu::Texture>,
    sampler: Option<wgpu::Sampler>,
    double_sided: bool,
    label: Option<String>,
    state: RefCell<MaterialState>,
    dirty: Cell<bool>,
}

/// The values a material uploads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaterialState {
    pub(crate) base_color: Color4,
    pub(crate) shininess: f32,
    pub(crate) specular_strength: f32,
    pub(crate) unlit: bool,
    pub(crate) alpha_mode: MaterialAlphaMode,
    pub(crate) alpha_cutoff: f32,
    pub(crate) texture_transform: Affine2,
}

impl MaterialState {
    /// The C# defaults for a base colour.
    pub(crate) fn new(base_color: Color4) -> MaterialState {
        MaterialState { base_color, shininess: 32.0, specular_strength: 0.25, unlit: false, alpha_mode: MaterialAlphaMode::Auto, alpha_cutoff: 0.5, texture_transform: Affine2::IDENTITY }
    }

    /// True when the renderer alpha-blends the material.
    pub(crate) fn is_translucent(&self) -> bool {
        self.alpha_mode == MaterialAlphaMode::Blend || (self.alpha_mode == MaterialAlphaMode::Auto && self.base_color.a < 0.999)
    }

    /// The shader's alpha parameter: 0 keeps alpha, -1 forces it to 1, a positive value is the
    /// mask cutoff.
    pub(crate) fn alpha_parameter(&self) -> f32 {
        match self.alpha_mode {
            MaterialAlphaMode::Opaque => -1.0,
            MaterialAlphaMode::Mask => self.alpha_cutoff.max(1e-6),
            _ => 0.0,
        }
    }

    /// The uniform block: base colour, (shininess, specular, unlit, alpha), and the two rows of
    /// the texture transform (C#: `MaterialUniforms`).
    pub(crate) fn uniforms(&self) -> [Vec4; 4] {
        let m = self.texture_transform;
        [
            self.base_color.to_vector4(),
            Vec4::new(self.shininess, self.specular_strength, if self.unlit { 1.0 } else { 0.0 }, self.alpha_parameter()),
            Vec4::new(m.matrix2.x_axis.x, m.matrix2.y_axis.x, m.translation.x, 0.0),
            Vec4::new(m.matrix2.x_axis.y, m.matrix2.y_axis.y, m.translation.y, 0.0),
        ]
    }
}

impl LitMaterial {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(renderer_id: u64, uniforms: GpuBuffer, bind_group: GpuBindGroup, base_color: Color4, base_color_texture: Option<wgpu::Texture>, sampler: Option<wgpu::Sampler>, double_sided: bool, label: Option<&str>) -> LitMaterial {
        LitMaterial {
            inner: Rc::new(Inner { renderer_id, uniforms, bind_group, base_color_texture, sampler, double_sided, label: label.map(str::to_string), state: RefCell::new(MaterialState::new(base_color)), dirty: Cell::new(true) }),
        }
    }

    fn change(&self, f: impl FnOnce(&mut MaterialState)) {
        f(&mut self.inner.state.borrow_mut());
        self.inner.dirty.set(true);
    }

    fn state(&self) -> MaterialState {
        *self.inner.state.borrow()
    }

    /// The base-colour texture given at creation, if any (C#: `BaseColorTexture`).
    pub fn base_color_texture(&self) -> Option<&wgpu::Texture> {
        self.inner.base_color_texture.as_ref()
    }

    /// The sampler given at creation, if any (C#: `Sampler`; None means the renderer's linear
    /// repeating sampler).
    pub fn sampler(&self) -> Option<&wgpu::Sampler> {
        self.inner.sampler.as_ref()
    }

    /// True when back faces are drawn too (C#: `DoubleSided`).
    pub fn double_sided(&self) -> bool {
        self.inner.double_sided
    }

    /// The label given at creation.
    pub fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }

    /// The base colour: it multiplies the texture and the vertex colour; alpha below 1 blends
    /// (C#: `BaseColor`).
    pub fn base_color(&self) -> Color4 {
        self.state().base_color
    }

    /// Sets the base colour (C#: `BaseColor = value`).
    pub fn set_base_color(&self, value: Color4) {
        self.change(|s| s.base_color = value);
    }

    /// The specular exponent (C#: `Shininess`, default 32).
    pub fn shininess(&self) -> f32 {
        self.state().shininess
    }

    /// Sets the specular exponent (C#: `Shininess = value`).
    pub fn set_shininess(&self, value: f32) {
        self.change(|s| s.shininess = value);
    }

    /// The specular strength, 0 for matte (C#: `SpecularStrength`, default 0.25).
    pub fn specular_strength(&self) -> f32 {
        self.state().specular_strength
    }

    /// Sets the specular strength (C#: `SpecularStrength = value`).
    pub fn set_specular_strength(&self, value: f32) {
        self.change(|s| s.specular_strength = value);
    }

    /// True when lighting is ignored and the colour shown as it is (C#: `Unlit`, default false).
    pub fn unlit(&self) -> bool {
        self.state().unlit
    }

    /// Sets whether lighting is ignored (C#: `Unlit = value`).
    pub fn set_unlit(&self, value: bool) {
        self.change(|s| s.unlit = value);
    }

    /// How alpha is treated (C#: `AlphaMode`, default Auto).
    pub fn alpha_mode(&self) -> MaterialAlphaMode {
        self.state().alpha_mode
    }

    /// Sets how alpha is treated (C#: `AlphaMode = value`).
    pub fn set_alpha_mode(&self, value: MaterialAlphaMode) {
        self.change(|s| s.alpha_mode = value);
    }

    /// The alpha below which `Mask` discards a fragment (C#: `AlphaCutoff`, default 0.5).
    pub fn alpha_cutoff(&self) -> f32 {
        self.state().alpha_cutoff
    }

    /// Sets the alpha cutoff (C#: `AlphaCutoff = value`).
    pub fn set_alpha_cutoff(&self, value: f32) {
        self.change(|s| s.alpha_cutoff = value);
    }

    /// The transform of the texture coordinate before the base-colour texture is sampled
    /// (C#: `TextureTransform`, a `Matrix3x2`; glam's `Affine2` here; default identity).
    pub fn texture_transform(&self) -> Affine2 {
        self.state().texture_transform
    }

    /// Sets the texture-coordinate transform (C#: `TextureTransform = value`), for example a glTF
    /// KHR_texture_transform.
    pub fn set_texture_transform(&self, value: Affine2) {
        self.change(|s| s.texture_transform = value);
    }

    /// True when the renderer alpha-blends the material: `Blend`, or `Auto` with a base-colour
    /// alpha below 1 (C#: `IsTranslucent`).
    pub fn is_translucent(&self) -> bool {
        self.state().is_translucent()
    }

    /// The renderer that made it.
    pub(crate) fn renderer_id(&self) -> u64 {
        self.inner.renderer_id
    }

    /// The material's bind group (group 1 of the lit shader).
    pub(crate) fn bind_group(&self) -> &GpuBindGroup {
        &self.inner.bind_group
    }

    /// Uploads the values when they changed since the last upload (C#: `Upload`).
    pub(crate) fn upload(&self) -> Result<(), GraphicsException> {
        if !self.inner.dirty.replace(false) {
            return Ok(());
        }
        self.inner.uniforms.write(&self.state().uniforms(), 0)
    }
}

impl PartialEq for LitMaterial {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl fmt::Debug for LitMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LitMaterial({})", self.label().unwrap_or("unlabelled"))
    }
}

#[cfg(test)]
mod tests {
    use glam::{Affine2, Vec2, Vec4};

    use super::{MaterialAlphaMode, MaterialState};
    use crate::common::Color4;

    #[test]
    fn the_defaults_are_the_csharp_ones() {
        let state = MaterialState::new(Color4::RED);
        assert_eq!((state.shininess, state.specular_strength, state.alpha_cutoff), (32.0, 0.25, 0.5));
        assert_eq!(state.alpha_mode, MaterialAlphaMode::Auto);
        assert!(!state.is_translucent());
    }

    #[test]
    fn translucency_follows_the_alpha_mode_and_the_base_alpha() {
        let mut state = MaterialState::new(Color4::RED.with_alpha(0.5));
        assert!(state.is_translucent());
        state.alpha_mode = MaterialAlphaMode::Opaque;
        assert!(!state.is_translucent());
        assert_eq!(state.alpha_parameter(), -1.0);
        state.alpha_mode = MaterialAlphaMode::Mask;
        state.alpha_cutoff = 0.0;
        assert_eq!(state.alpha_parameter(), 1e-6);
        state.alpha_mode = MaterialAlphaMode::Blend;
        state.base_color = Color4::WHITE;
        assert!(state.is_translucent());
        assert_eq!(state.alpha_parameter(), 0.0);
    }

    #[test]
    fn the_uniforms_carry_the_texture_transform_rows() {
        let mut state = MaterialState::new(Color4::WHITE);
        state.unlit = true;
        state.texture_transform = Affine2::from_scale_angle_translation(Vec2::new(2.0, 3.0), 0.0, Vec2::new(0.5, 0.25));
        let [color, params, row0, row1] = state.uniforms();
        assert_eq!(color, Vec4::ONE);
        assert_eq!(params, Vec4::new(32.0, 0.25, 1.0, 0.0));
        assert_eq!(row0, Vec4::new(2.0, 0.0, 0.5, 0.0));
        assert_eq!(row1, Vec4::new(0.0, 3.0, 0.25, 0.0));
        // The WGSL computes (dot(row0, uv1), dot(row1, uv1)): Affine2's transform_point2.
        let uv = Vec2::new(0.25, 0.5);
        let shader = Vec2::new(row0.truncate().dot(uv.extend(1.0)), row1.truncate().dot(uv.extend(1.0)));
        assert_eq!(shader, state.texture_transform.transform_point2(uv));
    }
}
