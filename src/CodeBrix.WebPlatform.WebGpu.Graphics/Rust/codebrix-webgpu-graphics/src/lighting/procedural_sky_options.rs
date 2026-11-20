//! [`ProceduralSkyOptions`]: the analytic sky of [`crate::GraphicsContext::create_procedural_sky`]
//! (C#: `ProceduralSkyOptions`).

use glam::Vec3;

use crate::common::Color4;

/// A simple analytic sky (C#: `ProceduralSkyOptions`; its properties are the public fields, with
/// the same defaults): a zenith-to-horizon gradient, a ground colour below the horizon, a haze
/// band and glow around the sun controlled by `turbidity`, and an optional sun disc. The colours
/// are linear radiance. The defaults are the sky [`crate::GraphicsContext::default_environment`]
/// uses: a clear day whose sun direction matches the default [`crate::SceneLighting`] key light.
///
/// The sun is usually lit by a directional light: keep `sun_intensity` at 0 and point the light
/// along `sun_direction`. To light a scene from the environment alone, switch the key light off
/// and give the sun an intensity instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProceduralSkyOptions {
    /// The direction the sunlight TRAVELS (from the sun into the scene), as
    /// [`crate::SceneLighting::light_direction`]. Default the default key light's,
    /// normalize(-0.4, -1, -0.5) (C#: `SunDirection`).
    pub sun_direction: Vec3,
    /// The sun's colour, linear. Default a warm white (1, 0.95, 0.86) (C#: `SunColor`).
    pub sun_color: Color4,
    /// The light the sun disc adds, in directional-light units. Default 0: the sun is a glow only
    /// and a directional light plays the sun (C#: `SunIntensity`).
    pub sun_intensity: f32,
    /// The sun disc's angular radius in radians (0.002 to 0.2). Default 0.0175 (C#:
    /// `SunAngularRadius`).
    pub sun_angular_radius: f32,
    /// The strength of the glow around the sun (0 = none). Default 1 (C#: `SunGlow`).
    pub sun_glow: f32,
    /// The sky colour straight up, linear. Default a clear blue (0.12, 0.24, 0.52) (C#:
    /// `ZenithColor`).
    pub zenith_color: Color4,
    /// The sky colour at the horizon, linear. Default a pale blue-white (0.56, 0.64, 0.74) (C#:
    /// `HorizonColor`).
    pub horizon_color: Color4,
    /// The colour below the horizon, linear. Default a dark warm grey (0.16, 0.14, 0.12) (C#:
    /// `GroundColor`).
    pub ground_color: Color4,
    /// Haze, 1 (very clear) to 10 (hazy). Default 2.5 (C#: `Turbidity`).
    pub turbidity: f32,
    /// Scales the whole sky (not the sun disc). Default 1 (C#: `Intensity`).
    pub intensity: f32,
    /// The direction that is "up" for the sky. Default +Y (C#: `Up`).
    pub up: Vec3,
}

impl Default for ProceduralSkyOptions {
    fn default() -> ProceduralSkyOptions {
        ProceduralSkyOptions {
            sun_direction: Vec3::new(-0.4, -1.0, -0.5).normalize(),
            sun_color: Color4::new(1.0, 0.95, 0.86, 1.0),
            sun_intensity: 0.0,
            sun_angular_radius: 0.0175,
            sun_glow: 1.0,
            zenith_color: Color4::new(0.12, 0.24, 0.52, 1.0),
            horizon_color: Color4::new(0.56, 0.64, 0.74, 1.0),
            ground_color: Color4::new(0.16, 0.14, 0.12, 1.0),
            turbidity: 2.5,
            intensity: 1.0,
            up: Vec3::Y,
        }
    }
}
