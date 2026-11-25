//! [`SceneLighting`], [`DirectionalLight`] and [`AmbientLighting`]: the lighting the mesh
//! renderer, the model renderer and the skybox apply (C#: `SceneLighting`), with the model
//! renderer's local lights and shadows (Not in C#).

use glam::Vec3;

use crate::common::Color4;
use crate::lighting::{EnvironmentMap, LocalLight, SpotShadowSettings, SunShadowSettings};

/// Where the model renderer's ambient (indirect) light comes from (C#: `AmbientLighting`). The
/// mesh renderer always uses [`SceneLighting::ambient_color`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AmbientLighting {
    /// Image-based lighting from [`SceneLighting::environment`] (or the context's default
    /// procedural sky when that is None), with the split-sum BRDF LUT (the C# default).
    #[default]
    Environment,
    /// A colour gradient between the sky colour ([`SceneLighting::ambient_color`]) and the ground
    /// colour ([`SceneLighting::ambient_ground_color`]) around [`SceneLighting::ambient_up`].
    Hemisphere,
}

/// A directional light: the direction it travels in, a linear colour and an intensity (C#:
/// `DirectionalLight`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// The direction the light travels (C#: `Direction`).
    pub direction: Vec3,
    /// The colour, linear (C#: `Color`).
    pub color: Color4,
    /// Scales the colour (C#: `Intensity`).
    pub intensity: f32,
}

impl DirectionalLight {
    /// A light (C#: `new DirectionalLight(direction, color, intensity = 1)`).
    pub fn new(direction: Vec3, color: Color4, intensity: f32) -> DirectionalLight {
        DirectionalLight { direction, color, intensity }
    }
}

/// The lighting of a scene (C#: `SceneLighting`; its properties are the public fields, with the
/// same defaults): a key directional light, more directional lights (the model renderer's; the
/// mesh renderer uses the key light only), an ambient term and an exposure. Everything is linear.
///
/// Not in C# (the model renderer's; the mesh renderer and the skybox ignore them): LOCAL LIGHTS
/// ([`SceneLighting::local_lights`], point lights and spotlights, at most
/// [`SceneLighting::MAX_LOCAL_LIGHTS`] shaded per frame) and SHADOWS - the key light's
/// ([`SceneLighting::sun_shadows`], off by default) and those of the spotlights that cast them
/// ([`SceneLighting::spot_shadows`]). A lighting with no local lights and the sun's shadow off
/// lights exactly as before, and costs nothing more.
///
/// `environment` is a shared handle ([`EnvironmentMap`] clones are the same environment), so
/// cloning the lighting is cheap and both clones light with the same environment.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLighting {
    /// The direction the key light TRAVELS, from the light into the scene (C#: `LightDirection`,
    /// default a sun high to the front-left, normalize(-0.4, -1, -0.5)).
    pub light_direction: Vec3,
    /// The key light's colour, linear (C#: `LightColor`, default white).
    pub light_color: Color4,
    /// Scales the key light's colour; 0 switches it off (C#: `LightIntensity`, default 1).
    pub light_intensity: f32,
    /// More directional lights, for the model renderer (C#: `AdditionalLights`, empty).
    pub additional_lights: Vec<DirectionalLight>,
    /// The ambient light, linear: added everywhere by the mesh renderer (C#: `AmbientColor`,
    /// default 25% grey).
    pub ambient_color: Color4,
    /// The colour of the lower hemisphere of the model renderer's hemispheric ambient, or None for
    /// the ambient colour (C#: `AmbientGroundColor`, default None).
    pub ambient_ground_color: Option<Color4>,
    /// The direction the sky hemisphere is centred on (C#: `AmbientUp`, default +Y).
    pub ambient_up: Vec3,
    /// Scales everything the model renderer outputs (C#: `Exposure`, default 1).
    pub exposure: f32,
    /// Where the model renderer's ambient light comes from (C#: `Ambient`, default Environment).
    pub ambient: AmbientLighting,
    /// The environment for image-based lighting (the model renderer, with `ambient` =
    /// Environment) and for [`crate::SkyboxRenderer::draw`]; None = the context's
    /// [`crate::GraphicsContext::default_environment`], a procedural sky (C#: `Environment`,
    /// default null).
    pub environment: Option<EnvironmentMap>,
    /// Scales the environment's light and the skybox (C#: `EnvironmentIntensity`, default 1).
    pub environment_intensity: f32,
    /// Turns the environment about +Y, in radians (C#: `EnvironmentRotation`, default 0).
    pub environment_rotation: f32,
    /// Point lights and spotlights at places in the scene, for the model renderer (Not in C#;
    /// default empty). At most [`SceneLighting::MAX_LOCAL_LIGHTS`] are shaded in a frame: over
    /// that, the renderer uses those that matter most to the view
    /// ([`LocalLight::importance`]: brightness over the squared distance from the camera, among
    /// the lights whose reach gets into the view) and skips the rest for that frame. Read at
    /// every `begin`, so they can move from frame to frame. A lighting without them costs nothing.
    pub local_lights: Vec<LocalLight>,
    /// The shadow of the key light (the sun) in the model renderer (Not in C#; default OFF).
    pub sun_shadows: SunShadowSettings,
    /// The settings of the spotlights' shadows in the model renderer (Not in C#); a spotlight
    /// casts only with [`crate::SpotLight::casts_shadows`].
    pub spot_shadows: SpotShadowSettings,
}

impl SceneLighting {
    /// The most directional lights the model renderer shades: the key light and seven more (C#:
    /// `MaxDirectionalLights`).
    pub const MAX_DIRECTIONAL_LIGHTS: usize = 8;

    /// The most local lights (point lights and spotlights together) the model renderer shades in a
    /// frame: a fixed ceiling, not a setting (Not in C#).
    pub const MAX_LOCAL_LIGHTS: usize = 16;

    /// The most spotlights that cast shadows in a frame, besides the sun (Not in C#).
    pub const MAX_SHADOWED_SPOTLIGHTS: usize = 4;

    /// Lighting with the C# defaults (C#: `new SceneLighting()`).
    pub fn new() -> SceneLighting {
        SceneLighting::default()
    }
}

impl Default for SceneLighting {
    fn default() -> SceneLighting {
        SceneLighting {
            light_direction: Vec3::new(-0.4, -1.0, -0.5).normalize(),
            light_color: Color4::WHITE,
            light_intensity: 1.0,
            additional_lights: Vec::new(),
            ambient_color: Color4::new(0.25, 0.25, 0.25, 1.0),
            ambient_ground_color: None,
            ambient_up: Vec3::Y,
            exposure: 1.0,
            ambient: AmbientLighting::Environment,
            environment: None,
            environment_intensity: 1.0,
            environment_rotation: 0.0,
            local_lights: Vec::new(),
            sun_shadows: SunShadowSettings::default(),
            spot_shadows: SpotShadowSettings::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{AmbientLighting, SceneLighting};

    #[test]
    fn the_defaults_are_the_csharp_ones() {
        let lighting = SceneLighting::new();
        assert!(lighting.light_direction.distance(Vec3::new(-0.4, -1.0, -0.5).normalize()) < 1e-6);
        assert_eq!(lighting.light_intensity, 1.0);
        assert_eq!(lighting.ambient_color.r, 0.25);
        assert_eq!(lighting.ambient, AmbientLighting::Environment);
        assert!(lighting.environment.is_none());
        assert_eq!(SceneLighting::MAX_DIRECTIONAL_LIGHTS, 8);
        // Not in C#: no local lights, no shadows; the ceilings.
        assert!(lighting.local_lights.is_empty());
        assert!(!lighting.sun_shadows.enabled);
        assert_eq!((SceneLighting::MAX_LOCAL_LIGHTS, SceneLighting::MAX_SHADOWED_SPOTLIGHTS), (16, 4));
    }
}
