//! The few vector, quaternion and matrix operations the glTF builder needs, written out so that
//! they do EXACTLY what the reference converter's .NET `System.Numerics` calls do, operation for
//! operation (C#: `Vector3`, `Vector4`, `Quaternion`, `Matrix4x4`, `Matrix3x2`). glam's own
//! functions are not used for these: glam normalises by multiplying with a reciprocal where .NET
//! divides, and builds matrices with other (equally valid) expressions, which changes the last
//! bit of a result - and the loader's output is checked against the reference converter's.

/// `a + b` (C#: `Vector3 + Vector3`).
pub(crate) fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a - b` (C#: `Vector3 - Vector3`).
pub(crate) fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `a * s` (C#: `Vector3 * float`).
pub(crate) fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// The dot product (C#: `Vector3.Dot`: the three products summed first to last).
pub(crate) fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}

/// The cross product (C#: `Vector3.Cross`).
pub(crate) fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// The squared length (C#: `Vector3.LengthSquared`).
pub(crate) fn length_squared3(a: [f32; 3]) -> f32 {
    dot3(a, a)
}

/// The vector divided by its length (C#: `Vector3.Normalize`, which DIVIDES by the length; a
/// zero vector gives NaN there and here - callers test the length first, as the C# code does).
pub(crate) fn normalize3(a: [f32; 3]) -> [f32; 3] {
    let length = length_squared3(a).sqrt();
    [a[0] / length, a[1] / length, a[2] / length]
}

/// The length of a quaternion or 4-vector (C#: `Quaternion.Length`: the four products summed in
/// pairs, as the runtime's vectorised dot product does).
pub(crate) fn length4(q: [f32; 4]) -> f32 {
    ((q[0] * q[0] + q[1] * q[1]) + (q[2] * q[2] + q[3] * q[3])).sqrt()
}

/// The unit quaternion, or the identity when the length is not above 1e-12 (C#: the builder's
/// `NormalizeOrIdentity`, with `Quaternion.Normalize` dividing by the length). Order x, y, z, w.
pub(crate) fn normalize_or_identity(q: [f32; 4]) -> [f32; 4] {
    let length = length4(q);
    if length > 1e-12 {
        [q[0] / length, q[1] / length, q[2] / length, q[3] / length]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// The matrix that scales, then rotates, then translates, as 16 floats in .NET's `Matrix4x4`
/// member order M11, M12, ... M44 - the same 16 floats as glam's column-major `Mat4` (C#: the
/// builder's `TrsMatrix`: `Matrix4x4.CreateFromQuaternion`, its three rows multiplied by the
/// scale, the translation in the fourth row).
pub(crate) fn trs_matrix(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> [f32; 16] {
    let [x, y, z, w] = rotation;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, wz, xz, wy, yz, wx) = (x * y, z * w, z * x, y * w, y * z, x * w);
    let mut m = [
        1.0 - 2.0 * (yy + zz),
        2.0 * (xy + wz),
        2.0 * (xz - wy),
        0.0,
        2.0 * (xy - wz),
        1.0 - 2.0 * (zz + xx),
        2.0 * (yz + wx),
        0.0,
        2.0 * (xz + wy),
        2.0 * (yz - wx),
        1.0 - 2.0 * (yy + xx),
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ];
    for row in 0..3 {
        for column in 0..3 {
            m[row * 4 + column] *= scale[row];
        }
    }
    m[12] = translation[0];
    m[13] = translation[1];
    m[14] = translation[2];
    m
}

/// A matrix node's translation, rotation and scale, or None when the matrix has none (a shear,
/// a projection) - .NET's `Matrix4x4.Decompose`, operation for operation (C#: the reference
/// converter's `node.LocalTransform.TryDecompose`, which calls it for every node given as a
/// matrix). `m` is the node's matrix as glTF writes it (16 floats, column-major: the same order
/// as .NET's M11..M44); its fourth column is not looked at (C#: the glTF library forces it to
/// 0, 0, 0, 1). Returns (translation, rotation x y z w, scale).
pub(crate) fn decompose(m: &[f32; 16]) -> Option<([f32; 3], [f32; 4], [f32; 3])> {
    const EPSILON: f32 = 0.0001;
    const CANONICAL: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let translation = [m[12], m[13], m[14]];
    let mut basis = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
    let mut scales = [length_squared3(basis[0]).sqrt(), length_squared3(basis[1]).sqrt(), length_squared3(basis[2]).sqrt()];
    let (x, y, z) = (scales[0], scales[1], scales[2]);
    let (a, b, c): (usize, usize, usize) = if x < y {
        if y < z {
            (2, 1, 0)
        } else if x < z {
            (1, 2, 0)
        } else {
            (1, 0, 2)
        }
    } else if x < z {
        (2, 0, 1)
    } else if y < z {
        (0, 2, 1)
    } else {
        (0, 1, 2)
    };
    if scales[a] < EPSILON {
        basis[a] = CANONICAL[a];
    }
    basis[a] = normalize3(basis[a]);
    if scales[b] < EPSILON {
        let (abs_x, abs_y, abs_z) = (basis[a][0].abs(), basis[a][1].abs(), basis[a][2].abs());
        let cc: usize = if abs_x < abs_y {
            if abs_y < abs_z || abs_x < abs_z {
                0
            } else {
                2
            }
        } else if abs_x < abs_z || abs_y < abs_z {
            1
        } else {
            2
        };
        basis[b] = cross3(basis[a], CANONICAL[cc]);
    }
    basis[b] = normalize3(basis[b]);
    if scales[c] < EPSILON {
        basis[c] = cross3(basis[a], basis[b]);
    }
    basis[c] = normalize3(basis[c]);
    // The determinant of the 4 x 4 matrix whose rows are the basis and (0, 0, 0, 1).
    let [[m11, m12, m13], [m21, m22, m23], [m31, m32, m33]] = basis;
    let mut determinant = m11 * (m22 * m33 - m23 * m32) - m12 * (m21 * m33 - m23 * m31) + m13 * (m21 * m32 - m22 * m31);
    if determinant < 0.0 {
        scales[a] = -scales[a];
        basis[a] = [-basis[a][0], -basis[a][1], -basis[a][2]];
        determinant = -determinant;
    }
    determinant -= 1.0;
    determinant *= determinant;
    if EPSILON < determinant {
        return None;
    }
    Some((translation, quaternion_from_rotation(&basis), scales))
}

/// The quaternion (x, y, z, w) of a rotation matrix given by its three rows (C#:
/// `Quaternion.CreateFromRotationMatrix`).
fn quaternion_from_rotation(rows: &[[f32; 3]; 3]) -> [f32; 4] {
    let [[m11, m12, m13], [m21, m22, m23], [m31, m32, m33]] = *rows;
    let trace = m11 + m22 + m33;
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt();
        let w = s * 0.5;
        let s = 0.5 / s;
        [(m23 - m32) * s, (m31 - m13) * s, (m12 - m21) * s, w]
    } else if m11 >= m22 && m11 >= m33 {
        let s = (1.0 + m11 - m22 - m33).sqrt();
        let inverse = 0.5 / s;
        [0.5 * s, (m12 + m21) * inverse, (m13 + m31) * inverse, (m23 - m32) * inverse]
    } else if m22 > m33 {
        let s = (1.0 + m22 - m11 - m33).sqrt();
        let inverse = 0.5 / s;
        [(m21 + m12) * inverse, 0.5 * s, (m32 + m23) * inverse, (m31 - m13) * inverse]
    } else {
        let s = (1.0 + m33 - m11 - m22).sqrt();
        let inverse = 0.5 / s;
        [(m31 + m13) * inverse, (m32 + m23) * inverse, 0.5 * s, (m12 - m21) * inverse]
    }
}

/// `a * b` for .NET `Matrix3x2` values (M11, M12, M21, M22, M31, M32), row-vector convention.
fn multiply3x2(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

/// The matrix of a KHR_texture_transform as .NET's `Matrix3x2` (M11, M12, M21, M22, M31, M32):
/// scale, then a rotation by MINUS `rotation`, then the offset (C#: the glTF library's
/// `TextureTransform.Matrix`: `CreateScale(scale) * CreateRotation(-rotation) *
/// CreateTranslation(offset)`). As .NET's `CreateRotation` does, an angle within a thousandth of
/// a degree of 0, a quarter turn or a half turn gives the exact matrix of that turn.
pub(crate) fn texture_transform_matrix(offset: [f32; 2], rotation: f32, scale: [f32; 2]) -> [f32; 6] {
    use std::f32::consts::PI;
    const EPSILON: f32 = 0.001 * PI / 180.0;
    let radians = ieee_remainder(-rotation, PI * 2.0);
    let (c, s) = if radians > -EPSILON && radians < EPSILON {
        (1.0, 0.0)
    } else if radians > PI / 2.0 - EPSILON && radians < PI / 2.0 + EPSILON {
        (0.0, 1.0)
    } else if radians < -PI + EPSILON || radians > PI - EPSILON {
        (-1.0, 0.0)
    } else if radians > -PI / 2.0 - EPSILON && radians < -PI / 2.0 + EPSILON {
        (0.0, -1.0)
    } else {
        (radians.cos(), radians.sin())
    };
    let scale_matrix = [scale[0], 0.0, 0.0, scale[1], 0.0, 0.0];
    let rotation_matrix = [c, s, -s, c, 0.0, 0.0];
    let translation_matrix = [1.0, 0.0, 0.0, 1.0, offset[0], offset[1]];
    multiply3x2(multiply3x2(scale_matrix, rotation_matrix), translation_matrix)
}

/// The IEEE remainder: `x - y * n` with n the integer nearest to `x / y` (C#:
/// `MathF.IEEERemainder`, written the way the .NET runtime writes it).
fn ieee_remainder(x: f32, y: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }
    let regular = x % y;
    if regular.is_nan() {
        return f32::NAN;
    }
    if regular == 0.0 && x.is_sign_negative() {
        return -0.0;
    }
    let sign = if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    };
    let alternative = regular - (y.abs() * sign);
    if alternative.abs() == regular.abs() {
        let division = x / y;
        // .NET's MathF.Round: to the nearest integer, halfway cases to the even one.
        let rounded = division.round_ties_even();
        return if rounded.abs() > division.abs() { alternative } else { regular };
    }
    if alternative.abs() < regular.abs() {
        alternative
    } else {
        regular
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3};

    use super::*;

    #[test]
    fn the_vector_operations_are_the_plain_formulas() {
        assert_eq!(add3([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), [5.0, 7.0, 9.0]);
        assert_eq!(sub3([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), [-3.0, -3.0, -3.0]);
        assert_eq!(scale3([1.0, 2.0, 3.0], 2.0), [2.0, 4.0, 6.0]);
        assert_eq!(dot3([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
        assert_eq!(cross3([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(normalize3([3.0, 0.0, 4.0]), [0.6, 0.0, 0.8]);
        assert_eq!(length4([1.0, 2.0, 2.0, 4.0]), 5.0);
    }

    #[test]
    fn a_normalised_vector_is_divided_by_its_length_not_multiplied_by_a_reciprocal() {
        // 1/3 of each component by division; the reciprocal product differs in the last bit for
        // some inputs, which is why this module exists.
        let v = [1.0f32, 1.0, 1.0];
        let length = 3.0f32.sqrt();
        assert_eq!(normalize3(v), [1.0 / length, 1.0 / length, 1.0 / length]);
    }

    #[test]
    fn a_quaternion_is_normalised_or_replaced_by_the_identity() {
        assert_eq!(normalize_or_identity([0.0, 0.0, 0.0, 2.0]), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(normalize_or_identity([0.0, 0.0, 0.0, 0.0]), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(normalize_or_identity([0.0, 3.0, 0.0, 4.0]), [0.0, 0.6, 0.0, 0.8]);
        assert_eq!(normalize_or_identity([1e-13, 0.0, 0.0, 0.0]), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn the_trs_matrix_is_glam_s_up_to_rounding_and_has_dotnet_s_layout() {
        let (t, s) = ([1.0, 2.0, 3.0], [2.0, 3.0, 4.0]);
        let q = Quat::from_euler(glam::EulerRot::XYZ, 0.3, -0.7, 1.1);
        let ours = trs_matrix(t, [q.x, q.y, q.z, q.w], s);
        let glam = Mat4::from_scale_rotation_translation(Vec3::from(s), q, Vec3::from(t)).to_cols_array();
        for (a, b) in ours.iter().zip(glam.iter()) {
            assert!((a - b).abs() < 1e-6, "{ours:?} against {glam:?}");
        }
        assert_eq!(trs_matrix([5.0, 6.0, 7.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]), [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 6.0, 7.0, 1.0]);
        // The scale multiplies ROWS (M11..M13 by x), as the builder's TrsMatrix does.
        assert_eq!(trs_matrix([0.0; 3], [0.0, 0.0, 0.0, 1.0], [2.0, 3.0, 4.0])[..11], [2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0]);
    }

    #[test]
    fn a_matrix_decomposes_into_what_it_was_made_of() {
        let q = Quat::from_euler(glam::EulerRot::XYZ, 0.4, 0.2, -0.9);
        let matrix = trs_matrix([1.0, -2.0, 3.0], [q.x, q.y, q.z, q.w], [2.0, 0.5, 3.0]);
        let (translation, rotation, scale) = decompose(&matrix).unwrap();
        assert_eq!(translation, [1.0, -2.0, 3.0]);
        for (a, b) in scale.iter().zip([2.0f32, 0.5, 3.0]) {
            assert!((a - b).abs() < 1e-5);
        }
        let back = Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]);
        assert!(back.dot(q).abs() > 0.99999);
        // The identity is exact.
        let identity = trs_matrix([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3]);
        assert_eq!(decompose(&identity), Some(([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3])));
    }

    #[test]
    fn a_mirrored_matrix_gets_a_negative_scale_and_a_sheared_one_does_not_decompose() {
        let mirrored = [-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let (_, rotation, scale) = decompose(&mirrored).unwrap();
        assert_eq!(scale.iter().filter(|s| **s < 0.0).count(), 1);
        assert!((length4(rotation) - 1.0).abs() < 1e-6);
        let sheared = [1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        assert_eq!(decompose(&sheared), None);
        // A matrix with no size at all still decomposes (to a zero scale), as .NET's does.
        assert!(decompose(&[0.0; 16]).is_some());
    }

    #[test]
    fn a_texture_transform_scales_rotates_backwards_and_offsets() {
        assert_eq!(texture_transform_matrix([0.0, 0.0], 0.0, [1.0, 1.0]), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(texture_transform_matrix([0.5, 0.25], 0.0, [2.0, 3.0]), [2.0, 0.0, 0.0, 3.0, 0.5, 0.25]);
        // A quarter turn is exact: rotation by -90 degrees is cos 0, sin -1.
        assert_eq!(texture_transform_matrix([0.0, 1.0], std::f32::consts::FRAC_PI_2, [1.0, 1.0]), [0.0, -1.0, 1.0, 0.0, 0.0, 1.0]);
        assert_eq!(texture_transform_matrix([0.0, 0.0], std::f32::consts::PI, [1.0, 1.0]), [-1.0, 0.0, -0.0, -1.0, 0.0, 0.0]);
        let m = texture_transform_matrix([0.0, 0.0], 0.3, [1.0, 1.0]);
        assert!((m[0] - 0.3f32.cos()).abs() < 1e-6 && (m[1] + 0.3f32.sin()).abs() < 1e-6 && (m[2] - 0.3f32.sin()).abs() < 1e-6);
    }
}
