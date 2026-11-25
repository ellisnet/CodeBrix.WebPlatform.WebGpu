//! [`PointLight`], [`SpotLight`] and [`LocalLight`]: lights at a place in the scene, which the model
//! renderer shades beside the directional lights of [`SceneLighting`](crate::SceneLighting) (Not in
//! C#), with the curves they light by and the choice of the lights that matter most to a view.
//! The curves here are the shader's, written again in Rust so that they are tested on the host.

use glam::Vec3;

use crate::common::Color4;
use crate::visibility::CullVolume;

/// The distance below which a local light no longer brightens (the inverse-square law would grow
/// without bound at the light itself): 1 cm.
const MIN_DISTANCE: f32 = 0.01;

/// The widest a spotlight's cone may be, from its axis: a quarter turn (a half space).
pub(crate) const MAX_CONE_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

/// A light at a point that shines the same way in every direction (Not in C#): a lamp, a candle, a
/// glowing sign. It lights a surface with `color * intensity` at 1 m, by the inverse-square law
/// eased to exactly zero at `range` ([`LocalLight::falloff`]); at 1 m a point light of intensity
/// `i` lights as much as a directional light of intensity `i`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    /// Where the light is, in world space.
    pub position: Vec3,
    /// The colour, linear.
    pub color: Color4,
    /// Scales the colour; 0 or less switches the light off.
    pub intensity: f32,
    /// How far the light reaches, in world units: its light falls to exactly zero there. Must be
    /// above 0 (a light without a positive range is left out); `f32::INFINITY` for a light that
    /// never ends (plain inverse square - it can never be skipped for being out of reach).
    pub range: f32,
}

impl PointLight {
    /// A point light.
    pub fn new(position: Vec3, color: Color4, intensity: f32, range: f32) -> PointLight {
        PointLight { position, color, intensity, range }
    }
}

/// A light at a point that shines in a cone (Not in C#): a torch, a stage light, a car's
/// headlamp. As a [`PointLight`], plus the direction of the cone's axis and its two angles: full
/// brightness inside `inner_angle` of the axis, eased smoothly to nothing at `outer_angle`
/// ([`SpotLight::cone`]). With `casts_shadows` it casts shadows (see
/// [`SpotShadowSettings`](crate::SpotShadowSettings)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotLight {
    /// Where the light is, in world space.
    pub position: Vec3,
    /// The direction the light shines (the cone's axis), world space; need not be unit length. A
    /// zero direction switches the light off.
    pub direction: Vec3,
    /// The colour, linear.
    pub color: Color4,
    /// Scales the colour; 0 or less switches the light off.
    pub intensity: f32,
    /// How far the light reaches (as [`PointLight::range`]).
    pub range: f32,
    /// The angle from the axis, in radians, inside which the light is at full brightness; at most
    /// `outer_angle` (a larger value counts as `outer_angle`: a hard edge).
    pub inner_angle: f32,
    /// The angle from the axis, in radians, where the light has faded to nothing; at most a
    /// quarter turn (larger counts as a quarter turn).
    pub outer_angle: f32,
    /// True when the light casts shadows; default false. At most
    /// [`SceneLighting::MAX_SHADOWED_SPOTLIGHTS`](crate::SceneLighting::MAX_SHADOWED_SPOTLIGHTS)
    /// spotlights cast shadows in a frame (the ones that matter most to the view); the others
    /// still light, without shadows.
    pub casts_shadows: bool,
}

impl SpotLight {
    /// A spotlight that casts no shadows.
    pub fn new(position: Vec3, direction: Vec3, color: Color4, intensity: f32, range: f32, inner_angle: f32, outer_angle: f32) -> SpotLight {
        SpotLight { position, direction, color, intensity, range, inner_angle, outer_angle, casts_shadows: false }
    }

    /// The same spotlight, casting shadows (or not).
    pub fn with_shadows(self, casts_shadows: bool) -> SpotLight {
        SpotLight { casts_shadows, ..self }
    }

    /// The outer angle as the renderer uses it: within 0.001 radians and a quarter turn.
    pub(crate) fn clamped_outer_angle(&self) -> f32 {
        if self.outer_angle.is_nan() {
            return MAX_CONE_ANGLE;
        }
        self.outer_angle.clamp(0.001, MAX_CONE_ANGLE)
    }

    /// The inner angle as the renderer uses it: within 0 and the outer angle.
    pub(crate) fn clamped_inner_angle(&self) -> f32 {
        if self.inner_angle.is_nan() {
            return 0.0;
        }
        self.inner_angle.clamp(0.0, self.clamped_outer_angle())
    }

    /// How much of the light reaches a direction `angle` radians off the cone's axis, 0 to 1: 1
    /// within the inner angle, 0 at and beyond the outer one, and between them a smooth step
    /// (`t * t * (3 - 2t)` of where the angle's cosine lies between the two angles' cosines).
    pub fn cone(&self, angle: f32) -> f32 {
        cone_factor(angle.cos(), self.clamped_inner_angle().cos(), self.clamped_outer_angle().cos())
    }
}

/// A local light of [`SceneLighting::local_lights`](crate::SceneLighting::local_lights): a point
/// light or a spotlight (Not in C#). The model renderer shades at most
/// [`SceneLighting::MAX_LOCAL_LIGHTS`](crate::SceneLighting::MAX_LOCAL_LIGHTS) of them per frame;
/// over that it uses the ones that matter most to the view ([`LocalLight::importance`]) and
/// skips the others for that frame. Positions and directions are read at every
/// [`ModelRenderer::begin`](crate::ModelRenderer::begin), so lights can move (or ride on a node:
/// [`ModelInstance::get_node_world_matrix_named`](crate::ModelInstance::get_node_world_matrix_named)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocalLight {
    /// A point light.
    Point(PointLight),
    /// A spotlight.
    Spot(SpotLight),
}

impl From<PointLight> for LocalLight {
    fn from(light: PointLight) -> LocalLight {
        LocalLight::Point(light)
    }
}

impl From<SpotLight> for LocalLight {
    fn from(light: SpotLight) -> LocalLight {
        LocalLight::Spot(light)
    }
}

impl LocalLight {
    /// Where the light is.
    pub fn position(&self) -> Vec3 {
        match self {
            LocalLight::Point(light) => light.position,
            LocalLight::Spot(light) => light.position,
        }
    }

    /// The colour times the intensity, linear RGB.
    pub fn radiance(&self) -> Vec3 {
        let (color, intensity) = match self {
            LocalLight::Point(light) => (light.color, light.intensity),
            LocalLight::Spot(light) => (light.color, light.intensity),
        };
        Vec3::new(color.r, color.g, color.b) * intensity
    }

    /// How far the light reaches.
    pub fn range(&self) -> f32 {
        match self {
            LocalLight::Point(light) => light.range,
            LocalLight::Spot(light) => light.range,
        }
    }

    /// The spotlight, if this is one.
    pub fn as_spot(&self) -> Option<&SpotLight> {
        match self {
            LocalLight::Spot(light) => Some(light),
            LocalLight::Point(_) => None,
        }
    }

    /// True when the light can light anything: a positive range, a colour and intensity that
    /// give some light, finite numbers, and (a spotlight) a direction.
    pub fn is_lit(&self) -> bool {
        let radiance = self.radiance();
        let range = self.range();
        let direction_ok = self.as_spot().is_none_or(|spot| spot.direction.is_finite() && spot.direction.length_squared() > 0.0);
        radiance.is_finite() && radiance.max_element() > 0.0 && range > 0.0 && !range.is_nan() && self.position().is_finite() && direction_ok
    }

    /// The distance curve every local light lights by, 0 to 1 / (1 cm)^2: the inverse square of
    /// the distance, eased to EXACTLY zero at `range` by the window `(1 - (d / range)^4)^2` (the
    /// curve glTF's KHR_lights_punctual recommends): `window / max(d, 0.01)^2`. At 1 m it is
    /// (almost) 1 for a range much larger than 1 m; at and beyond the range it is 0. An infinite
    /// range gives the plain inverse square.
    pub fn falloff(distance: f32, range: f32) -> f32 {
        let inverse_range = if range.is_finite() { 1.0 / range } else { 0.0 };
        falloff_factor(distance, inverse_range)
    }

    /// The light that reaches a point, as a factor of [`LocalLight::radiance`] (before the
    /// surface's angle to the light and before shadows): the falloff, times the cone for a
    /// spotlight. 0 for a light that is not lit ([`LocalLight::is_lit`]).
    pub fn light_at(&self, point: Vec3) -> f32 {
        if !self.is_lit() {
            return 0.0;
        }
        let to_point = point - self.position();
        let distance = to_point.length();
        let mut factor = LocalLight::falloff(distance, self.range());
        if let Some(spot) = self.as_spot() {
            let axis = spot.direction.normalize();
            let cosine = if distance > 0.0 { to_point.dot(axis) / distance } else { 1.0 };
            factor *= cone_factor(cosine, spot.clamped_inner_angle().cos(), spot.clamped_outer_angle().cos());
        }
        factor
    }

    /// How much the light matters to a view from `camera_position`, the order in which the
    /// renderer keeps lights when there are more than it shades: the brightest channel of
    /// [`LocalLight::radiance`] over the squared distance from the camera (at least 1 m). 0 for a
    /// light that is not lit.
    pub fn importance(&self, camera_position: Vec3) -> f32 {
        if !self.is_lit() {
            return 0.0;
        }
        let distance_squared = self.position().distance_squared(camera_position).max(1.0);
        self.radiance().max_element() / distance_squared
    }
}

/// The shader's distance curve, with the range given as its inverse (0 = infinite).
pub(crate) fn falloff_factor(distance: f32, inverse_range: f32) -> f32 {
    let x = distance * inverse_range;
    let x2 = x * x;
    let window = (1.0 - x2 * x2).clamp(0.0, 1.0);
    let d = distance.max(MIN_DISTANCE);
    window * window / (d * d)
}

/// The shader's cone curve on cosines: 1 at or inside the inner cone, 0 at or outside the outer
/// one, a smooth step between (a hard step where the two are equal).
pub(crate) fn cone_factor(cosine: f32, cos_inner: f32, cos_outer: f32) -> f32 {
    let t = ((cosine - cos_outer) / (cos_inner - cos_outer).max(1e-4)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// True when a sphere is not entirely outside the volume (outside one plane).
pub(crate) fn sphere_in_volume(volume: &CullVolume, center: Vec3, radius: f32) -> bool {
    volume.planes().iter().all(|plane| plane.truncate().dot(center) + plane.w >= -radius)
}

/// The local lights the renderer shades for a view, at most `limit`, as indices into `lights`,
/// most important first: every light that is lit ([`LocalLight::is_lit`]) and whose reach (the
/// sphere of its range) is not entirely outside `view`, ordered by [`LocalLight::importance`]
/// (equal importance keeps the list order).
pub(crate) fn select_local_lights(lights: &[LocalLight], camera_position: Vec3, view: &CullVolume, limit: usize) -> Vec<usize> {
    let mut chosen: Vec<(usize, f32)> = lights
        .iter()
        .enumerate()
        .filter(|(_, light)| light.is_lit() && sphere_in_volume(view, light.position(), light.range()))
        .map(|(index, light)| (index, light.importance(camera_position)))
        .collect();
    chosen.sort_by(|a, b| b.1.total_cmp(&a.1));
    chosen.truncate(limit);
    chosen.into_iter().map(|(index, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{cone_factor, falloff_factor, select_local_lights, LocalLight, PointLight, SpotLight};
    use crate::drawing3d::Camera;
    use crate::visibility::CullVolume;
    use crate::Color4;

    fn point(position: Vec3, intensity: f32, range: f32) -> LocalLight {
        PointLight::new(position, Color4::WHITE, intensity, range).into()
    }

    #[test]
    fn the_falloff_is_inverse_square_eased_to_exactly_zero_at_the_range() {
        // Far inside a long range: the inverse square.
        assert!((LocalLight::falloff(1.0, 1000.0) - 1.0).abs() < 1e-6);
        assert!((LocalLight::falloff(2.0, 1000.0) - 0.25).abs() < 1e-6);
        assert!((LocalLight::falloff(4.0, 1000.0) - 1.0 / 16.0).abs() < 1e-5);
        // Exactly zero at and beyond the range, and falling all the way to it.
        assert_eq!(LocalLight::falloff(10.0, 10.0), 0.0);
        assert_eq!(LocalLight::falloff(25.0, 10.0), 0.0);
        let mut previous = f32::MAX;
        for step in 1..=100 {
            let value = LocalLight::falloff(step as f32 * 0.1, 10.0);
            assert!(value < previous, "not falling at {step}");
            assert!(value >= 0.0);
            previous = value;
        }
        // The window: at half the range (1 - 1/16)^2 of the inverse square.
        let expected = (1.0f32 - 1.0 / 16.0).powi(2) / 25.0;
        assert!((LocalLight::falloff(5.0, 10.0) - expected).abs() < 1e-7);
        // No end with an infinite range; capped at 1 cm at the light itself.
        assert!((LocalLight::falloff(100.0, f32::INFINITY) - 1e-4).abs() < 1e-9);
        assert_eq!(LocalLight::falloff(0.0, 10.0), LocalLight::falloff(0.01, 10.0));
        assert_eq!(falloff_factor(3.0, 0.1), LocalLight::falloff(3.0, 10.0));
    }

    #[test]
    fn the_cone_is_full_inside_the_inner_angle_and_eases_to_zero_at_the_outer() {
        let spot = SpotLight::new(Vec3::ZERO, Vec3::NEG_Y, Color4::WHITE, 1.0, 10.0, 20f32.to_radians(), 40f32.to_radians());
        assert_eq!(spot.cone(0.0), 1.0);
        assert_eq!(spot.cone(20f32.to_radians()), 1.0);
        assert_eq!(spot.cone(40f32.to_radians()), 0.0);
        assert_eq!(spot.cone(60f32.to_radians()), 0.0);
        let mut previous = 1.0;
        for degrees in 21..40 {
            let value = spot.cone((degrees as f32).to_radians());
            assert!(value < previous && value > 0.0, "{degrees}: {value}");
            previous = value;
        }
        // Smooth at both ends: the steps next to the edges are small.
        assert!(1.0 - spot.cone(20.5f32.to_radians()) < 0.01);
        assert!(spot.cone(39.5f32.to_radians()) < 0.01);
        // Inner beyond outer: a hard edge at the outer angle; the outer angle at most a quarter turn.
        let hard = SpotLight { inner_angle: 1.0, outer_angle: 0.5, ..spot };
        assert_eq!(hard.cone(0.49), 1.0);
        assert_eq!(hard.cone(0.51), 0.0);
        let wide = SpotLight { outer_angle: 3.0, ..spot };
        assert_eq!(wide.cone(std::f32::consts::FRAC_PI_2 + 0.01), 0.0);
        assert_eq!(cone_factor(1.0, 0.9, 0.8), 1.0);
    }

    #[test]
    fn light_at_combines_the_falloff_and_the_cone() {
        let spot = LocalLight::from(SpotLight::new(Vec3::new(0.0, 4.0, 0.0), Vec3::new(0.0, -2.0, 0.0), Color4::WHITE, 3.0, 8.0, 0.3, 0.6));
        let below = spot.light_at(Vec3::ZERO);
        assert!((below - LocalLight::falloff(4.0, 8.0)).abs() < 1e-7);
        // To the side (45 degrees off the axis): outside the outer cone.
        assert_eq!(spot.light_at(Vec3::new(4.0, 0.0, 0.0)), 0.0);
        // Behind the lamp.
        assert_eq!(spot.light_at(Vec3::new(0.0, 6.0, 0.0)), 0.0);
        // Switched off: no intensity, no range, no direction.
        assert_eq!(point(Vec3::ZERO, 0.0, 5.0).light_at(Vec3::X), 0.0);
        assert_eq!(point(Vec3::ZERO, 1.0, 0.0).light_at(Vec3::X), 0.0);
        let no_direction = LocalLight::from(SpotLight::new(Vec3::ZERO, Vec3::ZERO, Color4::WHITE, 1.0, 5.0, 0.3, 0.6));
        assert!(!no_direction.is_lit());
    }

    #[test]
    fn the_sixteen_that_matter_most_to_the_view_are_chosen() {
        // The default camera at z = 5 looking at the origin.
        let camera = Camera::default();
        let view = CullVolume::from_camera(&camera, 1.0);
        let mut lights: Vec<LocalLight> = Vec::new();
        // 20 equal lights in a row in front of the camera, further and further away.
        for i in 0..20 {
            lights.push(point(Vec3::new(0.0, 0.0, -(i as f32) * 2.0), 1.0, 5.0));
        }
        // A very bright one far away still counts; one behind the camera whose reach does not get
        // into the view does not; one switched off does not.
        lights.push(point(Vec3::new(0.0, 0.0, -60.0), 10000.0, 5.0));
        lights.push(point(Vec3::new(0.0, 0.0, 30.0), 100.0, 5.0));
        lights.push(point(Vec3::new(0.0, 0.0, -1.0), 0.0, 5.0));
        let chosen = select_local_lights(&lights, camera.position, &view, 16);
        assert_eq!(chosen.len(), 16);
        assert_eq!(chosen[0], 20, "the brightest first");
        assert!(!chosen.contains(&21) && !chosen.contains(&22));
        // The nearest fifteen of the row follow, nearest first; the five furthest are skipped.
        assert_eq!(&chosen[1..], &(0..15).collect::<Vec<usize>>()[..]);
        // Behind the camera but reaching into the view: kept.
        let mut reaching = lights.clone();
        reaching[21] = point(Vec3::new(0.0, 0.0, 6.0), 100.0, 5.0);
        assert!(select_local_lights(&reaching, camera.position, &view, 16).contains(&21));
        // Fewer than the limit: all that light the view, in order of importance.
        assert_eq!(select_local_lights(&lights[..3], camera.position, &view, 16), vec![0, 1, 2]);
        // A light whose reach is entirely off to the side: skipped.
        let side = [point(Vec3::new(500.0, 0.0, 0.0), 1.0, 10.0), point(Vec3::new(500.0, 0.0, 0.0), 1.0, f32::INFINITY)];
        assert_eq!(select_local_lights(&side, camera.position, &view, 16), vec![1]);
        // Culling volume with no planes holds everything: both kept.
        assert_eq!(select_local_lights(&side, camera.position, &CullVolume::from_planes([]), 16), vec![0, 1]);
    }
}
