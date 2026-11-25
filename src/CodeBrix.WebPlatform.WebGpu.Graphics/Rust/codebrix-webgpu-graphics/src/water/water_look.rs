//! How a [`crate::WaterSurface`] looks (Not in C#): [`WaterLook`] - absorption and scattering by
//! the real depth of water, refraction, Fresnel reflection, sun glints, the screen-space
//! reflection switch, highlights of local lights, the sun's shadow, the soft shore - with
//! [`WaterCaustics`] (the net of light on the bottom), [`WaterDetail`] (fine travelling ripples
//! everywhere, a setting of the surface) and [`WaterSheen`] (an optional thin-film rainbow sheen, off by default). Every
//! default is a first choice for the owner's eye.

use glam::Vec3;

/// The caustic net of light on the bottom seen through the water (Not in C#): a moving net of
/// bright cell edges at the point where the sun's light entered the water above the bottom,
/// strengthened where a wave field focuses light (under crests), fading with depth and with how
/// the bottom faces the sun, and darkened in the sun's shadow. It brightens the bottom's OWN lit
/// colour (`bottom *= 1 + strength * net`), so a dark or shadowed bottom stays dark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterCaustics {
    /// How much the net brightens the bottom (0: none). 0.7
    pub strength: f32,
    /// Net cells per scene unit. 1.4
    pub scale: f32,
    /// How fast the net moves (cycles of its motion per second). 0.35
    pub speed: f32,
    /// How much a wave field's curvature focuses the net (its Laplacian: brighter under crests,
    /// darker under troughs; 0 ignores the field). 0.5
    pub focus: f32,
    /// The depth of water (scene units, straight down) over which the net fades to about a third
    /// (`exp(-depth / depth_fade)`). 4
    pub depth_fade: f32,
}

impl Default for WaterCaustics {
    fn default() -> WaterCaustics {
        WaterCaustics { strength: 0.7, scale: 1.4, speed: 0.35, focus: 0.5, depth_fade: 4.0 }
    }
}

/// Fine procedural ripples over the whole surface (Not in C#): four travelling sines in seeded
/// directions, an exact function of time, that only turn the normals (they never move the
/// vertices) and fade out where they are shorter than a few pixels (no shimmer far away).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterDetail {
    /// The steepest slope the ripples add (0: none). 0.06
    pub strength: f32,
    /// The longest ripple's wavelength in scene units (the others are shorter). 0.35
    pub wavelength: f32,
    /// How fast the ripples travel, scene units per second. 0.4
    pub speed: f32,
    /// Picks the directions (and the caustic net's pattern). 1
    pub seed: u32,
}

impl Default for WaterDetail {
    fn default() -> WaterDetail {
        WaterDetail { strength: 0.06, wavelength: 0.35, speed: 0.4, seed: 1 }
    }
}

/// An optional thin-film rainbow sheen on the surface (Not in C#; OFF by default: `strength` 0):
/// interference colours from a film whose thickness drifts over the water in broad bands and grows
/// with the waves' slope and height, so the colours shimmer where the water moves. A look, not
/// physics: it is added to the water's colour, scaled by how the scene is lit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterSheen {
    /// How strong the sheen is (0: off). 0
    pub strength: f32,
    /// Bands of film thickness per scene unit across the water. 0.35
    pub scale: f32,
    /// How fast the colours drift, film cycles per second. 0.05
    pub drift: f32,
    /// How much the slope thickens the film (colours change where the surface tilts). 4
    pub slope_gain: f32,
    /// How much the height above the still level thickens it, per scene unit. 6
    pub height_gain: f32,
}

impl Default for WaterSheen {
    fn default() -> WaterSheen {
        WaterSheen { strength: 0.0, scale: 0.35, drift: 0.05, slope_gain: 4.0, height_gain: 6.0 }
    }
}

/// How a water surface looks (Not in C#). Lengths are scene units, colours linear RGB.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterLook {
    /// Absorption per scene unit of path through the water, linear RGB: red fades first, so deep
    /// water loses its red and the bottom turns blue-green. (0.45, 0.09, 0.05)
    pub absorption: Vec3,
    /// The colour deep water turns (the light it scatters back), lit by the key light and the
    /// sky above. (0.01, 0.05, 0.08)
    pub scattering_color: Vec3,
    /// How fast that colour takes over, per unit of path. 0.12
    pub turbidity: f32,
    /// How far slopes bend the view of the bottom (times the path through the water, capped at 2
    /// units). 0.25
    pub refraction: f32,
    /// Scales the Fresnel reflection (Schlick, F0 0.02). 1
    pub reflectivity: f32,
    /// GGX roughness of the surface: the glints' size and how sharp reflections are. 0.06
    pub roughness: f32,
    /// Scales the key light's highlight (the sun's glints; HDR, so they bloom). 1
    pub sun_glint: f32,
    /// The caustic net on the bottom; None for none. Some(default)
    pub caustics: Option<WaterCaustics>,
    /// Reflect what is ON SCREEN by marching the captured depth (trees on the bank), with the
    /// environment behind; false reflects only the environment. true
    pub screen_reflections: bool,
    /// Highlights of the lighting's point lights and spotlights (at most 16, chosen as the model
    /// renderer chooses them; no spotlight shadows on the water). true
    pub local_light_highlights: bool,
    /// The glints and caustics go dark in the sun's shadow (needs [`crate::WaterFrame::sun_shadow`]).
    /// true
    pub receives_shadows: bool,
    /// Water thinner than this (path through it) fades to clear, so the shoreline has no hard
    /// edge. 0.12
    pub soft_shore: f32,
    /// The optional thin-film sheen (off by default).
    pub sheen: WaterSheen,
}

impl Default for WaterLook {
    fn default() -> WaterLook {
        WaterLook {
            absorption: Vec3::new(0.45, 0.09, 0.05),
            scattering_color: Vec3::new(0.01, 0.05, 0.08),
            turbidity: 0.12,
            refraction: 0.25,
            reflectivity: 1.0,
            roughness: 0.06,
            sun_glint: 1.0,
            caustics: Some(WaterCaustics::default()),
            screen_reflections: true,
            local_light_highlights: true,
            receives_shadows: true,
            soft_shore: 0.12,
            sheen: WaterSheen::default(),
        }
    }
}
