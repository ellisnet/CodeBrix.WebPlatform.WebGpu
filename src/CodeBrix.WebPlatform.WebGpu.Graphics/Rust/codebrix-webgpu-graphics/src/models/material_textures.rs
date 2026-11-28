//! The CPU-only parts of animating a model material's textures in the [`ModelRenderer`]
//! (Not in C#): where a texture slot's transform rows lie in the material uniforms, and which
//! bind group a material uses while some of its slots sample textures of the application's
//! choosing. Tested on the host.
//!
//! [`ModelRenderer`]: crate::ModelRenderer

use glam::{Affine2, Vec4};

use crate::models::pbr_data::PbrMaterialUniforms;
use crate::models::ModelTextureKind;

/// The number of texture slots of a material (base colour, metallic-roughness, normal,
/// occlusion, emissive).
pub(crate) const SLOT_COUNT: usize = 5;

/// The material bind groups a material keeps for texture combinations of the application's
/// (besides its own): enough for the frames of a flipbook. The oldest goes first.
pub(crate) const KEPT_OVERRIDE_GROUPS: usize = 32;

/// The slot's number in the PBR shader's order ([`crate::ModelMaterial::slots`]): its bindings
/// are texture `number + 1` and sampler `number + 6`, its transform rows `uv[2 * number]` and
/// `uv[2 * number + 1]`.
pub(crate) fn slot_number(kind: ModelTextureKind) -> usize {
    match kind {
        ModelTextureKind::BaseColor => 0,
        ModelTextureKind::MetallicRoughness => 1,
        ModelTextureKind::Normal => 2,
        ModelTextureKind::Occlusion => 3,
        ModelTextureKind::Emissive => 4,
    }
}

/// A texture-coordinate transform in C#'s `Matrix3x2` order (M11, M12, M21, M22, M31, M32), the
/// form of [`crate::ModelTextureSlot::transform`]; the inverse of
/// [`crate::ModelTextureSlot::transform_affine`].
pub(crate) fn matrix3x2(transform: Affine2) -> [f32; 6] {
    let (x, y, t) = (transform.matrix2.x_axis, transform.matrix2.y_axis, transform.translation);
    [x.x, x.y, y.x, y.y, t.x, t.y]
}

/// The two uniform rows of a slot's transform, as [`PbrMaterialUniforms::new`] writes them: the
/// shader's `uv = (dot(row0.xyz, (u, v, 1)), dot(row1.xyz, (u, v, 1)))`.
pub(crate) fn uv_rows(transform: [f32; 6]) -> [Vec4; 2] {
    let t = transform;
    [Vec4::new(t[0], t[2], t[4], 0.0), Vec4::new(t[1], t[3], t[5], 0.0)]
}

/// The byte offset of slot `number`'s two transform rows in the material uniforms (32 bytes).
pub(crate) fn uv_rows_offset(number: usize) -> u64 {
    (std::mem::offset_of!(PbrMaterialUniforms, uv) + number * 2 * std::mem::size_of::<Vec4>()) as u64
}

/// The textures a material's slots sample instead of the model's own, and the bind groups kept
/// for such combinations: `V` is a texture view, `G` a bind group (generic so that the
/// bookkeeping is tested without a GPU).
pub(crate) struct TextureOverrides<V, G> {
    slots: [Option<V>; SLOT_COUNT],
    kept: Vec<([Option<V>; SLOT_COUNT], G)>,
}

impl<V: Clone + PartialEq, G: Clone> TextureOverrides<V, G> {
    /// No slot overridden, no group kept.
    pub(crate) fn new() -> TextureOverrides<V, G> {
        TextureOverrides { slots: std::array::from_fn(|_| None), kept: Vec::new() }
    }

    /// What slot `number` samples instead of the model's texture, or None for the model's own.
    #[cfg(test)]
    pub(crate) fn slot(&self, number: usize) -> Option<&V> {
        self.slots[number].as_ref()
    }

    /// Sets what slot `number` samples (None: the model's own texture again).
    pub(crate) fn set(&mut self, number: usize, view: Option<V>) {
        self.slots[number] = view;
    }

    /// The bind group for the slots as they are now: None when no slot is overridden (the
    /// material's own group then applies), else the kept group of this combination, or a new one
    /// from `make`, kept (the least recently used goes beyond [`KEPT_OVERRIDE_GROUPS`]).
    pub(crate) fn group(&mut self, make: impl FnOnce(&[Option<V>; SLOT_COUNT]) -> G) -> Option<G> {
        if self.slots.iter().all(Option::is_none) {
            return None;
        }
        if let Some(position) = self.kept.iter().position(|(slots, _)| *slots == self.slots) {
            let entry = self.kept.remove(position);
            let group = entry.1.clone();
            self.kept.push(entry);
            return Some(group);
        }
        let group = make(&self.slots);
        if self.kept.len() >= KEPT_OVERRIDE_GROUPS {
            self.kept.remove(0);
        }
        self.kept.push((self.slots.clone(), group.clone()));
        Some(group)
    }

    /// The number of groups kept.
    #[cfg(test)]
    pub(crate) fn kept_count(&self) -> usize {
        self.kept.len()
    }
}

#[cfg(test)]
mod tests {
    use glam::{Affine2, Vec2, Vec4};

    use super::{matrix3x2, slot_number, uv_rows, uv_rows_offset, TextureOverrides, KEPT_OVERRIDE_GROUPS};
    use crate::models::pbr_data::PbrMaterialUniforms;
    use crate::models::{ModelMaterial, ModelTextureKind, ModelTextureSlot};

    const KINDS: [ModelTextureKind; 5] = [ModelTextureKind::BaseColor, ModelTextureKind::MetallicRoughness, ModelTextureKind::Normal, ModelTextureKind::Occlusion, ModelTextureKind::Emissive];

    #[test]
    fn an_affine_transform_round_trips_through_the_slots_matrix3x2() {
        let turn = Affine2::from_scale_angle_translation(Vec2::new(2.0, 0.5), 0.3, Vec2::new(0.25, -1.5));
        let slot = ModelTextureSlot { transform: matrix3x2(turn), ..ModelTextureSlot::EMPTY };
        assert_eq!(slot.transform_affine(), turn);
        assert_eq!(matrix3x2(Affine2::IDENTITY), ModelTextureSlot::IDENTITY_TRANSFORM);
        let point = Vec2::new(0.7, 0.2);
        assert_eq!(slot.transform_affine().transform_point2(point), turn.transform_point2(point));
    }

    #[test]
    fn each_slots_rows_are_where_the_material_uniforms_have_them() {
        // Every slot with a transform of its own: the rows written in place must be the bytes
        // the whole uniforms hold at that offset.
        let mut material = ModelMaterial::new(0, None);
        for (number, kind) in KINDS.iter().enumerate() {
            let offset = number as f32;
            material.slot_mut(*kind).transform = matrix3x2(Affine2::from_scale_angle_translation(Vec2::splat(1.0 + offset), offset * 0.1, Vec2::new(offset, -offset)));
        }
        let uniforms = PbrMaterialUniforms::new(&material);
        let bytes = bytemuck::bytes_of(&uniforms);
        for (number, kind) in KINDS.iter().enumerate() {
            assert_eq!(slot_number(*kind), number);
            assert_eq!(material.slots()[number], material.slot(*kind));
            let rows = uv_rows(material.slot(*kind).transform);
            let at = uv_rows_offset(number) as usize;
            assert_eq!(&bytes[at..at + 32], bytemuck::cast_slice::<Vec4, u8>(&rows), "slot {number}");
            assert_eq!([uniforms.uv[number * 2], uniforms.uv[number * 2 + 1]], rows);
        }
        // The rows sit after the six Vec4 of factors and flags, the last ones end the 256 bytes.
        assert_eq!(uv_rows_offset(0), 96);
        assert_eq!(uv_rows_offset(4) + 32, 256);
    }

    #[test]
    fn the_rows_give_the_shaders_texture_coordinates() {
        // A river: scrolled by 0.1 along u, turned a quarter.
        let flow = Affine2::from_translation(Vec2::new(0.1, 0.0)) * Affine2::from_angle(std::f32::consts::FRAC_PI_2);
        let rows = uv_rows(matrix3x2(flow));
        let source = glam::Vec3::new(0.25, 0.5, 1.0);
        let shader = Vec2::new(rows[0].truncate().dot(source), rows[1].truncate().dot(source));
        let expected = flow.transform_point2(Vec2::new(0.25, 0.5));
        assert!((shader - expected).length() < 1e-6, "{shader} {expected}");
    }

    #[test]
    fn without_an_override_the_material_uses_its_own_group() {
        let mut overrides: TextureOverrides<u32, String> = TextureOverrides::new();
        assert_eq!(overrides.group(|_| unreachable!("no group is made")), None);
        overrides.set(0, Some(7));
        overrides.set(0, None);
        assert_eq!(overrides.group(|_| unreachable!("no group is made")), None);
        assert_eq!(overrides.kept_count(), 0);
    }

    #[test]
    fn a_group_is_made_once_per_combination_and_kept() {
        let mut overrides: TextureOverrides<u32, String> = TextureOverrides::new();
        let made = std::cell::Cell::new(0);
        let make = |slots: &[Option<u32>; 5]| {
            made.set(made.get() + 1);
            format!("{slots:?}")
        };
        // A flipbook of three frames in the base colour slot, played twice.
        for _ in 0..2 {
            for frame in [10, 11, 12] {
                overrides.set(0, Some(frame));
                assert_eq!(overrides.group(make).as_deref(), Some(format!("{:?}", [Some(frame), None, None, None, None]).as_str()));
            }
        }
        assert_eq!(made.get(), 3);
        // A second slot makes new combinations; the emissive slot alone is one of its own.
        overrides.set(4, Some(20));
        assert_eq!(overrides.slot(4), Some(&20));
        overrides.group(make);
        assert_eq!(made.get(), 4);
        overrides.set(0, None);
        overrides.group(make);
        assert_eq!((made.get(), overrides.kept_count()), (5, 5));
        // Restoring every slot goes back to the material's own group; the kept ones stay.
        overrides.set(4, None);
        assert_eq!(overrides.group(make), None);
        assert_eq!((made.get(), overrides.kept_count()), (5, 5));
    }

    #[test]
    fn the_least_recently_used_group_goes_first() {
        let mut overrides: TextureOverrides<u32, u32> = TextureOverrides::new();
        let made = std::cell::Cell::new(0u32);
        let make = |slots: &[Option<u32>; 5]| {
            made.set(made.get() + 1);
            slots[0].unwrap()
        };
        for frame in 0..KEPT_OVERRIDE_GROUPS as u32 {
            overrides.set(0, Some(frame));
            overrides.group(make);
        }
        // Frame 0 used again, then one more frame: frame 1 (the oldest unused) goes, frame 0 stays.
        overrides.set(0, Some(0));
        overrides.group(make);
        overrides.set(0, Some(1000));
        overrides.group(make);
        assert_eq!((overrides.kept_count(), made.get()), (KEPT_OVERRIDE_GROUPS, KEPT_OVERRIDE_GROUPS as u32 + 1));
        overrides.set(0, Some(0));
        overrides.group(make);
        assert_eq!(made.get(), KEPT_OVERRIDE_GROUPS as u32 + 1, "frame 0 was kept");
        overrides.set(0, Some(1));
        overrides.group(make);
        assert_eq!(made.get(), KEPT_OVERRIDE_GROUPS as u32 + 2, "frame 1 was made again");
    }
}
