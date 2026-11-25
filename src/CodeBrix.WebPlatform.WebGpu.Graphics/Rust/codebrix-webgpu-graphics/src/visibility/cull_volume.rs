//! [`CullVolume`]: a convex volume as a set of planes, and the test of a (transformed) box against
//! it (Not in C#). [`ModelRenderer`](crate::ModelRenderer) culls its instances with the volume of
//! the camera's view; the same test serves any other volume given by a view-projection matrix (a
//! light's shadow camera) or by planes.

use glam::{Mat4, Vec3, Vec4};

use crate::drawing3d::Camera;

/// A convex volume: the points on the inner side of every one of its planes (Not in C#). Each
/// plane is `(normal, d)` in a [`Vec4`], normalised, and a point `p` is inside it when
/// `normal.dot(p) + d >= 0`.
///
/// Made from a camera ([`CullVolume::from_camera`]: the six planes of its view, near and far
/// included), from any WebGPU view-projection matrix ([`CullVolume::from_view_projection`]: a
/// perspective one gives a frustum, an orthographic one a box - a spotlight's or the sun's shadow
/// camera), or from planes ([`CullVolume::from_planes`]).
///
/// THE TEST IS CONSERVATIVE: [`CullVolume::intersects_transformed_box`] is false only when the box
/// is certainly entirely outside (outside one plane). A box near a corner of the volume, outside
/// two planes at once but not wholly outside either, counts as inside - it is drawn, never wrongly
/// left out.
#[derive(Clone, Debug, PartialEq)]
pub struct CullVolume {
    planes: Vec<Vec4>,
}

impl CullVolume {
    /// A volume from planes `(normal, d)` (inside where `normal.dot(p) + d >= 0`); each is
    /// normalised, and a plane without a finite, non-zero normal is left out. No planes: a volume
    /// that holds everything.
    pub fn from_planes(planes: impl IntoIterator<Item = Vec4>) -> CullVolume {
        let planes = planes
            .into_iter()
            .filter_map(|plane| {
                let length = plane.truncate().length();
                (length.is_finite() && length > 0.0 && plane.w.is_finite()).then(|| plane / length)
            })
            .collect();
        CullVolume { planes }
    }

    /// The volume a WebGPU view-projection matrix shows: the points whose clip coordinates satisfy
    /// `-w <= x <= w`, `-w <= y <= w` and `0 <= z <= w` (left, right, bottom, top, near, far, in
    /// that order). A perspective matrix gives a frustum, an orthographic one a box.
    pub fn from_view_projection(view_projection: Mat4) -> CullVolume {
        let (x, y, z, w) = (view_projection.row(0), view_projection.row(1), view_projection.row(2), view_projection.row(3));
        CullVolume::from_planes([w + x, w - x, w + y, w - y, z, w - z])
    }

    /// The volume a camera shows on a target of this aspect ratio (width / height) - the same
    /// matrix [`ModelRenderer::begin`](crate::ModelRenderer::begin) draws with.
    pub fn from_camera(camera: &Camera, aspect_ratio: f32) -> CullVolume {
        CullVolume::from_view_projection(camera.get_view_projection_matrix(aspect_ratio))
    }

    /// The planes, normalised.
    pub fn planes(&self) -> &[Vec4] {
        &self.planes
    }

    /// True when the point is inside (or on) every plane.
    pub fn contains_point(&self, point: Vec3) -> bool {
        self.planes.iter().all(|plane| plane.truncate().dot(point) + plane.w >= 0.0)
    }

    /// The test of an axis-aligned box: false only when it is certainly entirely outside.
    pub fn intersects_box(&self, min: Vec3, max: Vec3) -> bool {
        self.intersects_transformed_box(min, max, Mat4::IDENTITY)
    }

    /// The test of a box `min..max` in its own space, placed by `transform` (a model's bounds and
    /// an instance's world matrix): false only when the placed box is certainly entirely outside
    /// the volume - wholly on the outer side of one plane. Exact per plane for any affine
    /// transform (rotated, scaled unevenly, mirrored). A box whose `min` is above its `max` on an
    /// axis is empty: always false. A transform that is not finite: always true (never culled).
    pub fn intersects_transformed_box(&self, min: Vec3, max: Vec3, transform: Mat4) -> bool {
        if !min.cmple(max).all() {
            return false;
        }
        let half = (max - min) * 0.5;
        let center = transform.transform_point3((min + max) * 0.5);
        let axes = [transform.x_axis.truncate() * half.x, transform.y_axis.truncate() * half.y, transform.z_axis.truncate() * half.z];
        for plane in &self.planes {
            let normal = plane.truncate();
            let distance = normal.dot(center) + plane.w;
            let reach = axes[0].dot(normal).abs() + axes[1].dot(normal).abs() + axes[2].dot(normal).abs();
            // NaN compares false: a box with a non-finite place is never culled.
            if distance + reach < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3, Vec4};

    use super::CullVolume;
    use crate::drawing3d::{Camera, ProjectionKind};

    /// At z = 10 looking down -z at the origin, 60 degrees vertical field of view, near 1, far 100.
    fn camera() -> Camera {
        Camera { position: Vec3::new(0.0, 0.0, 10.0), target: Vec3::ZERO, field_of_view: 60f32.to_radians(), near_plane: 1.0, far_plane: 100.0, ..Camera::default() }
    }

    fn unit_box_at(volume: &CullVolume, at: Vec3) -> bool {
        volume.intersects_transformed_box(Vec3::splat(-0.5), Vec3::splat(0.5), Mat4::from_translation(at))
    }

    #[test]
    fn a_camera_s_view_keeps_what_it_sees_and_drops_what_is_outside() {
        let volume = CullVolume::from_camera(&camera(), 1.0);
        assert_eq!(volume.planes().len(), 6);
        assert!(unit_box_at(&volume, Vec3::ZERO));
        // Behind the camera, beyond the far plane, far to the side, above.
        assert!(!unit_box_at(&volume, Vec3::new(0.0, 0.0, 12.0)));
        assert!(!unit_box_at(&volume, Vec3::new(0.0, 0.0, -95.0)));
        assert!(!unit_box_at(&volume, Vec3::new(20.0, 0.0, 0.0)));
        assert!(!unit_box_at(&volume, Vec3::new(0.0, 20.0, 0.0)));
        // At 10 m the half-height of the view is tan(30 deg) * 10 = 5.77: a box at x = 6 still
        // reaches into it (its near side is at 5.5); one at x = 7 does not (its near side, 6.5,
        // is outside even at its far face, 10.5 m away, where the half-width is 6.06).
        assert!(unit_box_at(&volume, Vec3::new(6.0, 0.0, 0.0)));
        assert!(!unit_box_at(&volume, Vec3::new(7.0, 0.0, 0.0)));
        // Straddling the near plane.
        assert!(unit_box_at(&volume, Vec3::new(0.0, 0.0, 9.2)));
        assert!(volume.contains_point(Vec3::ZERO) && !volume.contains_point(Vec3::new(0.0, 0.0, 11.0)));
    }

    #[test]
    fn the_aspect_ratio_widens_the_view() {
        let narrow = CullVolume::from_camera(&camera(), 1.0);
        let wide = CullVolume::from_camera(&camera(), 2.0);
        let at = Vec3::new(9.0, 0.0, 0.0);
        assert!(!unit_box_at(&narrow, at));
        assert!(unit_box_at(&wide, at));
    }

    #[test]
    fn a_transformed_box_is_tested_where_it_is_placed() {
        let volume = CullVolume::from_camera(&camera(), 1.0);
        // A long thin box along x, centred at x = 10 (outside), rotated a quarter turn about z so
        // that it stands upright at x = 10: still outside. Scaled 10x along its length it reaches
        // back into the view.
        let (min, max) = (Vec3::new(-1.0, -0.1, -0.1), Vec3::new(1.0, 0.1, 0.1));
        let upright = Mat4::from_rotation_translation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Vec3::new(10.0, 0.0, 0.0));
        assert!(!volume.intersects_transformed_box(min, max, upright));
        let long = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)) * Mat4::from_scale(Vec3::new(10.0, 1.0, 1.0));
        assert!(volume.intersects_transformed_box(min, max, long));
        // A mirroring transform is no different.
        let mirrored = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) * Mat4::from_translation(Vec3::new(-20.0, 0.0, 0.0));
        assert!(!volume.intersects_transformed_box(min, max, mirrored));
        assert!(volume.intersects_transformed_box(min, max, Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))));
    }

    #[test]
    fn empty_boxes_are_outside_and_non_finite_places_are_never_culled() {
        let volume = CullVolume::from_camera(&camera(), 1.0);
        assert!(!volume.intersects_box(Vec3::ONE, Vec3::ZERO));
        assert!(volume.intersects_transformed_box(Vec3::ZERO, Vec3::ONE, Mat4::from_translation(Vec3::splat(f32::NAN))));
        // A degenerate box (a point) inside is inside.
        assert!(volume.intersects_box(Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn an_orthographic_matrix_gives_a_box_such_as_a_light_s_volume() {
        let light = Camera { projection: ProjectionKind::Orthographic, orthographic_height: 4.0, ..camera() };
        let volume = CullVolume::from_camera(&light, 1.0);
        // Half-width and half-height 2; near 1 to far 100 in front of z = 10.
        assert!(unit_box_at(&volume, Vec3::new(2.3, 0.0, 0.0)));
        assert!(!unit_box_at(&volume, Vec3::new(2.6, 0.0, 0.0)));
        assert!(!unit_box_at(&volume, Vec3::new(0.0, 0.0, -91.0)));
        // The same as planes given by hand: x in -2..2 only.
        let slab = CullVolume::from_planes([Vec4::new(1.0, 0.0, 0.0, 2.0), Vec4::new(-2.0, 0.0, 0.0, 4.0), Vec4::ZERO]);
        assert_eq!(slab.planes().len(), 2);
        assert!(unit_box_at(&slab, Vec3::new(2.3, 50.0, 0.0)) && !unit_box_at(&slab, Vec3::new(-2.6, 0.0, 0.0)));
        assert!(CullVolume::from_planes([]).intersects_box(Vec3::splat(1e9), Vec3::splat(1e9 + 1.0)));
    }
}
