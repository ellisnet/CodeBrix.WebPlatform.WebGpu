//! Shadows (Not in C#): the settings of the sun's shadow ([`SunShadowSettings`]) and of the
//! spotlights' shadows ([`SpotShadowSettings`]) with the map sizes ([`ShadowMapSize`]), and the
//! CPU half of the model renderer's shadow passes: the light's view-projection matrices, the
//! volumes each pass culls against, and which spotlights get shadows. Tested on the host.

use glam::{Mat4, Vec3};

use crate::lighting::local_lights::LocalLight;
use crate::lighting::SpotLight;
use crate::visibility::CullVolume;

/// The width and height of a shadow map, in texels (Not in C#). Larger maps give sharper shadows
/// over a larger region, at 4 bytes a texel of GPU memory (1024: 4 MB, 2048: 16 MB, 4096: 64 MB;
/// the spotlights' map holds four such layers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadowMapSize {
    /// 1024 x 1024 (the spotlights' default).
    Size1024,
    /// 2048 x 2048 (the sun's default).
    #[default]
    Size2048,
    /// 4096 x 4096.
    Size4096,
}

impl ShadowMapSize {
    /// The size in texels.
    pub fn texels(self) -> u32 {
        match self {
            ShadowMapSize::Size1024 => 1024,
            ShadowMapSize::Size2048 => 2048,
            ShadowMapSize::Size4096 => 4096,
        }
    }
}

/// The shadow of the sun - the KEY light of [`SceneLighting`](crate::SceneLighting)
/// (`light_direction`) - in the model renderer (Not in C#). Off by default. On, the renderer draws
/// the queued instances that cast shadows into one shadow map seen from the sun before it draws
/// them, and the parts that receive shadows are darkened where the map says something is between
/// them and the sun. The map covers ONE REGION, a box in world space: outside it nothing is in
/// shadow from the sun (one region, one resolution - no cascades). Things outside the box still
/// cast into it when they are between it and the sun (up to twice the box's diagonal towards the
/// sun). Only the key light casts; the additional directional lights do not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SunShadowSettings {
    /// True to draw the sun's shadow; default false.
    pub enabled: bool,
    /// The shadow map's size; default 2048. A texel covers the region's width / size.
    pub map_size: ShadowMapSize,
    /// The lowest corner of the region the shadow covers, world space; default (-10, -10, -10).
    pub region_min: Vec3,
    /// The highest corner of the region; default (10, 10, 10). Keep the region as small as the
    /// scene allows: the map's texels are spread over it.
    pub region_max: Vec3,
    /// How many samples of the map are averaged per pixel - how soft the shadow's edge is: 1 (one
    /// filtered sample: a hard edge), 4, 9 (default), 16 or 25; other values round to the nearest
    /// square (a square grid of samples one texel apart; 0 counts as 1, above 25 as 25).
    pub softness: u32,
    /// How dark the shadow is, 0 (no shadow) to 1 (none of the sun's light; default). Ambient and
    /// image-based light are never shadowed, so even at 1 a shadow shows the ambient light.
    pub strength: f32,
}

impl Default for SunShadowSettings {
    fn default() -> SunShadowSettings {
        SunShadowSettings { enabled: false, map_size: ShadowMapSize::Size2048, region_min: Vec3::splat(-10.0), region_max: Vec3::splat(10.0), softness: 9, strength: 1.0 }
    }
}

impl SunShadowSettings {
    /// The sun's shadow switched ON over the region `min..max` (for example the box given to
    /// [`OrbitController::frame_box`](crate::OrbitController::frame_box), or a model's bounds
    /// placed in the world), with the other defaults.
    pub fn covering(min: Vec3, max: Vec3) -> SunShadowSettings {
        SunShadowSettings { enabled: true, region_min: min, region_max: max, ..SunShadowSettings::default() }
    }
}

/// The shadows of the spotlights that cast them ([`SpotLight::casts_shadows`]) in the model
/// renderer (Not in C#). At most
/// [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`](crate::SceneLighting::MAX_SHADOWED_SPOTLIGHTS) spotlights
/// cast shadows in a frame - the ones that matter most to the view among those lit; the others
/// still light, without shadows. Each has one layer of one shadow map; the map sees the spot's
/// whole cone out to its range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotShadowSettings {
    /// The size of each spotlight's shadow map; default 1024.
    pub map_size: ShadowMapSize,
    /// Samples averaged per pixel (as [`SunShadowSettings::softness`]); default 9.
    pub softness: u32,
    /// How dark the shadow is, 0 to 1 (as [`SunShadowSettings::strength`]); default 1.
    pub strength: f32,
}

impl Default for SpotShadowSettings {
    fn default() -> SpotShadowSettings {
        SpotShadowSettings { map_size: ShadowMapSize::Size1024, softness: 9, strength: 1.0 }
    }
}

/// The side of the square grid of samples for a softness: 1 to 5.
pub(crate) fn sample_grid(softness: u32) -> u32 {
    ((softness.max(1) as f32).sqrt().round() as u32).clamp(1, 5)
}

/// One shadow pass worked out on the CPU: the light's view-projection, the volume the pass culls
/// against, and how wide a texel of the map is (world units; for a spotlight per unit of distance
/// from the light).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShadowView {
    pub(crate) view_projection: Mat4,
    pub(crate) volume: CullVolume,
    pub(crate) texel: f32,
}

/// A view direction's up vector: +Y, or +Z when the direction is (nearly) vertical.
fn up_for(direction: Vec3) -> Vec3 {
    if direction.cross(Vec3::Y).length_squared() < 1e-6 {
        Vec3::Z
    } else {
        Vec3::Y
    }
}

/// The sun's shadow pass for a light travelling along `direction` over the region `min..max`
/// with a map of `texels` texels: an orthographic view along the light, fitted to the region's
/// eight corners across, and in depth from the region's far side back towards the sun by twice the
/// region's diagonal (so that a caster between the region and the sun still casts into it). None
/// for no direction or an empty or non-finite region.
pub(crate) fn sun_shadow_view(direction: Vec3, min: Vec3, max: Vec3, texels: u32) -> Option<ShadowView> {
    let direction = direction.normalize_or_zero();
    if direction == Vec3::ZERO || !direction.is_finite() || !min.is_finite() || !max.is_finite() || !min.cmple(max).all() {
        return None;
    }
    let center = (min + max) * 0.5;
    let diagonal = (max - min).length().max(1e-3);
    let view = glam::camera::rh::view::look_to_mat4(center - direction * diagonal, direction, up_for(direction));
    let mut low = Vec3::splat(f32::MAX);
    let mut high = Vec3::splat(f32::MIN);
    for corner in 0..8 {
        let point = Vec3::new(if corner & 1 == 0 { min.x } else { max.x }, if corner & 2 == 0 { min.y } else { max.y }, if corner & 4 == 0 { min.z } else { max.z });
        let at = view.transform_point3(point);
        low = low.min(at);
        high = high.max(at);
    }
    // The view looks down -Z: the distance along the light is -z.
    let (near, far) = (-high.z - 2.0 * diagonal, -low.z);
    let width = (high.x - low.x).max(1e-3);
    let height = (high.y - low.y).max(1e-3);
    let projection = glam::camera::rh::proj::directx::orthographic(low.x, low.x + width, low.y, low.y + height, near, far.max(near + 1e-3));
    let view_projection = projection * view;
    Some(ShadowView { view_projection, volume: CullVolume::from_view_projection(view_projection), texel: width.max(height) / texels as f32 })
}

/// The widest a spotlight's shadow camera looks: 170 degrees (a perspective view cannot see a
/// half space).
const MAX_SPOT_SHADOW_FOV: f32 = 170.0 * std::f32::consts::PI / 180.0;

/// A spotlight's shadow pass with a map of `texels` texels: a perspective view from the light
/// along its axis, wide enough for the outer cone (plus one degree each side, at most 170
/// degrees), out to its range (`far_fallback` - the camera's far plane - for an infinite range).
/// `texel` is the width of a texel at 1 unit from the light. None for a light without a direction.
pub(crate) fn spot_shadow_view(spot: &SpotLight, far_fallback: f32, texels: u32) -> Option<ShadowView> {
    let direction = spot.direction.normalize_or_zero();
    if direction == Vec3::ZERO || !direction.is_finite() || !spot.position.is_finite() {
        return None;
    }
    let field_of_view = (2.0 * spot.clamped_outer_angle() + 2.0f32.to_radians()).min(MAX_SPOT_SHADOW_FOV);
    let far = if spot.range.is_finite() && spot.range > 0.0 { spot.range } else { far_fallback.max(1.0) };
    let near = (far * 0.002).max(0.02).min(far * 0.5);
    let view = glam::camera::rh::view::look_to_mat4(spot.position, direction, up_for(direction));
    let view_projection = glam::camera::rh::proj::directx::perspective(field_of_view, 1.0, near, far) * view;
    let texel = 2.0 * (field_of_view * 0.5).tan() / texels as f32;
    Some(ShadowView { view_projection, volume: CullVolume::from_view_projection(view_projection), texel })
}

/// Which of the chosen local lights (`chosen`, indices into `lights`, most important first) cast
/// shadows: the first `limit` spotlights with [`SpotLight::casts_shadows`], as positions in
/// `chosen`.
pub(crate) fn select_shadowed_spots(lights: &[LocalLight], chosen: &[usize], limit: usize) -> Vec<usize> {
    chosen.iter().enumerate().filter(|(_, &index)| lights[index].as_spot().is_some_and(|spot| spot.casts_shadows)).map(|(position, _)| position).take(limit).collect()
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, Vec4};

    use super::{sample_grid, select_shadowed_spots, spot_shadow_view, sun_shadow_view, ShadowMapSize, SpotShadowSettings, SunShadowSettings};
    use crate::lighting::{LocalLight, PointLight, SpotLight};
    use crate::Color4;

    fn clip(view_projection: glam::Mat4, point: Vec3) -> Vec3 {
        let clip = view_projection * point.extend(1.0);
        clip.truncate() / clip.w
    }

    #[test]
    fn the_defaults() {
        let sun = SunShadowSettings::default();
        assert!(!sun.enabled);
        assert_eq!(sun.map_size.texels(), 2048);
        assert_eq!((sun.softness, sun.strength), (9, 1.0));
        assert_eq!(SpotShadowSettings::default().map_size, ShadowMapSize::Size1024);
        assert_eq!(ShadowMapSize::Size4096.texels(), 4096);
        let covering = SunShadowSettings::covering(Vec3::ZERO, Vec3::ONE);
        assert!(covering.enabled && covering.region_max == Vec3::ONE);
        assert_eq!([0, 1, 2, 4, 9, 12, 16, 25, 99].map(sample_grid), [1, 1, 1, 2, 3, 3, 4, 5, 5]);
    }

    #[test]
    fn the_sun_s_view_holds_the_region_and_reaches_back_towards_the_sun() {
        let (min, max) = (Vec3::new(-10.0, 0.0, -5.0), Vec3::new(10.0, 4.0, 5.0));
        let direction = Vec3::new(-0.4, -1.0, -0.5);
        let view = sun_shadow_view(direction, min, max, 2048).unwrap();
        for corner in 0..8 {
            let point = Vec3::new(if corner & 1 == 0 { min.x } else { max.x }, if corner & 2 == 0 { min.y } else { max.y }, if corner & 4 == 0 { min.z } else { max.z });
            let ndc = clip(view.view_projection, point);
            assert!(ndc.x.abs() <= 1.0001 && ndc.y.abs() <= 1.0001 && ndc.z >= -1e-4 && ndc.z <= 1.0001, "corner {corner}: {ndc}");
            assert!(view.volume.contains_point(point));
        }
        // Nearer the sun means a smaller depth; a point one diagonal up-light from the centre is
        // still inside (it casts into the region), one far beyond it is not.
        let diagonal = (max - min).length();
        let towards_sun = -direction.normalize();
        let center = (min + max) * 0.5;
        let near = clip(view.view_projection, center + towards_sun * diagonal);
        assert!(near.z >= 0.0 && near.z < clip(view.view_projection, center).z);
        assert!(view.volume.contains_point(center + towards_sun * diagonal * 1.5));
        assert!(!view.volume.contains_point(center + towards_sun * diagonal * 10.0));
        // Beyond the region away from the sun, or beside it: outside.
        assert!(!view.volume.contains_point(center - towards_sun * diagonal * 2.0));
        assert!(!view.volume.contains_point(center + Vec3::new(100.0, 0.0, 0.0)));
        // A texel's width: the footprint's larger side over the map's size.
        assert!(view.texel > 15.0 / 2048.0 && view.texel < diagonal / 2048.0 + 1e-6);
        // Straight down works too (the up vector changes); no direction or an empty region: None.
        assert!(sun_shadow_view(Vec3::NEG_Y, min, max, 1024).is_some());
        assert!(sun_shadow_view(Vec3::ZERO, min, max, 1024).is_none());
        assert!(sun_shadow_view(direction, max, min, 1024).is_none());
    }

    #[test]
    fn a_spot_s_view_sees_its_cone_out_to_its_range() {
        let spot = SpotLight::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, Color4::WHITE, 10.0, 8.0, 0.3, 0.5).with_shadows(true);
        let view = spot_shadow_view(&spot, 1000.0, 1024).unwrap();
        // On the axis, near the range: inside, depth near 1; beyond the range or behind: outside.
        let ndc = clip(view.view_projection, Vec3::new(0.0, -2.9, 0.0));
        assert!(ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4 && ndc.z > 0.9 && ndc.z <= 1.0);
        assert!(!view.volume.contains_point(Vec3::new(0.0, -3.5, 0.0)));
        assert!(!view.volume.contains_point(Vec3::new(0.0, 6.0, 0.0)));
        // At the edge of the outer cone (0.5 rad off the axis, 4 m down): inside; well outside the cone: outside.
        let edge = Vec3::new(4.0 * 0.5f32.tan(), 1.0, 0.0);
        assert!(view.volume.contains_point(edge));
        assert!(!view.volume.contains_point(Vec3::new(4.0, 1.0, 0.0)));
        // A texel at 1 m: 2 tan(fov / 2) / size.
        let fov = 2.0 * 0.5 + 2.0f32.to_radians();
        assert!((view.texel - 2.0 * (fov * 0.5).tan() / 1024.0).abs() < 1e-7);
        // An infinite range reaches out to the fallback; a very wide cone is capped at 170 degrees.
        let endless = SpotLight { range: f32::INFINITY, ..spot };
        assert!(spot_shadow_view(&endless, 50.0, 1024).unwrap().volume.contains_point(Vec3::new(0.0, -40.0, 0.0)));
        let wide = SpotLight { outer_angle: 1.55, ..spot };
        assert!(spot_shadow_view(&wide, 50.0, 1024).unwrap().volume.planes().iter().all(|p: &Vec4| p.is_finite()));
        assert!(spot_shadow_view(&SpotLight { direction: Vec3::ZERO, ..spot }, 50.0, 1024).is_none());
    }

    #[test]
    fn the_four_most_important_shadow_casting_spots_get_shadows() {
        let spot = |casts: bool| LocalLight::from(SpotLight::new(Vec3::ZERO, Vec3::NEG_Y, Color4::WHITE, 1.0, 5.0, 0.3, 0.5).with_shadows(casts));
        let point = LocalLight::from(PointLight::new(Vec3::ZERO, Color4::WHITE, 1.0, 5.0));
        // List: point, casting spots at 1, 2, 4, 5, 6, a plain spot at 3.
        let lights = [point, spot(true), spot(true), spot(false), spot(true), spot(true), spot(true)];
        // Chosen most important first: 6, 0, 3, 5, 1, 4, 2.
        let chosen = [6, 0, 3, 5, 1, 4, 2];
        // Positions in `chosen` of the first four casters: 6 (0), 5 (3), 1 (4), 4 (5); 2 is the fifth.
        assert_eq!(select_shadowed_spots(&lights, &chosen, 4), vec![0, 3, 4, 5]);
        assert_eq!(select_shadowed_spots(&lights, &chosen[..2], 4), vec![0]);
    }
}
