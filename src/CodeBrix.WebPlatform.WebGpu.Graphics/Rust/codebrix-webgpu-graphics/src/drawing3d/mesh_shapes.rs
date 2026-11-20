//! [`MeshShapes`]: simple generated shapes as [`MeshData`] - boxes, cubes, planes and spheres,
//! with normals, texture coordinates and counter-clockwise front faces (C#: `MeshShapes`).

use glam::{Vec2, Vec3};

use crate::common::Color4;
use crate::drawing3d::{MeshData, MeshVertex};

/// The built-in mesh primitives (C#: `MeshShapes`). Every argument is written out: C#'s
/// defaults are given with each (a `None` colour is white).
pub struct MeshShapes;

impl MeshShapes {
    /// A cube centred on the origin, 24 vertices and 36 indices (C#: `Cube(size = 1, color =
    /// null)`).
    pub fn cube(size: f32, color: Option<Color4>) -> MeshData {
        MeshShapes::box_shape(Vec3::splat(size), color)
    }

    /// A box centred on the origin, 24 vertices and 36 indices (C#: `Box(size, color = null)`;
    /// `box` is a Rust keyword).
    pub fn box_shape(size: Vec3, color: Option<Color4>) -> MeshData {
        let tint = color.unwrap_or(Color4::WHITE);
        let half = size / 2.0;
        let mut data = MeshData::default();
        add_face(&mut data, Vec3::X, -Vec3::Z, Vec3::Y, half, tint);
        add_face(&mut data, -Vec3::X, Vec3::Z, Vec3::Y, half, tint);
        add_face(&mut data, Vec3::Y, Vec3::X, -Vec3::Z, half, tint);
        add_face(&mut data, -Vec3::Y, Vec3::X, Vec3::Z, half, tint);
        add_face(&mut data, Vec3::Z, Vec3::X, Vec3::Y, half, tint);
        add_face(&mut data, -Vec3::Z, -Vec3::X, Vec3::Y, half, tint);
        data
    }

    /// A horizontal plane (normal +Y) centred on the origin, 4 vertices and 6 indices; the
    /// texture repeats `uv_repeat` times across it (use a repeating sampler) (C#: `Plane(width,
    /// depth, color = null, uvRepeat = 1)`).
    pub fn plane(width: f32, depth: f32, color: Option<Color4>, uv_repeat: f32) -> MeshData {
        let tint = color.unwrap_or(Color4::WHITE);
        let x = width / 2.0;
        let z = depth / 2.0;
        let vertices = vec![
            MeshVertex::new(Vec3::new(-x, 0.0, z), Vec3::Y, Vec2::new(0.0, uv_repeat), tint),
            MeshVertex::new(Vec3::new(x, 0.0, z), Vec3::Y, Vec2::new(uv_repeat, uv_repeat), tint),
            MeshVertex::new(Vec3::new(x, 0.0, -z), Vec3::Y, Vec2::new(uv_repeat, 0.0), tint),
            MeshVertex::new(Vec3::new(-x, 0.0, -z), Vec3::Y, Vec2::new(0.0, 0.0), tint),
        ];
        MeshData::new(vertices, vec![0, 1, 2, 0, 2, 3])
    }

    /// A UV sphere centred on the origin, with `segments` divisions around the equator (at least
    /// 3) and `rings` from pole to pole (at least 2) (C#: `Sphere(radius = 0.5, segments = 32,
    /// rings = 16, color = null)`).
    pub fn sphere(radius: f32, segments: u32, rings: u32, color: Option<Color4>) -> MeshData {
        let tint = color.unwrap_or(Color4::WHITE);
        let segments = segments.max(3);
        let rings = rings.max(2);
        let mut vertices = Vec::with_capacity(((rings + 1) * (segments + 1)) as usize);
        for ring in 0..=rings {
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..=segments {
                let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
                let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                vertices.push(MeshVertex::new(normal * radius, normal, Vec2::new(segment as f32 / segments as f32, ring as f32 / rings as f32), tint));
            }
        }
        let mut indices = Vec::new();
        let stride = segments + 1;
        for ring in 0..rings {
            for segment in 0..segments {
                let a = ring * stride + segment;
                let b = (ring + 1) * stride + segment;
                let c = b + 1;
                let d = a + 1;
                add_outward(&vertices, &mut indices, a, b, c);
                add_outward(&vertices, &mut indices, a, c, d);
            }
        }
        MeshData::new(vertices, indices)
    }
}

/// One face of a box: four vertices around the face's centre, two triangles.
fn add_face(data: &mut MeshData, normal: Vec3, u: Vec3, v: Vec3, half: Vec3, color: Color4) {
    let first = data.vertices.len() as u32;
    let center = normal * half;
    let du = u * half;
    let dv = v * half;
    data.vertices.push(MeshVertex::new(center - du - dv, normal, Vec2::new(0.0, 1.0), color));
    data.vertices.push(MeshVertex::new(center + du - dv, normal, Vec2::new(1.0, 1.0), color));
    data.vertices.push(MeshVertex::new(center + du + dv, normal, Vec2::new(1.0, 0.0), color));
    data.vertices.push(MeshVertex::new(center - du + dv, normal, Vec2::new(0.0, 0.0), color));
    data.indices.extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
}

/// A sphere triangle wound to face outward; a degenerate one (at a pole) is left out.
fn add_outward(vertices: &[MeshVertex], indices: &mut Vec<u32>, a: u32, b: u32, c: u32) {
    let pa = vertices[a as usize].position;
    let pb = vertices[b as usize].position;
    let pc = vertices[c as usize].position;
    let cross = (pb - pa).cross(pc - pa);
    if cross.length_squared() < 1e-12 {
        return;
    }
    if cross.dot(pa + pb + pc) >= 0.0 {
        indices.extend_from_slice(&[a, b, c]);
    } else {
        indices.extend_from_slice(&[a, c, b]);
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3, Vec4};

    use super::MeshShapes;
    use crate::common::Color4;
    use crate::drawing3d::MeshData;

    fn all_triangles_face_outward(data: &MeshData) {
        for triangle in data.indices.chunks(3) {
            let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|i| data.vertices[i as usize]);
            let face = (b.position - a.position).cross(c.position - a.position);
            assert!(face.dot(a.normal + b.normal + c.normal) > 0.0);
        }
    }

    #[test]
    fn cube_has_24_vertices_36_indices_and_outward_faces() {
        let cube = MeshShapes::cube(2.0, Some(Color4::RED));
        let (min, max) = cube.get_bounds();
        assert_eq!((cube.vertices.len(), cube.indices.len()), (24, 36));
        assert_eq!((min, max), (Vec3::splat(-1.0), Vec3::splat(1.0)));
        assert_eq!(cube.vertices[0].color, Vec4::new(1.0, 0.0, 0.0, 1.0));
        all_triangles_face_outward(&cube);
    }

    #[test]
    fn sphere_and_plane_face_outward() {
        let sphere = MeshShapes::sphere(1.0, 12, 6, None);
        let plane = MeshShapes::plane(2.0, 3.0, None, 1.0);
        all_triangles_face_outward(&sphere);
        all_triangles_face_outward(&plane);
        assert!(plane.vertices.iter().all(|v| v.normal == Vec3::Y));
        assert_eq!(sphere.vertices.len(), 7 * 13);
        assert!(sphere.vertices.iter().all(|v| (v.position.length() - 1.0).abs() < 1e-5));
        // The pole rows' degenerate triangles are left out: 2 per quad but one per pole quad.
        assert_eq!(sphere.indices.len(), (12 * 6 * 2 - 2 * 12) * 3);
    }

    #[test]
    fn a_box_has_its_size_and_the_minimums_hold() {
        let shape = MeshShapes::box_shape(Vec3::new(2.0, 4.0, 6.0), None);
        assert_eq!(shape.get_bounds(), (Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0)));
        all_triangles_face_outward(&shape);
        let tiny = MeshShapes::sphere(1.0, 1, 1, None);
        assert_eq!(tiny.vertices.len(), 3 * 4);
        assert_eq!(MeshShapes::plane(1.0, 1.0, None, 4.0).vertices[1].tex_coord, glam::Vec2::new(4.0, 4.0));
    }

    #[test]
    fn transformed_and_combine_move_positions_and_rebase_indices() {
        let plane = MeshShapes::plane(2.0, 2.0, None, 1.0);
        let moved = plane.transformed(Mat4::from_translation(Vec3::new(0.0, 5.0, 0.0)));
        let both = MeshData::combine([&plane, &moved]);
        assert_eq!(moved.vertices[0].position.y, 5.0);
        assert_eq!((both.vertices.len(), both.indices.len()), (8, 12));
        assert_eq!(both.indices[6], 4);
        let turned = plane.transformed(Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2));
        assert!(turned.vertices[0].normal.distance(Vec3::Z) < 1e-5);
        assert_eq!(MeshData::default().get_bounds(), (Vec3::ZERO, Vec3::ZERO));
    }
}
