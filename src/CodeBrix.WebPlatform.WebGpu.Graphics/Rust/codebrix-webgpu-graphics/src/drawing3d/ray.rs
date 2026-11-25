//! [`Ray`]: a half-line in world space - what a canvas point sees through a camera
//! ([`Camera::canvas_to_ray`](crate::Camera::canvas_to_ray)) - and its tests against boxes, for
//! picking among instances by their bounds. Not in C# (the C# library has no picking).

use glam::{Mat4, Vec3};

/// A half-line: every point `origin + distance * direction` with `distance >= 0`. Not in C#.
///
/// [`Ray::new`] and [`Camera::canvas_to_ray`](crate::Camera::canvas_to_ray) make `direction` a
/// unit vector, so every distance a test returns is in WORLD UNITS from the origin. (The fields
/// are public; with a direction of another length the distances are in multiples of it.)
///
/// THE BOXES are given as their minimum and maximum corners - the form of
/// [`MeshData::get_bounds`](crate::MeshData::get_bounds) and of a model's
/// [`bounds_min`](crate::Model::bounds_min) / [`bounds_max`](crate::Model::bounds_max) - and,
/// for a box under a transform, the transform that places it (an instance's world matrix). To
/// pick among instances, test each instance's box under its world matrix and keep the nearest
/// hit: [`Ray::nearest_box_hit`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    /// Where the ray starts. Not in C#.
    pub origin: Vec3,
    /// Which way it goes (a unit vector when made by [`Ray::new`]). Not in C#.
    pub direction: Vec3,
}

impl Ray {
    /// A ray from `origin` in `direction`, which is normalized (a zero direction stays zero: such
    /// a ray hits nothing). Not in C#.
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction: direction.normalize_or_zero() }
    }

    /// The point `distance` along the ray: `origin + distance * direction`. Not in C#.
    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// Where the ray first meets the surface of the axis-aligned box from `min` to `max`, as a
    /// distance along the ray: where it ENTERS the box, or - when the ray starts inside the box -
    /// where it LEAVES it (the far side is the surface seen from inside, so a large box around
    /// the camera does not hide the boxes inside it). `None` when the ray misses the box, when
    /// the box lies behind the origin, when a corner is not finite or `min` exceeds `max` on an
    /// axis (an empty box), and for a ray with a zero direction. A flat box (`min == max` on an
    /// axis, such as the bounds of a plane) can be hit. A ray that grazes an edge or a face
    /// counts as a hit. Not in C#.
    pub fn intersect_box(&self, min: Vec3, max: Vec3) -> Option<f32> {
        slab_test(self.origin, self.direction, min, max)
    }

    /// Where the ray first meets the surface of the box from `min` to `max` placed by
    /// `transform` (for an instance: the model's bounds and the instance's world matrix), as a
    /// distance along THIS ray in world space - the same rule as [`Ray::intersect_box`], with
    /// the box rotated, scaled (also unevenly or mirrored), sheared and moved by the transform.
    /// The ray is taken into the box's own space by the inverse transform, so the box is tested
    /// exactly, not through a world-aligned box around it. `transform` must be affine (any
    /// world matrix is); `None` also when it cannot be inverted (a scale of 0) or is not
    /// finite. Not in C#.
    pub fn intersect_transformed_box(&self, min: Vec3, max: Vec3, transform: Mat4) -> Option<f32> {
        let determinant = transform.determinant();
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let inverse = transform.inverse();
        // The local direction is NOT normalized: a distance along it is then the same number as
        // the distance along the world ray.
        let origin = inverse.transform_point3(self.origin);
        let direction = inverse.transform_vector3(self.direction);
        if !origin.is_finite() || !direction.is_finite() {
            return None;
        }
        slab_test(origin, direction, min, max)
    }

    /// Picks among boxes: tests the ray against each `(min, max, transform)` with
    /// [`Ray::intersect_transformed_box`] and returns the index (in the order given) and the
    /// distance of the NEAREST hit, or `None` when it hits none. When two hits are at the same
    /// distance the first one given wins. Pass `Mat4::IDENTITY` for a box already in world
    /// space. Not in C#.
    pub fn nearest_box_hit<I>(&self, boxes: I) -> Option<(usize, f32)>
    where
        I: IntoIterator<Item = (Vec3, Vec3, Mat4)>,
    {
        let mut nearest: Option<(usize, f32)> = None;
        for (index, (min, max, transform)) in boxes.into_iter().enumerate() {
            if let Some(distance) = self.intersect_transformed_box(min, max, transform) {
                if nearest.is_none_or(|(_, best)| distance < best) {
                    nearest = Some((index, distance));
                }
            }
        }
        nearest
    }
}

/// The slab test of a ray (any direction length) against an axis-aligned box: the first surface
/// crossing at a distance of 0 or more, in multiples of `direction`.
fn slab_test(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    if !min.is_finite() || !max.is_finite() || min.cmpgt(max).any() || direction == Vec3::ZERO {
        return None;
    }
    let mut enter = f32::NEG_INFINITY;
    let mut leave = f32::INFINITY;
    for axis in 0..3 {
        let (o, d, lo, hi) = (origin[axis], direction[axis], min[axis], max[axis]);
        if d.abs() <= f32::EPSILON * f32::EPSILON {
            // Parallel to this axis's faces: inside the slab or never.
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut near, mut far) = ((lo - o) / d, (hi - o) / d);
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        enter = enter.max(near);
        leave = leave.min(far);
        if enter > leave {
            return None;
        }
    }
    if leave < 0.0 {
        return None;
    }
    Some(if enter >= 0.0 { enter } else { leave })
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3};

    use super::Ray;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn new_normalizes_the_direction_and_at_walks_along_it() {
        let ray = Ray::new(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 0.0, -10.0));
        assert_eq!(ray.direction, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(ray.at(4.0), Vec3::new(1.0, 2.0, -1.0));
        assert_eq!(Ray::new(Vec3::ZERO, Vec3::ZERO).direction, Vec3::ZERO);
    }

    #[test]
    fn a_box_ahead_is_hit_where_the_ray_enters_it() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::NEG_Z);
        let hit = ray.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)).unwrap();
        assert!(close(hit, 9.0));
        // At an angle: through the corner region, still the entry face.
        let slanted = Ray::new(Vec3::new(5.0, 0.0, 5.0), Vec3::new(-1.0, 0.0, -1.0));
        let hit = slanted.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)).unwrap();
        assert!(close(hit, 4.0 * std::f32::consts::SQRT_2));
    }

    #[test]
    fn a_box_beside_or_behind_the_ray_is_missed() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::NEG_Z);
        assert_eq!(ray.intersect_box(Vec3::new(2.0, -1.0, -1.0), Vec3::new(4.0, 1.0, 1.0)), None);
        let away = Ray::new(Vec3::new(0.0, 0.0, 10.0), Vec3::Z);
        assert_eq!(away.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)), None, "behind the origin");
        // Parallel to a face and outside its slab.
        let parallel = Ray::new(Vec3::new(0.0, 5.0, 10.0), Vec3::NEG_Z);
        assert_eq!(parallel.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)), None);
    }

    #[test]
    fn from_inside_the_hit_is_where_the_ray_leaves() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 0.5), Vec3::NEG_Z);
        assert!(close(ray.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)).unwrap(), 1.5));
    }

    #[test]
    fn edges_flat_boxes_and_bad_boxes() {
        let ray = Ray::new(Vec3::new(1.0, 0.0, 10.0), Vec3::NEG_Z);
        assert!(close(ray.intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)).unwrap(), 9.0), "grazing a face counts");
        let down = Ray::new(Vec3::new(0.5, 5.0, 0.5), Vec3::NEG_Y);
        assert!(close(down.intersect_box(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 0.0, 1.0)).unwrap(), 5.0), "a flat box (a plane's bounds)");
        assert_eq!(ray.intersect_box(Vec3::splat(1.0), Vec3::splat(-1.0)), None, "min above max: empty");
        assert_eq!(ray.intersect_box(Vec3::splat(-1.0), Vec3::new(1.0, f32::NAN, 1.0)), None);
        assert_eq!(Ray::new(Vec3::ZERO, Vec3::ZERO).intersect_box(Vec3::splat(-1.0), Vec3::splat(1.0)), None);
    }

    #[test]
    fn a_transformed_box_is_tested_in_its_own_space() {
        let (min, max) = (Vec3::splat(-1.0), Vec3::splat(1.0));
        // Moved to x = 10, scaled by 2: the faces at x = 8 and 12.
        let world = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)) * Mat4::from_scale(Vec3::splat(2.0));
        let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::X);
        assert!(close(ray.intersect_transformed_box(min, max, world).unwrap(), 8.0), "the distance is in world units");
        // Rotated 45 degrees about y: a diamond whose corner points at the ray, sqrt(2) from its centre.
        let rotated = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)) * Mat4::from_quat(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4));
        assert!(close(ray.intersect_transformed_box(min, max, rotated).unwrap(), 10.0 - std::f32::consts::SQRT_2));
        // A ray that would hit the world-aligned box around the diamond but misses the diamond.
        let past_corner = Ray::new(Vec3::new(0.0, 0.0, 1.3), Vec3::X);
        assert!(past_corner.intersect_transformed_box(Vec3::new(8.6, -1.0, -1.42), Vec3::new(11.42, 1.0, 1.42), Mat4::IDENTITY).is_some());
        let hit = past_corner.intersect_transformed_box(min, max, rotated).unwrap();
        assert!(hit > 8.6 + 0.5, "the diamond's slanted face, not the aligned box's: {hit}");
        // Unevenly scaled and mirrored.
        let mirrored = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)) * Mat4::from_scale(Vec3::new(-3.0, 1.0, 1.0));
        assert!(close(ray.intersect_transformed_box(min, max, mirrored).unwrap(), 7.0));
        // A scale of 0 cannot be inverted.
        assert_eq!(ray.intersect_transformed_box(min, max, Mat4::from_scale(Vec3::new(0.0, 1.0, 1.0))), None);
    }

    #[test]
    fn nearest_box_hit_picks_the_closest_box_and_the_first_of_a_tie() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 20.0), Vec3::NEG_Z);
        let unit = (Vec3::splat(-0.5), Vec3::splat(0.5));
        let at = |z: f32| Mat4::from_translation(Vec3::new(0.0, 0.0, z));
        let boxes = [
            (unit.0, unit.1, at(-5.0)),
            (unit.0, unit.1, Mat4::from_translation(Vec3::new(5.0, 0.0, 5.0))), // beside the ray
            (unit.0, unit.1, at(3.0)),
            (unit.0, unit.1, at(0.0)),
            (unit.0, unit.1, at(3.0)),
        ];
        let (index, distance) = ray.nearest_box_hit(boxes).unwrap();
        assert_eq!(index, 2);
        assert!(close(distance, 16.5));
        assert_eq!(ray.nearest_box_hit([(unit.0, unit.1, at(30.0))]), None, "behind the origin");
        assert_eq!(ray.nearest_box_hit(std::iter::empty()), None);
    }
}
