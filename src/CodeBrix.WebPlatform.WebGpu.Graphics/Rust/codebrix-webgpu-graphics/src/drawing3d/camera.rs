//! [`Camera`] and [`ProjectionKind`]: a right-handed camera with a perspective or orthographic
//! projection into WebGPU's depth range 0..1 (C#: `Camera`, `ProjectionKind`).

use glam::{Mat4, Vec3, Vec4};

/// How a [`Camera`] projects (C#: `ProjectionKind`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectionKind {
    /// Perspective: far things look smaller.
    #[default]
    Perspective,
    /// Orthographic: no foreshortening.
    Orthographic,
}

/// A right-handed camera (y up, looking down -z in view space) with a perspective or
/// orthographic projection whose depth range is WebGPU's 0..1 (C#: `Camera`; its properties are
/// the public fields, with the same defaults).
///
/// THE MATRIX ORDER: glam multiplies column vectors, so the products read right to left - C#'s
/// `world * View * Projection` is glam's `projection * view * world`, and
/// [`Camera::get_view_projection_matrix`] is `projection * view`. The bytes of the result are
/// those of the C# matrix, so the WGSL reads them unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Where the camera is (C#: `Position`, default (0, 0, 5)).
    pub position: Vec3,
    /// The point it looks at (C#: `Target`, default the origin).
    pub target: Vec3,
    /// The up direction (C#: `Up`, default +Y).
    pub up: Vec3,
    /// The projection (C#: `Projection`, default perspective).
    pub projection: ProjectionKind,
    /// The vertical field of view in radians, for a perspective projection (C#: `FieldOfView`,
    /// default 60 degrees).
    pub field_of_view: f32,
    /// The visible height in world units, for an orthographic projection (C#:
    /// `OrthographicHeight`, default 10).
    pub orthographic_height: f32,
    /// The near plane's distance (C#: `NearPlane`, default 0.1).
    pub near_plane: f32,
    /// The far plane's distance (C#: `FarPlane`, default 1000).
    pub far_plane: f32,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: ProjectionKind::Perspective,
            field_of_view: std::f32::consts::PI / 3.0,
            orthographic_height: 10.0,
            near_plane: 0.1,
            far_plane: 1000.0,
        }
    }
}

impl Camera {
    /// A camera with the C# defaults (C#: `new Camera()`).
    pub fn new() -> Camera {
        Camera::default()
    }

    /// The view matrix (C#: `ViewMatrix`, `Matrix4x4.CreateLookAt`). A camera whose position is
    /// its target, or whose view direction is parallel to `up`, has no view: the identity then.
    pub fn view_matrix(&self) -> Mat4 {
        let forward = self.target - self.position;
        let up = self.up.normalize_or_zero();
        if forward.length_squared() <= f32::EPSILON || up == Vec3::ZERO || forward.normalize().cross(up).length_squared() <= f32::EPSILON {
            return Mat4::IDENTITY;
        }
        glam::camera::rh::view::look_at_mat4(self.position, self.target, up)
    }

    /// The unit vector from the position towards the target (C#: `Forward`).
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }

    /// The projection matrix for an aspect ratio, width / height (C#: `GetProjectionMatrix`); an
    /// aspect ratio of 0 or less counts as 1.
    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let aspect = if aspect_ratio > 0.0 { aspect_ratio } else { 1.0 };
        match self.projection {
            ProjectionKind::Perspective => glam::camera::rh::proj::directx::perspective(self.field_of_view, aspect, self.near_plane, self.far_plane),
            ProjectionKind::Orthographic => {
                let half_height = self.orthographic_height / 2.0;
                let half_width = half_height * aspect;
                glam::camera::rh::proj::directx::orthographic(-half_width, half_width, -half_height, half_height, self.near_plane, self.far_plane)
            }
        }
    }

    /// The view and the projection in one matrix, `projection * view` in glam's order (C#:
    /// `GetViewProjectionMatrix`, `ViewMatrix * GetProjectionMatrix(aspect)` in C#'s).
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.get_projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Projects a world position to pixel coordinates (top-left origin, y down) and depth (C#:
    /// `WorldToScreen`): x and y in pixels of a `width` x `height` target; z the depth, outside
    /// 0..1 when the point is clipped.
    pub fn world_to_screen(&self, world: Vec3, width: u32, height: u32) -> Vec3 {
        let clip = self.get_view_projection_matrix(width as f32 / height.max(1) as f32) * Vec4::new(world.x, world.y, world.z, 1.0);
        let ndc = clip.truncate() / clip.w;
        Vec3::new((ndc.x + 1.0) * 0.5 * width as f32, (1.0 - ndc.y) * 0.5 * height as f32, ndc.z)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::{Camera, ProjectionKind};

    #[test]
    fn the_defaults_are_the_csharp_ones() {
        let camera = Camera::new();
        assert_eq!(camera.position, Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(camera.projection, ProjectionKind::Perspective);
        assert!((camera.field_of_view - std::f32::consts::FRAC_PI_3).abs() < 1e-6);
        assert_eq!((camera.near_plane, camera.far_plane, camera.orthographic_height), (0.1, 1000.0, 10.0));
        assert_eq!(camera.forward(), Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn world_to_screen_puts_the_target_in_the_middle_and_depth_in_0_to_1() {
        let camera = Camera { position: Vec3::new(0.0, 0.0, 5.0), target: Vec3::ZERO, near_plane: 1.0, far_plane: 100.0, ..Camera::default() };
        let center = camera.world_to_screen(Vec3::ZERO, 200, 100);
        let near = camera.world_to_screen(Vec3::new(0.0, 0.0, 3.999), 200, 100);
        let up = camera.world_to_screen(Vec3::new(0.0, 1.0, 0.0), 200, 100);
        assert!((center.x - 100.0).abs() < 1e-3 && (center.y - 50.0).abs() < 1e-3);
        assert!((0.0..=1.0).contains(&center.z));
        assert!(near.z < center.z && near.z >= -1e-4);
        assert!(up.y < 50.0);
    }

    #[test]
    fn orthographic_projection_keeps_sizes_regardless_of_distance() {
        let camera = Camera { projection: ProjectionKind::Orthographic, orthographic_height: 4.0, position: Vec3::new(0.0, 0.0, 10.0), ..Camera::default() };
        let near = camera.world_to_screen(Vec3::new(1.0, 1.0, 5.0), 100, 100);
        let far = camera.world_to_screen(Vec3::new(1.0, 1.0, -5.0), 100, 100);
        assert!((near.x - 75.0).abs() < 1e-3);
        assert!((far.x - 75.0).abs() < 1e-3);
        assert!((near.y - 25.0).abs() < 1e-3);
    }

    #[test]
    fn a_degenerate_camera_has_the_identity_view() {
        let camera = Camera { position: Vec3::ZERO, ..Camera::default() };
        assert_eq!(camera.view_matrix(), glam::Mat4::IDENTITY);
        let straight_down = Camera { position: Vec3::new(0.0, 5.0, 0.0), ..Camera::default() };
        assert_eq!(straight_down.view_matrix(), glam::Mat4::IDENTITY);
    }
}
