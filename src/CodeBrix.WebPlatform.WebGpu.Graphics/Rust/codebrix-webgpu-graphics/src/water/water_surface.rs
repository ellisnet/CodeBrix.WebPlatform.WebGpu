//! [`WaterSurface`]: one rectangle of water for a 3D scene - where it is, its grid, the wave field
//! and the open-water waves that shape it, its fine ripples and its look - with the mapping
//! between the world and a wave field's units for stirring ([`WaterSurface::field_point`]) and
//! the CPU passthroughs that float things ([`WaterSurface::height_at`], [`WaterSurface::normal_at`])
//! (Not in C#). [`WaterField`] hands a [`WaveField`]'s picture to the surface.

use glam::{Vec2, Vec3};

use crate::drawing3d::Ray;
use crate::water::water_look::{WaterDetail, WaterLook};
use crate::water::water_math::rotate_y;
use crate::water::{OpenWaterWaves, WaveField};

/// A [`WaveField`]'s picture laid over a whole [`WaterSurface`] (Not in C#): the field's column 0,
/// row 0 at the surface's corner of least local X and Z, its columns along local X, its rows along
/// local Z, stretched to the surface's size.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterField {
    /// The field's picture (`WaveField::surface().view().clone()`; a view compares by identity).
    pub view: wgpu::TextureView,
    /// The field's extent in field units (`WaveField::size()`): what [`WaterSurface::field_point`]
    /// maps to, and how the field's slopes are scaled onto the surface.
    pub size: Vec2,
    /// Multiplies the field's heights (and slopes) on the surface. 1
    pub height_scale: f32,
    /// Moves the grid's vertices by the heights too (else only the normals change). true
    pub displace: bool,
}

impl WaterField {
    /// The field's picture over the surface: its view and size, height scale 1, displacement on.
    pub fn of(field: &WaveField) -> WaterField {
        WaterField { view: field.surface().view().clone(), size: field.size(), height_scale: 1.0, displace: true }
    }
}

/// One rectangle of water in a 3D scene (Not in C#), drawn by a [`crate::WaterRenderer`]. It is a
/// SURFACE, not a fluid: a grid of `grid` x `grid` quads at the still water level, moved by a wave
/// field's heights and by open-water waves, its normals from both plus fine ripples, and every
/// pixel's colour worked out from the scene captured behind it (see the renderer for the look's
/// parts and the limits).
#[derive(Clone, Debug, PartialEq)]
pub struct WaterSurface {
    /// The still surface's centre; its y is the water level. ZERO
    pub center: Vec3,
    /// The extent along local X and Z (before rotation), scene units. (10, 10)
    pub size: Vec2,
    /// The rotation about +Y, radians. 0
    pub rotation: f32,
    /// Quads per side of the drawn grid, 1 to [`WaterSurface::MAX_GRID`] (1: one flat quad). 64
    pub grid: u32,
    /// A wave field over the whole surface. None
    pub field: Option<WaterField>,
    /// Open-water waves (in WORLD x and z, so neighbouring surfaces share them). None
    pub open_waves: Option<OpenWaterWaves>,
    /// Fine procedural ripples everywhere.
    pub detail: WaterDetail,
    /// How it looks.
    pub look: WaterLook,
}

impl Default for WaterSurface {
    fn default() -> WaterSurface {
        WaterSurface { center: Vec3::ZERO, size: Vec2::new(10.0, 10.0), rotation: 0.0, grid: 64, field: None, open_waves: None, detail: WaterDetail::default(), look: WaterLook::default() }
    }
}

impl WaterSurface {
    /// The most quads per side.
    pub const MAX_GRID: u32 = 512;

    /// The extent of the field units [`WaterSurface::field_point`] maps to: the field's, or the
    /// surface's own size without a field.
    fn field_size(&self) -> Vec2 {
        self.field.as_ref().map_or(self.size, |field| field.size)
    }

    /// Where a picking ray meets the STILL surface, in the field units of its [`WaterField`] (or
    /// of a field of the surface's own size without one), (0, 0) at the field's column 0, row 0 -
    /// for [`WaveField::stir`] and [`WaveField::drop`]; None when the ray misses the rectangle,
    /// runs parallel to it or meets it behind its origin. In a layer's `input`:
    /// `field.stir(input, &stir, &|x, y| surface.field_point(&camera.canvas_to_ray(x as f32, y as f32, w, h)))`.
    pub fn field_point(&self, ray: &Ray) -> Option<Vec2> {
        if ray.direction.y.abs() < 1e-8 {
            return None;
        }
        let t = (self.center.y - ray.origin.y) / ray.direction.y;
        if t.is_nan() || t < 0.0 {
            return None;
        }
        let hit = ray.origin + ray.direction * t;
        let local = rotate_y(hit - self.center, -self.rotation);
        let half = self.size * 0.5;
        if local.x.abs() > half.x || local.z.abs() > half.y {
            return None;
        }
        Some(self.to_field(hit))
    }

    /// A world point's place in field units (as [`WaterSurface::field_point`], without the bounds
    /// check; its height is ignored).
    pub fn to_field(&self, world: Vec3) -> Vec2 {
        let local = rotate_y(world - self.center, -self.rotation);
        let size = self.size.max(Vec2::splat(1e-6));
        (Vec2::new(local.x, local.z) + size * 0.5) / size * self.field_size()
    }

    /// A field point's place in the world, on the still surface.
    pub fn to_world(&self, field: Vec2) -> Vec3 {
        let local = field / self.field_size().max(Vec2::splat(1e-6)) * self.size - self.size * 0.5;
        self.center + rotate_y(Vec3::new(local.x, 0.0, local.y), self.rotation)
    }

    /// The surface's height above the world point (x, z) at `seconds` (`frame.info.time`): the
    /// water level plus the open-water waves' height there - to float things. The WAVE FIELD's
    /// heights are not included: they live on the GPU (read its picture back if you need them).
    pub fn height_at(&self, x: f32, z: f32, seconds: f64) -> f32 {
        self.center.y + self.open_waves.as_ref().map_or(0.0, |waves| waves.height_at(x, z, seconds))
    }

    /// The surface's unit normal above the world point (x, z) at `seconds`, from the open-water
    /// waves (+Y without them; the wave field is not included) - to tilt a floating thing.
    pub fn normal_at(&self, x: f32, z: f32, seconds: f64) -> Vec3 {
        self.open_waves.as_ref().map_or(Vec3::Y, |waves| waves.normal_at(x, z, seconds))
    }

    /// One quad's side in scene units (the larger of the two), what decides which open-water
    /// waves move the vertices.
    pub fn grid_cell(&self) -> f32 {
        self.size.max_element() / self.grid.max(1) as f32
    }
}
