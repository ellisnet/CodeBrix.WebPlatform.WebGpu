//! [`Camera`] and [`ProjectionKind`]: a right-handed camera with a perspective or orthographic
//! projection into WebGPU's depth range 0..1 (C#: `Camera`, `ProjectionKind`); with
//! [`CanvasProjection`], [`Camera::world_to_canvas`], [`Camera::canvas_to_ray`] and
//! [`Camera::fit_distance`] (not in C#).

use glam::{Mat4, Vec3, Vec4};

use crate::drawing3d::Ray;

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

    /// Projects a world position onto a canvas of `canvas_width` x `canvas_height` CSS pixels -
    /// for page content (a text label, a button) placed over the scene - and says whether the
    /// point is in front of the camera and inside the view. Not in C# (C#'s `WorldToScreen`,
    /// [`Camera::world_to_screen`], gives only the position and the depth).
    ///
    /// The canvas size is the one the scene is drawn for (its aspect ratio is the projection's):
    /// in a layer, `frame.info.width as f32 / frame.info.pixel_ratio as f32` and the same for
    /// the height; a size of 0 counts as 1. The result's `x` and `y` are CSS pixels from the
    /// canvas's top-left corner (y down): add the canvas's own place on the page to position an
    /// element absolutely, or put the element in a box laid over the canvas.
    pub fn world_to_canvas(&self, world: Vec3, canvas_width: f32, canvas_height: f32) -> CanvasProjection {
        let (width, height) = (positive_or_one(canvas_width), positive_or_one(canvas_height));
        let view = self.view_matrix().transform_point3(world);
        let clip = self.get_projection_matrix(width / height) * Vec4::new(view.x, view.y, view.z, 1.0);
        let ndc = clip.truncate() / clip.w;
        let view_distance = -view.z;
        let in_front = view_distance > 0.0;
        let inside = |v: f32| (-1.0..=1.0).contains(&v);
        CanvasProjection {
            x: (ndc.x + 1.0) * 0.5 * width,
            y: (1.0 - ndc.y) * 0.5 * height,
            depth: ndc.z,
            view_distance,
            in_front,
            in_view: in_front && inside(ndc.x) && inside(ndc.y) && (0.0..=1.0).contains(&ndc.z),
        }
    }

    /// The ray a point of the canvas sees: from the camera through the point `(x, y)`, in CSS
    /// pixels from the top-left corner of a canvas of `canvas_width` x `canvas_height` CSS pixels
    /// (the inverse of [`Camera::world_to_canvas`], for picking). `x` and `y` are the canvas
    /// input's own (`CanvasInput::PointerDown { x, y, .. }`, as `f32`). Not in C#.
    ///
    /// A perspective camera's ray starts AT the camera's position and spreads with the field of
    /// view; an orthographic camera's rays are parallel to its view direction and start in the
    /// plane through the camera's position across that direction. Either way the direction is a
    /// unit vector and a hit's distance ([`Ray::intersect_box`]) is in world units from the
    /// camera (a hit closer than the near plane is not drawn: compare with `near_plane` if that
    /// matters). A degenerate camera (see [`Camera::view_matrix`]) gives the ray of the identity
    /// view, which is what it draws.
    pub fn canvas_to_ray(&self, x: f32, y: f32, canvas_width: f32, canvas_height: f32) -> Ray {
        let (width, height) = (positive_or_one(canvas_width), positive_or_one(canvas_height));
        let aspect = width / height;
        let ndc_x = 2.0 * x / width - 1.0;
        let ndc_y = 1.0 - 2.0 * y / height;
        // In view space (looking down -z), matching get_projection_matrix exactly.
        let (origin, direction) = match self.projection {
            ProjectionKind::Perspective => {
                let tan_half = (self.field_of_view * 0.5).tan();
                (Vec3::ZERO, Vec3::new(ndc_x * tan_half * aspect, ndc_y * tan_half, -1.0))
            }
            ProjectionKind::Orthographic => {
                let half_height = self.orthographic_height / 2.0;
                (Vec3::new(ndc_x * half_height * aspect, ndc_y * half_height, 0.0), Vec3::NEG_Z)
            }
        };
        let to_world = self.view_matrix().inverse();
        Ray::new(to_world.transform_point3(origin), to_world.transform_vector3(direction))
    }

    /// The distance from a sphere's centre at which this camera sees the whole sphere of
    /// `radius` on a view of `aspect_ratio` (width / height; 0 or less counts as 1) - the
    /// distance that frames a model of that bounding radius from ANY direction. Not in C#.
    ///
    /// Perspective: the sphere just touches the narrower of the vertical and the horizontal
    /// field of view, `radius / sin(half angle)` (twice the radius for the default 60 degrees on
    /// a landscape view), and at least `radius + near_plane` so the near plane never cuts it.
    /// Orthographic: the size on screen does not depend on the distance (it is
    /// `orthographic_height`'s: [`Camera::fit_orthographic_height`]); the distance is
    /// `radius + near_plane`, which keeps the sphere in front of the near plane. Keep
    /// `far_plane` beyond the distance plus the radius.
    pub fn fit_distance(&self, radius: f32, aspect_ratio: f32) -> f32 {
        let radius = radius.abs();
        let in_front = radius + self.near_plane;
        match self.projection {
            ProjectionKind::Perspective => {
                let aspect = if aspect_ratio > 0.0 { aspect_ratio } else { 1.0 };
                let half_vertical = self.field_of_view * 0.5;
                let half_horizontal = (half_vertical.tan() * aspect).atan();
                let sine = half_vertical.min(half_horizontal).sin();
                if sine > f32::EPSILON && sine.is_finite() {
                    (radius / sine).max(in_front)
                } else {
                    in_front
                }
            }
            ProjectionKind::Orthographic => in_front,
        }
    }

    /// The `orthographic_height` at which an orthographic view of `aspect_ratio` (width /
    /// height; 0 or less counts as 1) shows the whole sphere of `radius`: `2 * radius`, divided
    /// by the aspect ratio when the view is narrower than it is high. Not in C#.
    pub fn fit_orthographic_height(&self, radius: f32, aspect_ratio: f32) -> f32 {
        let aspect = if aspect_ratio > 0.0 { aspect_ratio } else { 1.0 };
        2.0 * radius.abs() / aspect.min(1.0)
    }
}

/// Where a world position lands on the canvas ([`Camera::world_to_canvas`]). Not in C#.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasProjection {
    /// CSS pixels from the canvas's left edge (meaningful only when `in_front`; outside
    /// 0..width when the point is left or right of the view). Not in C#.
    pub x: f32,
    /// CSS pixels from the canvas's top edge, y down (meaningful only when `in_front`). Not in
    /// C#.
    pub y: f32,
    /// The depth the point would have in the depth buffer: 0 at the near plane, 1 at the far
    /// plane, outside 0..1 when the planes clip it. Not in C#.
    pub depth: f32,
    /// How far in front of the camera the point is, along the view direction, in world units
    /// (negative behind the camera): sort labels by it, or scale them with it. Not in C#.
    pub view_distance: f32,
    /// True when the point is in front of the camera (`view_distance > 0`); a point behind the
    /// camera has no place on the canvas - hide its label. Not in C#.
    pub in_front: bool,
    /// True when the point is drawn: in front, between the near and the far plane, and inside
    /// the canvas (its edges included). It says nothing about other things in front of it (no
    /// depth test). Not in C#.
    pub in_view: bool,
}

/// A canvas dimension, 1 when it is not above 0 (or not a number).
fn positive_or_one(value: f32) -> f32 {
    if value > 0.0 {
        value
    } else {
        1.0
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

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn a_seen_camera(projection: ProjectionKind) -> Camera {
        Camera { position: Vec3::new(3.0, 4.0, 8.0), target: Vec3::new(0.5, 0.0, -1.0), projection, near_plane: 0.5, far_plane: 50.0, ..Camera::default() }
    }

    #[test]
    fn world_to_canvas_agrees_with_world_to_screen_in_css_pixels() {
        let camera = a_seen_camera(ProjectionKind::Perspective);
        let point = Vec3::new(1.0, 0.5, 0.0);
        let screen = camera.world_to_screen(point, 800, 400);
        let canvas = camera.world_to_canvas(point, 800.0, 400.0);
        assert!(close(canvas.x, screen.x) && close(canvas.y, screen.y) && close(canvas.depth, screen.z));
        assert!(canvas.in_front && canvas.in_view);
        let target = camera.world_to_canvas(camera.target, 800.0, 400.0);
        assert!(close(target.x, 400.0) && close(target.y, 200.0));
        assert!(close(target.view_distance, (camera.target - camera.position).length()));
        // A canvas of 0 x 0 counts as 1 x 1 (no division by zero).
        let tiny = camera.world_to_canvas(camera.target, 0.0, 0.0);
        assert!(close(tiny.x, 0.5) && close(tiny.y, 0.5));
    }

    #[test]
    fn world_to_canvas_says_behind_outside_and_clipped() {
        let camera = Camera { near_plane: 1.0, far_plane: 20.0, ..Camera::default() }; // at z = 5, looking down -z
        let behind = camera.world_to_canvas(Vec3::new(0.0, 0.0, 9.0), 400.0, 300.0);
        assert!(!behind.in_front && !behind.in_view);
        assert!(close(behind.view_distance, -4.0));
        let left = camera.world_to_canvas(Vec3::new(-50.0, 0.0, 0.0), 400.0, 300.0);
        assert!(left.in_front && !left.in_view && left.x < 0.0);
        let too_near = camera.world_to_canvas(Vec3::new(0.0, 0.0, 4.5), 400.0, 300.0);
        assert!(too_near.in_front && !too_near.in_view && too_near.depth < 0.0, "closer than the near plane");
        let too_far = camera.world_to_canvas(Vec3::new(0.0, 0.0, -30.0), 400.0, 300.0);
        assert!(too_far.in_front && !too_far.in_view && too_far.depth > 1.0);
        assert!(camera.world_to_canvas(Vec3::ZERO, 400.0, 300.0).in_view);
    }

    #[test]
    fn canvas_to_ray_goes_back_through_the_projected_point() {
        for projection in [ProjectionKind::Perspective, ProjectionKind::Orthographic] {
            let camera = a_seen_camera(projection);
            for point in [Vec3::new(1.0, 0.5, 0.0), Vec3::new(-2.0, 1.5, -3.0), camera.target] {
                let projected = camera.world_to_canvas(point, 640.0, 360.0);
                let ray = camera.canvas_to_ray(projected.x, projected.y, 640.0, 360.0);
                assert!((ray.direction.length() - 1.0).abs() < 1e-5);
                // The point lies on the ray, at its distance along it.
                let along = (point - ray.origin).dot(ray.direction);
                assert!(along > 0.0, "{projection:?}");
                assert!(ray.at(along).distance(point) < 1e-3, "{projection:?} {point}");
            }
        }
        // The perspective ray starts at the camera; the middle of the canvas looks at the target.
        let camera = a_seen_camera(ProjectionKind::Perspective);
        let middle = camera.canvas_to_ray(320.0, 180.0, 640.0, 360.0);
        assert!(middle.origin.distance(camera.position) < 1e-4);
        assert!(middle.direction.distance(camera.forward()) < 1e-5);
        // Orthographic rays are parallel to the view direction and start in the camera's plane.
        let ortho = a_seen_camera(ProjectionKind::Orthographic);
        let corner = ortho.canvas_to_ray(0.0, 0.0, 640.0, 360.0);
        assert!(corner.direction.distance(ortho.forward()) < 1e-5);
        assert!((corner.origin - ortho.position).dot(ortho.forward()).abs() < 1e-4);
        assert!(corner.origin.distance(ortho.position) > 1.0);
    }

    #[test]
    fn a_ray_from_the_canvas_picks_the_box_under_the_pointer() {
        let camera = Camera::default(); // at z = 5 looking at the origin
        let unit = (Vec3::splat(-0.5), Vec3::splat(0.5));
        let left = glam::Mat4::from_translation(Vec3::new(-1.5, 0.0, 0.0));
        let right = glam::Mat4::from_translation(Vec3::new(1.5, 0.0, 0.0));
        let over_right = camera.world_to_canvas(Vec3::new(1.5, 0.0, 0.5), 500.0, 500.0);
        let ray = camera.canvas_to_ray(over_right.x, over_right.y, 500.0, 500.0);
        let (index, distance) = ray.nearest_box_hit([(unit.0, unit.1, left), (unit.0, unit.1, right)]).unwrap();
        assert_eq!(index, 1);
        assert!(close(distance, Vec3::new(1.5, 0.0, 0.5).distance(camera.position)));
        let between = camera.canvas_to_ray(250.0, 250.0, 500.0, 500.0);
        assert_eq!(between.nearest_box_hit([(unit.0, unit.1, left), (unit.0, unit.1, right)]), None);
    }

    #[test]
    fn fit_distance_frames_a_sphere_in_the_narrower_field_of_view() {
        let camera = Camera::default(); // 60 degrees vertically
        assert!(close(camera.fit_distance(1.0, 2.0), 2.0), "landscape: the vertical angle decides");
        assert!(close(camera.fit_distance(1.0, 1.0), 2.0));
        // Portrait 1:2: the horizontal half angle is atan(tan 30 / 2).
        let half = ((std::f32::consts::PI / 6.0).tan() * 0.5).atan();
        assert!(close(camera.fit_distance(1.0, 0.5), 1.0 / half.sin()));
        assert!(close(camera.fit_distance(0.0, 1.0), camera.near_plane), "never closer than the near plane");
        // At the fitted distance, the sphere's top touches the canvas's top edge.
        let fitted = Camera { position: Vec3::new(0.0, 0.0, camera.fit_distance(1.0, 1.0)), ..camera };
        let top = Vec3::new(0.0, 1.0 / (std::f32::consts::PI / 6.0).cos(), 0.0); // where the tangent line crosses y
        assert!(close(fitted.world_to_canvas(top, 100.0, 100.0).y, 0.0));
        let ortho = Camera { projection: ProjectionKind::Orthographic, ..camera };
        assert!(close(ortho.fit_distance(2.0, 1.0), 2.0 + ortho.near_plane));
    }

    #[test]
    fn a_degenerate_camera_has_the_identity_view() {
        let camera = Camera { position: Vec3::ZERO, ..Camera::default() };
        assert_eq!(camera.view_matrix(), glam::Mat4::IDENTITY);
        let straight_down = Camera { position: Vec3::new(0.0, 5.0, 0.0), ..Camera::default() };
        assert_eq!(straight_down.view_matrix(), glam::Mat4::IDENTITY);
    }
}
