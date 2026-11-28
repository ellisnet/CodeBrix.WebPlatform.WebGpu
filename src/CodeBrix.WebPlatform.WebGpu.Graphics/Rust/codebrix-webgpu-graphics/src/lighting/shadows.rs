//! Shadows (Not in C#): the settings of the sun's shadow ([`SunShadowSettings`]) and of the
//! spotlights' shadows ([`SpotShadowSettings`]) with the map sizes ([`ShadowMapSize`]), and the
//! CPU half of the model renderer's shadow passes: the light's view-projection matrices (the
//! sun's cascades among them), the volumes each pass culls against, which spotlights get shadows,
//! and the twin of the shader's choice of a cascade. Tested on the host.

use glam::{Mat4, Vec3};

use crate::drawing3d::{Camera, ProjectionKind, SceneLighting};
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
/// shadow from the sun. Things outside the box still cast into it when they are between it and the
/// sun (up to twice the box's diagonal towards the sun). Only the key light casts; the additional
/// directional lights do not.
///
/// CASCADES ([`SunShadowSettings::cascades`], default 1: one map over the whole region, one
/// resolution): with 2 to [`SunShadowSettings::MAX_CASCADES`] the camera's view is cut in depth
/// into that many slices out to [`SunShadowSettings::cascade_reach`], and each slice has a map of
/// its own (`map_size` each), fitted around that slice inside the region - small and sharp near the
/// camera, larger further away - so a large region keeps fine shadows close by and still has them
/// far off. The region still bounds what casts and receives; beyond the reach nothing is in the
/// sun's shadow. The maps move with the camera in whole texels and keep their size as it turns, so
/// the shadows' edges do not crawl. Each cascade is one more shadow pass (its own culling and draw
/// calls) and one more map of GPU memory.
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
    /// How many maps the shadow is drawn in (Not in C#): 1 (default) one map over the whole region,
    /// exactly as without cascades (the three settings below are then not used); 2 to
    /// [`SunShadowSettings::MAX_CASCADES`] (4) one map per slice of the camera's view in depth.
    /// 0 counts as 1, above 4 as 4.
    pub cascades: u32,
    /// How far from the camera the cascades reach, in world units of depth along the camera's view
    /// direction; beyond it nothing is in the sun's shadow. Default `f32::INFINITY`: as far as the
    /// region (the distance from the camera to the region's furthest corner, rounded up in steps of
    /// about a fifth so that the cascades do not change size while the camera moves a little);
    /// a value that is not a positive number counts the same. Never beyond the region or the
    /// camera's far plane. The smaller the reach, the sharper every cascade.
    pub cascade_reach: f32,
    /// Where the slices are cut between the camera and the reach: 0 in equal steps, 1 in equal
    /// ratios (each slice the same number of times deeper than the one before: the finest near the
    /// camera), between a mix of the two; default 0.7.
    pub cascade_split_weight: f32,
    /// The share of each slice's depth, at its far end, over which its shadow blends into the next
    /// one's (the last slice's fades out towards the reach), 0 (a hard switch) to 0.5; default 0.1.
    /// A point in a band reads two maps.
    pub cascade_blend: f32,
}

impl Default for SunShadowSettings {
    fn default() -> SunShadowSettings {
        SunShadowSettings {
            enabled: false,
            map_size: ShadowMapSize::Size2048,
            region_min: Vec3::splat(-10.0),
            region_max: Vec3::splat(10.0),
            softness: 9,
            strength: 1.0,
            cascades: 1,
            cascade_reach: f32::INFINITY,
            cascade_split_weight: 0.7,
            cascade_blend: 0.1,
        }
    }
}

impl SunShadowSettings {
    /// The most cascades the sun's shadow has (Not in C#).
    pub const MAX_CASCADES: u32 = 4;

    /// The sun's shadow switched ON over the region `min..max` (for example the box given to
    /// [`OrbitController::frame_box`](crate::OrbitController::frame_box), or a model's bounds
    /// placed in the world), with the other defaults.
    pub fn covering(min: Vec3, max: Vec3) -> SunShadowSettings {
        SunShadowSettings { enabled: true, region_min: min, region_max: max, ..SunShadowSettings::default() }
    }

    /// These settings with `cascades` cascades reaching `reach` from the camera
    /// (`f32::INFINITY`: as far as the region) (Not in C#; see [`SunShadowSettings::cascades`]
    /// and [`SunShadowSettings::cascade_reach`]).
    pub fn with_cascades(self, cascades: u32, reach: f32) -> SunShadowSettings {
        SunShadowSettings { cascades, cascade_reach: reach, ..self }
    }

    /// The number of cascades drawn: `cascades` limited to 1..=4.
    pub(crate) fn cascade_count(&self) -> u32 {
        self.cascades.clamp(1, SunShadowSettings::MAX_CASCADES)
    }
}

// Every shadow pass of a pair has a bit in the draw plan's 8-bit shadow mask.
const _: () = assert!(SunShadowSettings::MAX_CASCADES as usize + SceneLighting::MAX_SHADOWED_SPOTLIGHTS <= 8);

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
    let light = SunLightSpace::new(direction, min, max)?;
    let (low, high) = (light.low, light.high);
    let (near, far) = light.depth_range();
    let width = (high.x - low.x).max(1e-3);
    let height = (high.y - low.y).max(1e-3);
    let projection = glam::camera::rh::proj::directx::orthographic(low.x, low.x + width, low.y, low.y + height, near, far.max(near + 1e-3));
    let view_projection = projection * light.view;
    Some(ShadowView { view_projection, volume: CullVolume::from_view_projection(view_projection), texel: width.max(height) / texels as f32 })
}

/// The sun's light space for a region: the view along the light (from the region's centre back
/// towards the sun by its diagonal - fixed while the region and the light are), and the region's
/// eight corners' bounds in it.
struct SunLightSpace {
    view: Mat4,
    low: Vec3,
    high: Vec3,
    diagonal: f32,
}

impl SunLightSpace {
    /// None for no direction or an empty or non-finite region.
    fn new(direction: Vec3, min: Vec3, max: Vec3) -> Option<SunLightSpace> {
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
            let at = view.transform_point3(box_corner(min, max, corner));
            low = low.min(at);
            high = high.max(at);
        }
        Some(SunLightSpace { view, low, high, diagonal })
    }

    /// The near and far planes: from the region's far side back towards the sun by twice its
    /// diagonal. The view looks down -Z: the distance along the light is -z.
    fn depth_range(&self) -> (f32, f32) {
        (-self.high.z - 2.0 * self.diagonal, -self.low.z)
    }
}

/// Corner `corner` (0..8: bit 0 x, bit 1 y, bit 2 z) of the box `min..max`.
fn box_corner(min: Vec3, max: Vec3, corner: usize) -> Vec3 {
    Vec3::new(if corner & 1 == 0 { min.x } else { max.x }, if corner & 2 == 0 { min.y } else { max.y }, if corner & 4 == 0 { min.z } else { max.z })
}

/// The texels a cascade's map keeps free at each side around its slice: the snapping to whole
/// texels (1), the widest filter grid (2) and the normal offset (1.5), rounded up.
const CASCADE_MARGIN_TEXELS: f32 = 5.0;
/// The steps of the default reach per doubling of the distance to the region's furthest corner
/// (four: each step about a fifth further than the one before).
const REACH_STEPS_PER_DOUBLING: f32 = 4.0;

/// The sun's shadow passes of a begin/end pair (Not in C#): one view per cascade, the nearest
/// first, and what the shader needs to choose between them. With one cascade the view is
/// [`sun_shadow_view`]'s over the region, exactly.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SunCascades {
    /// One view per cascade, the nearest first.
    pub(crate) views: Vec<ShadowView>,
    /// The far end of each cascade: depth along `forward` from the camera (the last one's is the
    /// reach; entries past the last repeat it). Infinite for one cascade (the region's map has no
    /// reach).
    pub(crate) splits: [f32; 4],
    /// Each cascade's texel over the region's single map's texel (at most 1): the depth bias of a
    /// cascade is the single map's times this, the same bias per texel.
    pub(crate) texel_scales: [f32; 4],
    /// The camera's view direction, the axis the depth is measured along.
    pub(crate) forward: Vec3,
    /// The share of a slice's depth its blend band takes (0 to 0.5).
    pub(crate) blend: f32,
}

/// The sun's cascades for `settings` and a light travelling along `direction`, seen by `camera` on
/// a target of `aspect_ratio`: one cascade is [`sun_shadow_view`] over the region; more cut the
/// view in depth ([`cascade_splits`] out to [`cascade_reach`]) and fit an orthographic view along
/// the light to each slice ([`fit_cascade`]) - the LAST ONE to the whole view from the camera out
/// to the reach, so that it alone covers everything any cascade covers (what a receiver that reads
/// one map, the water, samples). None for no direction or an empty or non-finite region.
pub(crate) fn sun_cascades(settings: &SunShadowSettings, direction: Vec3, camera: &Camera, aspect_ratio: f32) -> Option<SunCascades> {
    let texels = settings.map_size.texels();
    let region = sun_shadow_view(direction, settings.region_min, settings.region_max, texels)?;
    // The view's own axis (the identity view of a degenerate camera looks down -Z).
    let forward = (-camera.view_matrix().inverse().z_axis.truncate()).normalize_or_zero();
    let count = settings.cascade_count() as usize;
    if count == 1 {
        return Some(SunCascades { views: vec![region], splits: [f32::INFINITY; 4], texel_scales: [1.0; 4], forward, blend: 0.0 });
    }
    let light = SunLightSpace::new(direction, settings.region_min, settings.region_max)?;
    let near = if camera.near_plane.is_finite() { camera.near_plane.max(1e-3) } else { 1e-3 };
    let reach = cascade_reach(settings, camera.position, near, camera.far_plane);
    let weight = if settings.cascade_split_weight.is_nan() { 0.7 } else { settings.cascade_split_weight.clamp(0.0, 1.0) };
    let splits = cascade_splits(near, reach, count, weight);
    let lateral = lateral_extent(camera, aspect_ratio);
    let mut views = Vec::with_capacity(count);
    let mut texel_scales = [1.0; 4];
    for cascade in 0..count {
        // The last cascade starts at the camera (it covers the whole view out to the reach).
        let start = if cascade == 0 || cascade == count - 1 { near } else { splits[cascade - 1] };
        let view = fit_cascade(&light, camera.position, forward, slice_sphere(start, splits[cascade], lateral), texels);
        texel_scales[cascade] = (view.texel / region.texel).min(1.0);
        views.push(view);
    }
    let blend = if settings.cascade_blend.is_nan() { 0.0 } else { settings.cascade_blend.clamp(0.0, 0.5) };
    Some(SunCascades { views, splits, texel_scales, forward, blend })
}

/// How far the cascades reach: the setting (a positive number; otherwise no limit of its own), at
/// most the camera's far plane and the region - the distance from `camera_position` to the
/// region's furthest corner rounded UP to the next of [`REACH_STEPS_PER_DOUBLING`] steps per
/// doubling, so that it changes only now and then while the camera moves - and at least twice the
/// near plane.
fn cascade_reach(settings: &SunShadowSettings, camera_position: Vec3, near: f32, far: f32) -> f32 {
    let furthest = (0..8).map(|corner| box_corner(settings.region_min, settings.region_max, corner).distance(camera_position)).fold(0.0f32, f32::max);
    let region = if furthest > 0.0 && furthest.is_finite() { 2.0f32.powf((furthest.log2() * REACH_STEPS_PER_DOUBLING).ceil() / REACH_STEPS_PER_DOUBLING) } else { f32::INFINITY };
    let limit = if settings.cascade_reach > 0.0 { settings.cascade_reach } else { f32::INFINITY };
    let far = if far.is_finite() && far > 0.0 { far } else { f32::INFINITY };
    let reach = limit.min(far).min(region);
    if reach.is_finite() {
        reach.max(near * 2.0)
    } else {
        near * 2.0
    }
}

/// The far end of each of `count` cascades between `near` and `reach` (the practical split scheme):
/// cut `i` of `count` is `weight` x the logarithmic cut near (reach / near)^(i / count) plus
/// (1 - `weight`) x the uniform cut near + (reach - near) i / count; the last is the reach, and the
/// entries past it repeat it.
fn cascade_splits(near: f32, reach: f32, count: usize, weight: f32) -> [f32; 4] {
    let count = count.clamp(1, 4);
    let mut splits = [reach; 4];
    for (cut, split) in splits.iter_mut().enumerate().take(count - 1) {
        let share = (cut + 1) as f32 / count as f32;
        let logarithmic = near * (reach / near).powf(share);
        let uniform = near + (reach - near) * share;
        *split = weight * logarithmic + (1.0 - weight) * uniform;
    }
    splits
}

/// How wide the camera's view is around its axis at a depth d: the distance from the axis to a
/// corner of the view is `.0 + .1 * d` (perspective: 0 + d x the tangent of the corner's angle;
/// orthographic: the half-diagonal of the visible rectangle + 0).
fn lateral_extent(camera: &Camera, aspect_ratio: f32) -> (f32, f32) {
    let aspect = if aspect_ratio > 0.0 { aspect_ratio } else { 1.0 };
    match camera.projection {
        ProjectionKind::Perspective => {
            let vertical = (camera.field_of_view * 0.5).tan();
            (0.0, (vertical * vertical * (1.0 + aspect * aspect)).sqrt())
        }
        ProjectionKind::Orthographic => {
            let half_height = camera.orthographic_height * 0.5;
            (half_height.abs() * (1.0 + aspect * aspect).sqrt(), 0.0)
        }
    }
}

/// The smallest sphere around the slice of the view from depth `start` to `end` whose centre is on
/// the view's axis: (the centre's depth, the radius). It holds the slice's eight corners; it
/// depends only on the depths, the field of view and the aspect ratio, never on where the camera
/// is or how it is turned (so a cascade keeps its size).
fn slice_sphere(start: f32, end: f32, lateral: (f32, f32)) -> (f32, f32) {
    let near_side = lateral.0 + lateral.1 * start;
    let far_side = lateral.0 + lateral.1 * end;
    let length = (end - start).max(1e-6);
    let center = ((end * end + far_side * far_side) - (start * start + near_side * near_side)) / (2.0 * length);
    let center = center.clamp(start, end);
    let radius = ((end - center).powi(2) + far_side * far_side).sqrt().max(((center - start).powi(2) + near_side * near_side).sqrt());
    (center, radius)
}

/// One cascade's view: the sun's light space of the region; across, a square window of the slice
/// sphere's diameter (with [`CASCADE_MARGIN_TEXELS`] free at each side) moved to lie inside the
/// region's footprint where it is smaller than the region (the whole footprint along an axis where
/// it is not) and SNAPPED to whole texels of a grid fixed in light space, so that the shadow does
/// not crawl while the camera moves; in depth, the region's range (as the single map's).
fn fit_cascade(light: &SunLightSpace, camera_position: Vec3, forward: Vec3, (depth, radius): (f32, f32), texels: u32) -> ShadowView {
    let size = texels as f32;
    let window = 2.0 * radius * size / (size - 2.0 * CASCADE_MARGIN_TEXELS);
    let center = light.view.transform_point3(camera_position + forward * depth);
    let (left, width) = fit_axis(center.x, window, light.low.x, light.high.x, size);
    let (bottom, height) = fit_axis(center.y, window, light.low.y, light.high.y, size);
    let (near, far) = light.depth_range();
    let projection = glam::camera::rh::proj::directx::orthographic(left, left + width, bottom, bottom + height, near, far.max(near + 1e-3));
    let view_projection = projection * light.view;
    ShadowView { view_projection, volume: CullVolume::from_view_projection(view_projection), texel: width.max(height) / size }
}

/// Along one light-space axis: (where the window starts, how wide it is) for a window of `window`
/// around `center` and the region's footprint `low..high`: the footprint itself when the window
/// would be as wide; else the window moved inside the footprint and its start snapped down to a
/// whole number of its texels (window / `texels`).
fn fit_axis(center: f32, window: f32, low: f32, high: f32, texels: f32) -> (f32, f32) {
    if window >= high - low {
        return (low, (high - low).max(1e-3));
    }
    let texel = window / texels;
    let start = (center - window * 0.5).clamp(low, high - window);
    ((start / texel).floor() * texel, window)
}

/// The shader's choice of a cascade for a point at `depth` along the camera's view (Not in C#;
/// the twin of `cascade_pick` in pbr_body.wgsl, with the same arithmetic): the first of the
/// `count` cascades whose far end (`splits`) is beyond the depth, and how much the next one
/// weighs there - 0 before the cascade's blend band (`blend` x its slice's depth, the slice
/// starting at 0 for the first), rising to 1 at its far end. None beyond the reach.
#[cfg(test)]
pub(crate) fn cascade_pick(depth: f32, splits: [f32; 4], count: usize, blend: f32) -> Option<(usize, f32)> {
    let mut start = 0.0f32;
    for (index, &end) in splits.iter().enumerate().take(count.min(4)) {
        if depth < end {
            let band = blend * (end - start);
            let weight = if band > 0.0 { ((depth - (end - band)) / band.max(1e-6)).clamp(0.0, 1.0) } else { 0.0 };
            return Some((index, weight));
        }
        start = end;
    }
    None
}

/// The light a receiver gets from the cascades (the twin of the end of `sun_visibility` in
/// pbr_body.wgsl): the chosen cascade's `near` mixed by `weight` into the next one's (`next`, or
/// 1 - no shadow - past the last cascade), as WGSL's `mix`.
#[cfg(test)]
pub(crate) fn blend_cascades(near: f32, next: Option<f32>, weight: f32) -> f32 {
    near * (1.0 - weight) + next.unwrap_or(1.0) * weight
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
        // No cascades unless asked: one map over the region.
        assert_eq!((sun.cascades, sun.cascade_reach, sun.cascade_split_weight, sun.cascade_blend), (1, f32::INFINITY, 0.7, 0.1));
        assert_eq!(covering.cascades, 1);
        let cascaded = covering.with_cascades(3, 250.0);
        assert_eq!((cascaded.cascades, cascaded.cascade_reach, cascaded.region_max, cascaded.enabled), (3, 250.0, Vec3::ONE, true));
        assert_eq!([0, 1, 2, 4, 5, 99].map(|count| SunShadowSettings { cascades: count, ..sun }.cascade_count()), [1, 1, 2, 4, 4, 4]);
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

#[cfg(test)]
mod cascade_tests {
    use glam::{Mat4, Vec3};

    use super::{blend_cascades, cascade_pick, cascade_reach, cascade_splits, lateral_extent, slice_sphere, sun_cascades, sun_shadow_view, ShadowMapSize, ShadowView, SunShadowSettings, CASCADE_MARGIN_TEXELS};
    use crate::drawing3d::{Camera, ProjectionKind};
    use crate::visibility::CullVolume;

    const SUN: Vec3 = Vec3::new(-0.4, -1.0, -0.5);

    fn ndc(view_projection: Mat4, point: Vec3) -> Vec3 {
        let clip = view_projection * point.extend(1.0);
        clip.truncate() / clip.w
    }

    fn bits(matrix: Mat4) -> [u32; 16] {
        matrix.to_cols_array().map(f32::to_bits)
    }

    /// The sun's view as the crate computed it before cascades, word for word (the reference the
    /// single map must still equal).
    fn single_map_before_cascades(direction: Vec3, min: Vec3, max: Vec3, texels: u32) -> Option<ShadowView> {
        let direction = direction.normalize_or_zero();
        if direction == Vec3::ZERO || !direction.is_finite() || !min.is_finite() || !max.is_finite() || !min.cmple(max).all() {
            return None;
        }
        let center = (min + max) * 0.5;
        let diagonal = (max - min).length().max(1e-3);
        let view = glam::camera::rh::view::look_to_mat4(center - direction * diagonal, direction, super::up_for(direction));
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for corner in 0..8 {
            let point = Vec3::new(if corner & 1 == 0 { min.x } else { max.x }, if corner & 2 == 0 { min.y } else { max.y }, if corner & 4 == 0 { min.z } else { max.z });
            let at = view.transform_point3(point);
            low = low.min(at);
            high = high.max(at);
        }
        let (near, far) = (-high.z - 2.0 * diagonal, -low.z);
        let width = (high.x - low.x).max(1e-3);
        let height = (high.y - low.y).max(1e-3);
        let projection = glam::camera::rh::proj::directx::orthographic(low.x, low.x + width, low.y, low.y + height, near, far.max(near + 1e-3));
        let view_projection = projection * view;
        Some(ShadowView { view_projection, volume: CullVolume::from_view_projection(view_projection), texel: width.max(height) / texels as f32 })
    }

    fn cameras() -> Vec<Camera> {
        let mut list = vec![Camera::default()];
        list.push(Camera { position: Vec3::new(120.0, 30.0, -40.0), target: Vec3::new(100.0, 0.0, 10.0), far_plane: 5000.0, ..Camera::default() });
        list.push(Camera { position: Vec3::new(-3.0, 1.7, 2.0), target: Vec3::new(-3.0, 1.6, -8.0), near_plane: 0.05, field_of_view: 0.8, ..Camera::default() });
        list.push(Camera { position: Vec3::new(0.0, 50.0, 0.0), target: Vec3::new(0.0, 0.0, 0.0), up: Vec3::Z, ..Camera::default() });
        list.push(Camera { projection: ProjectionKind::Orthographic, orthographic_height: 30.0, position: Vec3::new(10.0, 10.0, 10.0), ..Camera::default() });
        // A degenerate camera (its position is its target: the identity view).
        list.push(Camera { position: Vec3::ONE, target: Vec3::ONE, ..Camera::default() });
        list
    }

    /// The eight corners of the camera's view between depths `start` and `end`.
    fn slice_corners(camera: &Camera, aspect: f32, start: f32, end: f32) -> Vec<Vec3> {
        let inverse = camera.view_matrix().inverse();
        let (right, up, forward) = (inverse.x_axis.truncate(), inverse.y_axis.truncate(), -inverse.z_axis.truncate());
        let half = |depth: f32| match camera.projection {
            ProjectionKind::Perspective => {
                let v = depth * (camera.field_of_view * 0.5).tan();
                (v * aspect, v)
            }
            ProjectionKind::Orthographic => (camera.orthographic_height * 0.5 * aspect, camera.orthographic_height * 0.5),
        };
        let mut corners = Vec::new();
        for depth in [start, end] {
            let (w, h) = half(depth);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                corners.push(camera.position + forward * depth + right * (w * sx) + up * (h * sy));
            }
        }
        corners
    }

    #[test]
    fn one_cascade_is_the_single_map_bit_for_bit_for_every_camera() {
        let regions = [(Vec3::splat(-10.0), Vec3::splat(10.0)), (Vec3::new(-1585.0, -20.0, -1585.0), Vec3::new(1585.0, 300.0, 1585.0)), (Vec3::new(3.0, 0.0, -2.0), Vec3::new(3.5, 9.0, 40.0))];
        for (min, max) in regions {
            for size in [ShadowMapSize::Size1024, ShadowMapSize::Size2048, ShadowMapSize::Size4096] {
                // Whatever the other cascade settings say, one cascade ignores them.
                for settings in [SunShadowSettings { map_size: size, ..SunShadowSettings::covering(min, max) }, SunShadowSettings { map_size: size, cascade_reach: 40.0, cascade_split_weight: 0.0, cascade_blend: 0.5, cascades: 0, ..SunShadowSettings::covering(min, max) }] {
                    for camera in cameras() {
                        for direction in [SUN, Vec3::NEG_Y, Vec3::new(0.3, -0.2, 0.9)] {
                            let cascades = sun_cascades(&settings, direction, &camera, 1.6).expect("a region");
                            let before = single_map_before_cascades(direction, min, max, size.texels()).unwrap();
                            assert_eq!(cascades.views.len(), 1);
                            let view = &cascades.views[0];
                            assert_eq!(bits(view.view_projection), bits(before.view_projection), "the matrix, bit for bit");
                            assert_eq!(view.texel.to_bits(), before.texel.to_bits());
                            assert_eq!(view.volume, before.volume);
                            assert_eq!(*view, sun_shadow_view(direction, min, max, size.texels()).unwrap());
                            assert_eq!((cascades.texel_scales, cascades.blend), ([1.0; 4], 0.0));
                        }
                    }
                }
            }
        }
        assert!(sun_cascades(&SunShadowSettings::covering(Vec3::ONE, Vec3::ZERO), SUN, &Camera::default(), 1.0).is_none(), "an empty region: None, as before");
        assert!(sun_cascades(&SunShadowSettings::covering(Vec3::ZERO, Vec3::ONE).with_cascades(4, 10.0), Vec3::ZERO, &Camera::default(), 1.0).is_none());
    }

    #[test]
    fn the_splits_mix_equal_steps_and_equal_ratios() {
        let (near, reach) = (0.1f32, 1000.0f32);
        let uniform = cascade_splits(near, reach, 4, 0.0);
        let logarithmic = cascade_splits(near, reach, 4, 1.0);
        let practical = cascade_splits(near, reach, 4, 0.7);
        for cut in 0..3 {
            let share = (cut + 1) as f32 / 4.0;
            assert!((uniform[cut] - (near + (reach - near) * share)).abs() < 1e-3);
            assert!((logarithmic[cut] - near * 10000f32.powf(share)).abs() / logarithmic[cut] < 1e-5);
            assert!((practical[cut] - (0.7 * logarithmic[cut] + 0.3 * uniform[cut])).abs() < 1e-2);
        }
        // Logarithmic: equal ratios (1, 10, 100); uniform: 250, 500, 750.
        assert!((logarithmic[0] - 1.0).abs() < 1e-4 && (logarithmic[1] - 10.0).abs() < 1e-3 && (logarithmic[2] - 100.0).abs() < 1e-2);
        for splits in [uniform, logarithmic, practical, cascade_splits(near, reach, 4, 0.3)] {
            assert_eq!(splits[3], reach, "the last ends at the reach");
            assert!(splits.windows(2).all(|pair| pair[0] < pair[1]), "{splits:?}");
            assert!(splits[0] > near);
        }
        // Fewer cascades: the entries past the last repeat the reach.
        assert_eq!(cascade_splits(1.0, 300.0, 2, 0.5)[1..], [300.0; 3]);
        let two = cascade_splits(1.0, 300.0, 2, 0.5)[0];
        assert!((two - (0.5 * 300f32.sqrt() + 0.5 * 150.5)).abs() < 1e-3);
        assert_eq!(cascade_splits(1.0, 300.0, 1, 0.5), [300.0; 4]);
    }

    #[test]
    fn the_default_reach_is_the_region_in_steps_and_never_beyond_it_or_the_far_plane() {
        let region = SunShadowSettings::covering(Vec3::new(-100.0, 0.0, -100.0), Vec3::new(100.0, 20.0, 100.0));
        let inside = Vec3::new(10.0, 2.0, -30.0);
        let furthest = (Vec3::new(-100.0, 20.0, 100.0) - inside).length();
        let reach = cascade_reach(&region, inside, 0.1, 5000.0);
        assert!(reach >= furthest && reach < furthest * 2f32.powf(0.25) * 1.0001, "{reach} for {furthest}");
        // A step of 2^(1/4): log2 of the reach is a whole number of quarters.
        assert!(((reach.log2() * 4.0) - (reach.log2() * 4.0).round()).abs() < 1e-3);
        // A small move keeps it (the cascades keep their sizes).
        assert_eq!(cascade_reach(&region, inside + Vec3::new(0.3, 0.0, -0.2), 0.1, 5000.0), reach);
        // The far plane and an explicit reach limit it; neither goes past the region.
        assert_eq!(cascade_reach(&region, inside, 0.1, 100.0), 100.0);
        assert_eq!(cascade_reach(&SunShadowSettings { cascade_reach: 60.0, ..region }, inside, 0.1, 5000.0), 60.0);
        assert_eq!(cascade_reach(&SunShadowSettings { cascade_reach: 1.0e6, ..region }, inside, 0.1, 5000.0), reach);
        for unset in [0.0, -5.0, f32::NAN, f32::INFINITY] {
            assert_eq!(cascade_reach(&SunShadowSettings { cascade_reach: unset, ..region }, inside, 0.1, 5000.0), reach, "{unset} counts as no limit");
        }
        // Never at or inside the near plane.
        assert_eq!(cascade_reach(&SunShadowSettings { cascade_reach: 0.01, ..region }, inside, 0.1, 5000.0), 0.2);
    }

    #[test]
    fn a_slice_sphere_holds_the_slice_and_does_not_turn_with_the_camera() {
        for (fov, aspect) in [(std::f32::consts::FRAC_PI_3, 16.0 / 9.0), (0.6, 1.0), (1.4, 0.5)] {
            let camera = Camera { field_of_view: fov, position: Vec3::new(5.0, 2.0, 1.0), target: Vec3::new(-20.0, -3.0, 7.0), ..Camera::default() };
            let lateral = lateral_extent(&camera, aspect);
            for (start, end) in [(0.1, 8.0), (8.0, 40.0), (40.0, 900.0), (0.1, 900.0)] {
                let (depth, radius) = slice_sphere(start, end, lateral);
                let center = camera.position + camera.forward() * depth;
                for corner in slice_corners(&camera, aspect, start, end) {
                    assert!(corner.distance(center) <= radius * 1.0001, "fov {fov}: {start}..{end}");
                }
                // The smallest such sphere: some corner is on it.
                assert!(slice_corners(&camera, aspect, start, end).iter().any(|corner| corner.distance(center) >= radius * 0.999));
            }
        }
        // Orthographic: the box's half-diagonal.
        let ortho = Camera { projection: ProjectionKind::Orthographic, orthographic_height: 10.0, ..Camera::default() };
        let (depth, radius) = slice_sphere(0.0, 10.0, lateral_extent(&ortho, 2.0));
        assert!((depth - 5.0).abs() < 1e-5 && (radius - (5.0f32 * 5.0 + 10.0 * 10.0 + 5.0 * 5.0).sqrt()).abs() < 1e-4);
    }

    #[test]
    fn each_cascade_holds_its_slice_inside_the_region_and_its_casters_up_light() {
        let (min, max) = (Vec3::new(-1585.0, -20.0, -1585.0), Vec3::new(1585.0, 300.0, 1585.0));
        let settings = SunShadowSettings::covering(min, max).with_cascades(4, 400.0);
        let aspect = 16.0 / 9.0;
        let towards_sun = -SUN.normalize();
        for camera in [
            Camera { position: Vec3::new(30.0, 2.0, 40.0), target: Vec3::new(60.0, 0.0, -10.0), far_plane: 5000.0, ..Camera::default() },
            Camera { position: Vec3::new(-700.0, 120.0, 900.0), target: Vec3::new(-650.0, 0.0, 800.0), far_plane: 5000.0, field_of_view: 0.7, ..Camera::default() },
        ] {
            let cascades = sun_cascades(&settings, SUN, &camera, aspect).unwrap();
            assert_eq!(cascades.views.len(), 4);
            assert_eq!(cascades.splits[3], 400.0);
            assert!((cascades.forward - camera.forward()).length() < 1e-5);
            let mut previous = 0.0;
            for (index, view) in cascades.views.iter().enumerate() {
                // The last cascade covers the whole view from the camera to the reach.
                let start = if index == 0 || index == 3 { camera.near_plane } else { cascades.splits[index - 1] };
                for corner in slice_corners(&camera, aspect, start, cascades.splits[index]) {
                    let at = ndc(view.view_projection, corner);
                    // Inside the map with the margin to spare (in texels of a 2048 map).
                    let limit = 1.0 - 2.0 * (CASCADE_MARGIN_TEXELS - 1.0) / 2048.0;
                    assert!(at.x.abs() <= limit && at.y.abs() <= limit && at.z > 0.0 && at.z < 1.0, "cascade {index}: {at}");
                    assert!(view.volume.contains_point(corner));
                    // What stands between the slice and the sun, inside the region's depth, casts into it.
                    assert!(view.volume.contains_point(corner + towards_sun * 250.0), "cascade {index}: a caster up-light");
                }
                // Nearer is finer; the scale of the bias is the texel over the single map's.
                assert!(view.texel > previous);
                previous = view.texel;
                let single = sun_shadow_view(SUN, min, max, 2048).unwrap();
                assert!((cascades.texel_scales[index] - view.texel / single.texel).abs() < 1e-6 && cascades.texel_scales[index] < 1.0);
            }
            // The first cascade is far finer than the single map over the whole region.
            assert!(cascades.views[0].texel < sun_shadow_view(SUN, min, max, 2048).unwrap().texel / 20.0);
        }
    }

    #[test]
    fn a_cascade_never_leaves_the_region_and_one_wider_than_the_region_is_the_single_map() {
        // A small region far from a camera with a long reach: every cascade's sphere is wider than
        // the region, so every cascade is the single map's view, bit for bit.
        let (min, max) = (Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 3.0, 5.0));
        let settings = SunShadowSettings::covering(min, max).with_cascades(3, f32::INFINITY);
        let camera = Camera { position: Vec3::new(0.0, 20.0, 60.0), ..Camera::default() };
        let cascades = sun_cascades(&settings, SUN, &camera, 1.5).unwrap();
        let single = sun_shadow_view(SUN, min, max, 2048).unwrap();
        assert_eq!(cascades.views.len(), 3);
        for view in &cascades.views {
            assert_eq!(bits(view.view_projection), bits(single.view_projection));
        }
        assert_eq!(cascades.texel_scales[..3], [1.0; 3]);
        // A camera at the region's edge looking out: the window stays inside the region's
        // footprint (and still holds the part of the slice inside the region).
        let (min, max) = (Vec3::new(-500.0, 0.0, -500.0), Vec3::new(500.0, 50.0, 500.0));
        let settings = SunShadowSettings::covering(min, max).with_cascades(4, 300.0);
        let camera = Camera { position: Vec3::new(495.0, 2.0, 0.0), target: Vec3::new(600.0, 2.0, 0.0), far_plane: 4000.0, ..Camera::default() };
        let cascades = sun_cascades(&settings, SUN, &camera, 1.5).unwrap();
        let single = sun_shadow_view(SUN, min, max, 2048).unwrap();
        for view in &cascades.views {
            // The window's corners, back in the single map: inside it (to within one texel of the snap).
            let inverse = view.view_projection.inverse();
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let at = ndc(single.view_projection, inverse.project_point3(Vec3::new(x, y, 0.5)));
                assert!(at.x.abs() <= 1.0 + 2.0 / 2048.0 && at.y.abs() <= 1.0 + 2.0 / 2048.0, "{at}");
            }
            // The slice's points inside the region are inside the cascade.
            assert!(view.volume.contains_point(Vec3::new(497.0, 1.0, 0.5)));
        }
    }

    #[test]
    fn the_cascades_move_in_whole_texels_and_keep_their_size_as_the_camera_turns() {
        let (min, max) = (Vec3::new(-1585.0, -20.0, -1585.0), Vec3::new(1585.0, 300.0, 1585.0));
        let settings = SunShadowSettings::covering(min, max).with_cascades(4, 500.0);
        let base = Camera { position: Vec3::new(12.0, 3.0, 25.0), target: Vec3::new(40.0, 0.0, -30.0), far_plane: 5000.0, ..Camera::default() };
        let before = sun_cascades(&settings, SUN, &base, 1.5).unwrap();
        let fixed = [Vec3::new(20.0, 0.0, 0.0), Vec3::new(-7.0, 5.0, 31.0)];
        for step in [Vec3::new(0.013, 0.0, 0.0), Vec3::new(0.21, 0.05, -0.37), Vec3::new(1.7, 0.0, 2.3)] {
            let moved = Camera { position: base.position + step, target: base.target + step, ..base };
            let after = sun_cascades(&settings, SUN, &moved, 1.5).unwrap();
            assert_eq!(after.splits, before.splits);
            for (a, b) in before.views.iter().zip(&after.views) {
                assert_eq!(a.texel, b.texel, "the same size");
                for point in fixed {
                    // A fixed world point moves across the map by a whole number of texels
                    // (2 / 2048 of clip space each), or not at all.
                    let shift = (ndc(b.view_projection, point) - ndc(a.view_projection, point)).truncate() * (2048.0 / 2.0);
                    assert!((shift.x - shift.x.round()).abs() < 0.02 && (shift.y - shift.y.round()).abs() < 0.02, "a shift of {shift} texels for a step of {step}");
                    // Depth does not depend on the camera.
                    assert!((ndc(b.view_projection, point).z - ndc(a.view_projection, point).z).abs() < 1e-6);
                }
            }
        }
        // Turning the camera in place: every cascade keeps its texel (its size).
        for target in [Vec3::new(-40.0, 10.0, 0.0), Vec3::new(12.0, -50.0, 26.0), Vec3::new(80.0, 3.0, 25.0)] {
            let turned = sun_cascades(&settings, SUN, &Camera { target, ..base }, 1.5).unwrap();
            for (a, b) in before.views.iter().zip(&turned.views) {
                assert_eq!(a.texel, b.texel);
            }
        }
    }

    #[test]
    fn the_cascade_choice_and_its_blend_band_twin_the_shader() {
        let splits = [10.0, 30.0, 80.0, 200.0];
        // Before the band: that cascade alone; in it: rising to the next; at the end: the next cascade.
        assert_eq!(cascade_pick(0.5, splits, 4, 0.1), Some((0, 0.0)));
        assert_eq!(cascade_pick(8.99, splits, 4, 0.1), Some((0, 0.0)));
        let (index, weight) = cascade_pick(9.5, splits, 4, 0.1).unwrap();
        assert!(index == 0 && (weight - 0.5).abs() < 1e-5);
        assert_eq!(cascade_pick(10.0, splits, 4, 0.1), Some((1, 0.0)));
        // Cascade 1's slice is 10..30: its band is the last 2.
        let (index, weight) = cascade_pick(29.0, splits, 4, 0.1).unwrap();
        assert!(index == 1 && (weight - 0.5).abs() < 1e-5);
        assert_eq!(cascade_pick(27.9, splits, 4, 0.1), Some((1, 0.0)));
        // The last fades out towards the reach; beyond it, nothing.
        let (index, weight) = cascade_pick(195.0, splits, 4, 0.1).unwrap();
        assert!(index == 3 && (weight - (195.0 - 188.0) / 12.0).abs() < 1e-5);
        assert_eq!(cascade_pick(200.0, splits, 4, 0.1), None);
        // Fewer cascades: the reach is the last one's end; a hard switch has no band.
        assert_eq!(cascade_pick(50.0, splits, 2, 0.1), None);
        assert_eq!(cascade_pick(29.9, splits, 4, 0.0), Some((1, 0.0)));
        // Behind the camera (a normal offset): the first.
        assert_eq!(cascade_pick(-0.2, splits, 4, 0.1), Some((0, 0.0)));
        // The blend: the near cascade's light mixed into the next one's, or into 1 past the last.
        assert_eq!(blend_cascades(0.2, Some(0.6), 0.0), 0.2);
        assert!((blend_cascades(0.2, Some(0.6), 0.5) - 0.4).abs() < 1e-6);
        assert_eq!(blend_cascades(0.2, Some(0.6), 1.0), 0.6);
        assert!((blend_cascades(0.0, None, 0.25) - 0.25).abs() < 1e-6);
        // The WGSL copy has the same arithmetic.
        let body = crate::shaders::PbrShader::BODY;
        for line in ["let band = frame.sun_view.w * (end - start);", "let weight = select(0.0, clamp((depth - (end - band)) / max(band, 1e-6), 0.0, 1.0), band > 0.0);", "if (depth < end) {", "return vec2<f32>(-1.0, 0.0);", "return mix(near, next, pick.y);", "var next = 1.0;"] {
            assert!(body.contains(line), "pbr_body.wgsl: {line}");
        }
    }
}
