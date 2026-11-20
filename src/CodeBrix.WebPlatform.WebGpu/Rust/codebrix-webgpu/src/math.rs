//! The maths types: the `glam` crate under its OWN names - `Vec2`, `Vec3`, `Vec4`, `Mat3`,
//! `Mat4`, `Quat`, `Affine3A` and the rest (also the `f64` and integer types).
//!
//! There are no aliases for System.Numerics names on purpose: glam's matrices are column-major
//! and multiply the other way round. In .NET, `a * b` with `Matrix4x4` applies `a` FIRST (row
//! vectors); in glam, `a * b` with `Mat4` applies `b` first (column vectors), and a point is
//! transformed with `matrix.transform_point3(point)` or `matrix * vec4`. So the .NET
//! `scale * rotation * translation` is glam's `translation * rotation * scale`.
//!
//! With glam's `bytemuck` feature (which this crate turns on) the vector and matrix types are
//! plain bytes: `bytemuck::cast_slice(&[Mat4::IDENTITY])` gives what a uniform buffer takes
//! (an application that wants this names the `bytemuck` crate in its own Cargo.toml; the
//! package's crate bundle carries it).
//!
//! ```ignore
//! use codebrix_webgpu::math::{Mat4, Quat, Vec3};
//! let model = Mat4::from_scale_rotation_translation(Vec3::splat(2.0), Quat::from_rotation_y(0.5), Vec3::new(0.0, 1.0, 0.0));
//! // The view and projection constructors are in glam's `camera` module: right-handed, and
//! // `directx` for WebGPU's depth range 0..1 with y up.
//! let view = codebrix_webgpu::math::camera::rh::view::look_at_mat4(Vec3::new(0.0, 2.0, 5.0), Vec3::ZERO, Vec3::Y);
//! let projection = codebrix_webgpu::math::camera::rh::proj::directx::perspective(45f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
//! let model_view_projection = projection * view * model; // applied right to left
//! ```

pub use glam::*;

#[cfg(test)]
mod tests {
    use super::{Mat4, Quat, Vec3};

    #[test]
    fn the_right_hand_matrix_is_applied_first() {
        let translate = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let scale = Mat4::from_scale(Vec3::splat(2.0));
        // Scale first, then translate: (1,0,0) -> (2,0,0) -> (12,0,0).
        assert_eq!((translate * scale).transform_point3(Vec3::X), Vec3::new(12.0, 0.0, 0.0));
        // Translate first, then scale: (1,0,0) -> (11,0,0) -> (22,0,0).
        assert_eq!((scale * translate).transform_point3(Vec3::X), Vec3::new(22.0, 0.0, 0.0));
    }

    #[test]
    fn a_quaternion_turns_a_vector() {
        let turned = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Vec3::X;
        assert!(turned.abs_diff_eq(Vec3::Y, 1e-6));
    }

    #[test]
    fn the_camera_module_projects_into_webgpu_depth() {
        let view = super::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let projection = super::camera::rh::proj::directx::perspective(1.0, 1.0, 1.0, 100.0);
        let clip = (projection * view) * super::Vec4::new(0.0, 0.0, 4.0, 1.0);
        assert!((clip.z / clip.w).abs() < 1e-5, "the near plane is depth 0");
    }

    #[test]
    fn the_types_are_plain_bytes_for_the_gpu() {
        let bytes: &[u8] = bytemuck::bytes_of(&Mat4::IDENTITY);
        assert_eq!(bytes.len(), 64);
        assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
    }
}
