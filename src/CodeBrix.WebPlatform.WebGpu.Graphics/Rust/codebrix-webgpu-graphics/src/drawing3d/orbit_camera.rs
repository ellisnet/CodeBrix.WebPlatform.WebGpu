//! [`OrbitCamera`]: an orbit (turntable) controller - a target point, a distance and two
//! angles - that points a [`Camera`] (C#: `OrbitCamera`).

use glam::Vec3;

use crate::drawing3d::Camera;

/// An orbit (turntable) controller: a target point, a distance and two angles (C#:
/// `OrbitCamera`). Apply it to a [`Camera`] each frame; drive it from drags, a timer or keys - or
/// let an [`OrbitController`](crate::OrbitController) drive it from the canvas's input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    /// The point orbited (C#: `Target`, default the origin).
    pub target: Vec3,
    /// The angle around the vertical axis in radians; 0 puts the camera on +Z looking towards -Z
    /// (C#: `Yaw`).
    pub yaw: f32,
    pitch: f32,
    distance: f32,
    min_distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> OrbitCamera {
        OrbitCamera { target: Vec3::ZERO, yaw: 0.0, pitch: 0.4, distance: 10.0, min_distance: 0.01 }
    }
}

impl OrbitCamera {
    /// The largest pitch magnitude: just under 90 degrees (C#: `MaxPitch`).
    pub const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.001;

    /// A controller with the C# defaults: pitch 0.4, distance 10, minimum distance 0.01 (C#:
    /// `new OrbitCamera()`).
    pub fn new() -> OrbitCamera {
        OrbitCamera::default()
    }

    /// The elevation angle in radians (C#: `Pitch`).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets the elevation, clamped to just short of straight up or down (C#: `Pitch = value`).
    pub fn set_pitch(&mut self, value: f32) {
        self.pitch = value.clamp(-OrbitCamera::MAX_PITCH, OrbitCamera::MAX_PITCH);
    }

    /// The distance from the target (C#: `Distance`).
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Sets the distance, at least [`OrbitCamera::min_distance`] (C#: `Distance = value`).
    pub fn set_distance(&mut self, value: f32) {
        self.distance = value.max(self.min_distance);
    }

    /// The smallest distance allowed (C#: `MinDistance`, default 0.01).
    pub fn min_distance(&self) -> f32 {
        self.min_distance
    }

    /// Sets the smallest distance allowed (C#: `MinDistance = value`; as in C#, the current
    /// distance is held to it at its next change).
    pub fn set_min_distance(&mut self, value: f32) {
        self.min_distance = value;
    }

    /// The camera position the controller describes (C#: `Position`).
    pub fn position(&self) -> Vec3 {
        OrbitCamera::orbit_position(self.target, self.distance, self.yaw, self.pitch)
    }

    /// Turns the camera around the target (C#: `Rotate`): adds to the yaw and the (clamped)
    /// pitch.
    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.set_pitch(self.pitch + delta_pitch);
    }

    /// Multiplies the distance; below 1 zooms in (C#: `Zoom`).
    pub fn zoom(&mut self, factor: f32) {
        self.set_distance(self.distance * factor);
    }

    /// Points a camera at the target from the orbit position, up = +Y (C#: `Apply`).
    pub fn apply(&self, camera: &mut Camera) {
        camera.target = self.target;
        camera.position = self.position();
        camera.up = Vec3::Y;
    }

    /// The position on a sphere around a target: `target + distance * (cos p sin y, sin p, cos p
    /// cos y)` (C#: `OrbitPosition`).
    pub fn orbit_position(target: Vec3, distance: f32, yaw: f32, pitch: f32) -> Vec3 {
        target + distance * Vec3::new(pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos())
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::OrbitCamera;
    use crate::drawing3d::Camera;

    fn near(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    #[test]
    fn orbit_position_follows_yaw_and_pitch() {
        let target = Vec3::new(1.0, 2.0, 3.0);
        assert!(near(OrbitCamera::orbit_position(target, 10.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 13.0)));
        assert!(near(OrbitCamera::orbit_position(target, 10.0, std::f32::consts::FRAC_PI_2, 0.0), Vec3::new(11.0, 2.0, 3.0)));
        assert!(near(OrbitCamera::orbit_position(target, 10.0, 0.0, std::f32::consts::FRAC_PI_2), Vec3::new(1.0, 12.0, 3.0)));
    }

    #[test]
    fn it_clamps_pitch_and_distance_and_applies_to_a_camera() {
        let mut orbit = OrbitCamera::new();
        orbit.set_distance(5.0);
        let mut camera = Camera::new();
        orbit.rotate(std::f32::consts::PI, 10.0);
        orbit.zoom(0.5);
        orbit.set_distance(-3.0);
        assert_eq!(orbit.distance(), 0.01);
        orbit.set_distance(2.0);
        orbit.apply(&mut camera);
        assert_eq!(orbit.pitch(), OrbitCamera::MAX_PITCH);
        assert!((orbit.yaw - std::f32::consts::PI).abs() < 1e-6);
        assert_eq!(camera.target, Vec3::ZERO);
        assert!((camera.position.length() - 2.0).abs() < 1e-4);
        assert!(camera.position.y > 1.99);
    }
}
