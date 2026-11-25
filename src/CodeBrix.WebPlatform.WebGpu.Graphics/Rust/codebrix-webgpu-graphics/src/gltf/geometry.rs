//! The geometry steps of the glTF builder that work on one primitive's vertices and indices:
//! triangle strips and fans turned into lists, smooth normals for a primitive without NORMAL, and
//! tangents for a primitive with a normal map and no TANGENT (C#: the reference converter's
//! `GltfModelBuilder.FromStrip`, `FromFan`, `GenerateNormals` and `GenerateTangents`, ported
//! operation for operation - this is NOT MikkTSpace, on purpose: the browser and the desktop
//! library must draw a model alike).

use glam::{Vec2, Vec3};

use crate::gltf::numerics::{add3, cross3, dot3, length_squared3, normalize3, scale3, sub3};
use crate::models::ModelVertex;

/// A triangle strip as a triangle list; every second triangle is turned so that all wind the same
/// way (C#: `FromStrip`). Fewer than 3 indices give none.
pub(crate) fn from_strip(strip: &[u32]) -> Vec<u32> {
    if strip.len() < 3 {
        return Vec::new();
    }
    let mut list = Vec::with_capacity((strip.len() - 2) * 3);
    for index in 0..strip.len() - 2 {
        let even = index & 1 == 0;
        list.push(strip[index]);
        list.push(if even { strip[index + 1] } else { strip[index + 2] });
        list.push(if even { strip[index + 2] } else { strip[index + 1] });
    }
    list
}

/// A triangle fan as a triangle list: (i + 1, i + 2, first) for every i (C#: `FromFan`). Fewer
/// than 3 indices give none.
pub(crate) fn from_fan(fan: &[u32]) -> Vec<u32> {
    if fan.len() < 3 {
        return Vec::new();
    }
    let mut list = Vec::with_capacity((fan.len() - 2) * 3);
    for index in 0..fan.len() - 2 {
        list.push(fan[index + 1]);
        list.push(fan[index + 2]);
        list.push(fan[0]);
    }
    list
}

/// Smooth normals: every triangle's face normal (not normalised, so a larger triangle counts
/// for more) is added to its three vertices, then each sum is normalised; a vertex no triangle
/// uses, or whose sum is shorter than 1e-10, gets +Y (C#: `GenerateNormals`). What the normals
/// held before is added to: the caller hands vertices whose normals are zero. `indices` are a
/// triangle list relative to `vertices`, every one below `vertices.len()`.
pub(crate) fn generate_normals(vertices: &mut [ModelVertex], indices: &[u32]) {
    let mut index = 0;
    while index + 2 < indices.len() {
        let (ia, ib, ic) = (indices[index] as usize, indices[index + 1] as usize, indices[index + 2] as usize);
        let (a, b, c) = (vertices[ia].position.to_array(), vertices[ib].position.to_array(), vertices[ic].position.to_array());
        let face = cross3(sub3(b, a), sub3(c, a));
        // One after the other, as C# does: a triangle that names a vertex twice adds to it twice.
        vertices[ia].normal = Vec3::from(add3(vertices[ia].normal.to_array(), face));
        vertices[ib].normal = Vec3::from(add3(vertices[ib].normal.to_array(), face));
        vertices[ic].normal = Vec3::from(add3(vertices[ic].normal.to_array(), face));
        index += 3;
    }
    for vertex in vertices.iter_mut() {
        let normal = vertex.normal.to_array();
        vertex.normal = if length_squared3(normal) > 1e-20 { Vec3::from(normalize3(normal)) } else { Vec3::Y };
    }
}

/// Tangents from the texture coordinates of set 0 or 1: per triangle the direction of +U (with V
/// turned over, as glTF's V runs down) and of the bitangent, summed per vertex; then per vertex
/// the tangent made perpendicular to the normal and normalised, with w = -1 when the summed
/// bitangent points against `normal x tangent`, else +1. A triangle whose texture coordinates
/// span no area (determinant below 1e-12) adds nothing; a vertex left without a direction gets a
/// tangent perpendicular to its normal, built from the X axis (or the Y axis when the normal is
/// nearly along X) (C#: `GenerateTangents`). The normals must be final before this is called.
pub(crate) fn generate_tangents(vertices: &mut [ModelVertex], indices: &[u32], set: u32) {
    let uv = |vertex: &ModelVertex| -> Vec2 {
        if set == 1 {
            vertex.tex_coord1
        } else {
            vertex.tex_coord0
        }
    };
    let mut tangent = vec![[0.0f32; 3]; vertices.len()];
    let mut bitangent = vec![[0.0f32; 3]; vertices.len()];
    let mut index = 0;
    while index + 2 < indices.len() {
        let (i0, i1, i2) = (indices[index] as usize, indices[index + 1] as usize, indices[index + 2] as usize);
        index += 3;
        let (t0, t1, t2) = (uv(&vertices[i0]), uv(&vertices[i1]), uv(&vertices[i2]));
        let e1 = sub3(vertices[i1].position.to_array(), vertices[i0].position.to_array());
        let e2 = sub3(vertices[i2].position.to_array(), vertices[i0].position.to_array());
        let du1 = t1.x - t0.x;
        let du2 = t2.x - t0.x;
        let dv1 = -(t1.y - t0.y);
        let dv2 = -(t2.y - t0.y);
        let determinant = (du1 * dv2) - (du2 * dv1);
        if determinant.abs() < 1e-12 {
            continue;
        }
        let inverse = 1.0 / determinant;
        let s = scale3(sub3(scale3(e1, dv2), scale3(e2, dv1)), inverse);
        let t = scale3(sub3(scale3(e2, du1), scale3(e1, du2)), inverse);
        tangent[i0] = add3(tangent[i0], s);
        tangent[i1] = add3(tangent[i1], s);
        tangent[i2] = add3(tangent[i2], s);
        bitangent[i0] = add3(bitangent[i0], t);
        bitangent[i1] = add3(bitangent[i1], t);
        bitangent[i2] = add3(bitangent[i2], t);
    }
    for (index, vertex) in vertices.iter_mut().enumerate() {
        let n = vertex.normal.to_array();
        let mut t = sub3(tangent[index], scale3(n, dot3(n, tangent[index])));
        if length_squared3(t) < 1e-20 {
            let axis = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
            t = cross3(n, cross3(axis, n));
        }
        let t = normalize3(t);
        let handedness = if dot3(cross3(n, t), bitangent[index]) < 0.0 { -1.0 } else { 1.0 };
        vertex.tangent = [t[0], t[1], t[2], handedness];
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec2, Vec3};

    use super::{from_fan, from_strip, generate_normals, generate_tangents};
    use crate::models::ModelVertex;

    fn vertex(position: [f32; 3], uv: [f32; 2]) -> ModelVertex {
        ModelVertex { position: Vec3::from(position), tex_coord0: Vec2::from(uv), ..ModelVertex::default() }
    }

    #[test]
    fn a_strip_becomes_a_list_with_every_second_triangle_turned() {
        assert_eq!(from_strip(&[0, 1, 2, 3, 4]), vec![0, 1, 2, 1, 3, 2, 2, 3, 4]);
        assert_eq!(from_strip(&[7, 8, 9]), vec![7, 8, 9]);
        assert!(from_strip(&[0, 1]).is_empty());
        assert!(from_strip(&[]).is_empty());
    }

    #[test]
    fn a_fan_becomes_a_list_that_ends_each_triangle_on_the_first_vertex() {
        assert_eq!(from_fan(&[0, 1, 2, 3, 4]), vec![1, 2, 0, 2, 3, 0, 3, 4, 0]);
        assert!(from_fan(&[0, 1]).is_empty());
    }

    #[test]
    fn normals_are_area_weighted_and_an_unused_vertex_points_up() {
        // A unit right triangle in the XY plane (normal +Z, cross length 1) and a larger one in
        // the XZ plane sharing vertex 0 (cross (0, -4, 0)); vertex 5 is used by no triangle.
        let mut vertices = vec![
            vertex([0.0, 0.0, 0.0], [0.0, 0.0]),
            vertex([1.0, 0.0, 0.0], [0.0, 0.0]),
            vertex([0.0, 1.0, 0.0], [0.0, 0.0]),
            vertex([2.0, 0.0, 0.0], [0.0, 0.0]),
            vertex([0.0, 0.0, 2.0], [0.0, 0.0]),
            vertex([9.0, 9.0, 9.0], [0.0, 0.0]),
        ];
        generate_normals(&mut vertices, &[0, 1, 2, 0, 3, 4]);
        assert_eq!(vertices[1].normal, Vec3::Z);
        assert_eq!(vertices[3].normal, Vec3::NEG_Y);
        let shared = Vec3::new(0.0, -4.0, 1.0);
        let length = (shared.x * shared.x + shared.y * shared.y + shared.z * shared.z).sqrt();
        assert_eq!(vertices[0].normal, Vec3::new(0.0 / length, -4.0 / length, 1.0 / length));
        assert_eq!(vertices[5].normal, Vec3::Y);
    }

    #[test]
    fn a_degenerate_triangle_leaves_its_vertices_pointing_up() {
        let mut vertices = vec![vertex([1.0, 1.0, 1.0], [0.0, 0.0]); 3];
        generate_normals(&mut vertices, &[0, 1, 2]);
        assert!(vertices.iter().all(|v| v.normal == Vec3::Y));
    }

    #[test]
    fn tangents_follow_u_and_the_handedness_follows_the_bitangent() {
        // A quad in the XY plane, normal +Z, U along +X. glTF's V runs DOWN the image, so V
        // growing along -Y is the usual layout: tangent +X, handedness +1.
        let quad = |flip_v: bool| {
            let v = |y: f32| if flip_v { y } else { 1.0 - y };
            let mut vertices = vec![
                vertex([0.0, 0.0, 0.0], [0.0, v(0.0)]),
                vertex([1.0, 0.0, 0.0], [1.0, v(0.0)]),
                vertex([1.0, 1.0, 0.0], [1.0, v(1.0)]),
                vertex([0.0, 1.0, 0.0], [0.0, v(1.0)]),
            ];
            for vertex in vertices.iter_mut() {
                vertex.normal = Vec3::Z;
            }
            generate_tangents(&mut vertices, &[0, 1, 2, 0, 2, 3], 0);
            vertices
        };
        for vertex in quad(false) {
            assert_eq!(vertex.tangent, [1.0, 0.0, 0.0, 1.0]);
        }
        for vertex in quad(true) {
            assert_eq!(vertex.tangent, [1.0, 0.0, 0.0, -1.0]);
        }
    }

    #[test]
    fn tangents_use_the_second_texture_coordinate_set_when_asked() {
        let mut vertices = vec![vertex([0.0, 0.0, 0.0], [5.0, 5.0]), vertex([1.0, 0.0, 0.0], [5.0, 5.0]), vertex([0.0, 1.0, 0.0], [5.0, 5.0])];
        let uv1 = [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0]];
        for (vertex, uv) in vertices.iter_mut().zip(uv1) {
            vertex.normal = Vec3::Z;
            vertex.tex_coord1 = Vec2::from(uv);
        }
        generate_tangents(&mut vertices, &[0, 1, 2], 1);
        // U grows along +Y in set 1.
        assert_eq!(vertices[0].tangent[..3], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_vertex_without_usable_texture_coordinates_gets_a_tangent_perpendicular_to_its_normal() {
        // All texture coordinates equal: the determinant is 0, nothing is summed.
        let mut vertices = vec![vertex([0.0, 0.0, 0.0], [0.5, 0.5]), vertex([1.0, 0.0, 0.0], [0.5, 0.5]), vertex([0.0, 1.0, 0.0], [0.5, 0.5])];
        vertices[0].normal = Vec3::Z;
        vertices[1].normal = Vec3::X;
        vertices[2].normal = Vec3::new(0.0, 0.6, 0.8);
        generate_tangents(&mut vertices, &[0, 1, 2], 0);
        assert_eq!(vertices[0].tangent, [1.0, 0.0, 0.0, 1.0]);
        // Normal along X: the Y axis is used instead.
        assert_eq!(vertices[1].tangent, [0.0, 1.0, 0.0, 1.0]);
        let t = Vec3::new(vertices[2].tangent[0], vertices[2].tangent[1], vertices[2].tangent[2]);
        assert!(t.dot(vertices[2].normal).abs() < 1e-6 && (t.length() - 1.0).abs() < 1e-6);
    }
}
