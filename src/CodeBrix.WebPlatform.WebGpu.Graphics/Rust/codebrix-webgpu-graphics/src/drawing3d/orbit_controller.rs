//! [`OrbitController`]: an [`OrbitCamera`] driven by the canvas host's input - drag to orbit,
//! wheel to zoom, one finger to orbit and two to pinch-zoom - within [`OrbitLimits`], with the
//! model viewer's auto-orbit until the user drags. The C# library leaves this to each viewer
//! (its desktop and Android hosts); here it is one type, with the viewers' numbers as defaults.

use codebrix_webgpu::CanvasInput;
use glam::Vec2;

use crate::drawing3d::{Camera, OrbitCamera};

/// The limits an [`OrbitController`] holds the camera to (the model viewers' on Android, which
/// suit a pointer and a touch screen).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitLimits {
    /// The lowest pitch, in radians (default 0.05: just above the ground).
    pub min_pitch: f32,
    /// The highest pitch, in radians (default 1.35: not quite straight down).
    pub max_pitch: f32,
    /// The nearest distance (default 3).
    pub min_distance: f32,
    /// The farthest distance (default 30).
    pub max_distance: f32,
}

impl Default for OrbitLimits {
    fn default() -> OrbitLimits {
        OrbitLimits { min_pitch: 0.05, max_pitch: 1.35, min_distance: 3.0, max_distance: 30.0 }
    }
}

/// Drives an [`OrbitCamera`] from the canvas host's input ([`CanvasInput`]) and the frame's time.
///
/// - DRAG (a mouse button, a pen, or ONE finger): orbits. THE CONTENT FOLLOWS THE POINTER: a drag
///   to the right swings the scene to the right under the pointer (the camera moves the other
///   way: the yaw falls by `yaw_per_pixel` per CSS pixel), and a drag down tips the scene's near
///   side down (the camera rises: the pitch grows by `pitch_per_pixel`). The desktop viewer's
///   numbers are the defaults.
/// - WHEEL: one step towards the user zooms out by `wheel_zoom_out`, away from the user zooms in
///   by `wheel_zoom_in` (the desktop viewer's 1.1 and 0.9 per step).
/// - PINCH (TWO fingers): the distance follows the fingers' spread (apart zooms in), as on the
///   Android viewer.
/// - AUTO-ORBIT: until the user first drags, the yaw is `auto_orbit_start_yaw + auto_orbit_speed
///   * time` (the viewer's 0.7 + 0.18 t), from the frame's time - so it stops with the clock.
///   A drag stops it; [`OrbitController::set_auto_orbit`] starts it again from where the camera
///   is.
///
/// The pitch and the distance always stay within [`OrbitController::limits`]. Call
/// [`OrbitController::input`] from the layer's `input`, [`OrbitController::update`] once per
/// frame with the frame's time, then [`OrbitController::apply`] to the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitController {
    /// The orbit it drives.
    pub orbit: OrbitCamera,
    /// The limits of the pitch and the distance.
    pub limits: OrbitLimits,
    /// Radians of yaw per CSS pixel of a horizontal drag (default 0.008).
    pub yaw_per_pixel: f32,
    /// Radians of pitch per CSS pixel of a vertical drag (default 0.006).
    pub pitch_per_pixel: f32,
    /// The distance factor of one wheel step away from the user (default 0.9).
    pub wheel_zoom_in: f32,
    /// The distance factor of one wheel step towards the user (default 1.1).
    pub wheel_zoom_out: f32,
    /// The yaw of the auto-orbit at time 0 (default 0.7).
    pub auto_orbit_start_yaw: f32,
    /// Radians of yaw per second of the auto-orbit (default 0.18).
    pub auto_orbit_speed: f32,
    auto_orbit: bool,
    user_yaw: f32,
    time: f64,
    pointers: Vec<(i32, Vec2)>,
}

impl Default for OrbitController {
    fn default() -> OrbitController {
        OrbitController::new(OrbitCamera::default())
    }
}

impl OrbitController {
    /// A controller for `orbit` with the viewers' numbers, auto-orbiting; the orbit's pitch and
    /// distance are brought within the default limits.
    pub fn new(orbit: OrbitCamera) -> OrbitController {
        let mut controller = OrbitController {
            orbit,
            limits: OrbitLimits::default(),
            yaw_per_pixel: 0.008,
            pitch_per_pixel: 0.006,
            wheel_zoom_in: 0.9,
            wheel_zoom_out: 1.1,
            auto_orbit_start_yaw: 0.7,
            auto_orbit_speed: 0.18,
            auto_orbit: true,
            user_yaw: 0.0,
            time: 0.0,
            pointers: Vec::new(),
        };
        controller.clamp();
        controller
    }

    /// True while the camera orbits by itself.
    pub fn is_auto_orbiting(&self) -> bool {
        self.auto_orbit
    }

    /// Starts (true) or stops (false) the auto-orbit. Starting goes on from the camera's present
    /// yaw (no jump).
    pub fn set_auto_orbit(&mut self, on: bool) {
        if on && !self.auto_orbit {
            self.user_yaw = self.orbit.yaw - self.auto_yaw(self.time);
        }
        self.auto_orbit = on;
    }

    /// The number of pointers (buttons or fingers) pressed on the canvas now.
    pub fn pressed_pointer_count(&self) -> usize {
        self.pointers.len()
    }

    /// Hands one input event of the canvas to the controller; true when it changed the camera.
    pub fn input(&mut self, input: &CanvasInput) -> bool {
        match *input {
            CanvasInput::PointerDown { x, y, pointer_id } => {
                self.pointers.retain(|(id, _)| *id != pointer_id);
                self.pointers.push((pointer_id, Vec2::new(x as f32, y as f32)));
                false
            }
            CanvasInput::PointerMove { x, y, pressed, pointer_id, .. } => self.pointer_moved(pointer_id, Vec2::new(x as f32, y as f32), pressed),
            CanvasInput::PointerUp { pointer_id, .. } => {
                self.pointers.retain(|(id, _)| *id != pointer_id);
                false
            }
            CanvasInput::Wheel { delta_y, .. } => {
                if delta_y == 0.0 {
                    return false;
                }
                let factor = if delta_y > 0.0 { self.wheel_zoom_out } else { self.wheel_zoom_in };
                self.orbit.zoom(factor);
                self.clamp();
                true
            }
            CanvasInput::KeyDown { .. } | CanvasInput::KeyUp { .. } => false,
        }
    }

    /// Moves the auto-orbit to the frame's time (call it once per frame, before
    /// [`OrbitController::apply`]).
    pub fn update(&mut self, time: f64) {
        self.time = time;
        if self.auto_orbit {
            self.orbit.yaw = self.auto_yaw(time) + self.user_yaw;
        }
    }

    /// Points the camera from the orbit (C#: `OrbitCamera.Apply`).
    pub fn apply(&self, camera: &mut Camera) {
        self.orbit.apply(camera);
    }

    fn auto_yaw(&self, time: f64) -> f32 {
        self.auto_orbit_start_yaw + self.auto_orbit_speed * time as f32
    }

    fn pointer_moved(&mut self, id: i32, position: Vec2, pressed: bool) -> bool {
        let Some(index) = self.pointers.iter().position(|(known, _)| *known == id) else {
            // A pointer that went down elsewhere and is now pressed over the canvas: track it.
            if pressed {
                self.pointers.push((id, position));
            }
            return false;
        };
        if !pressed {
            self.pointers.remove(index);
            return false;
        }
        let previous = self.pointers[index].1;
        self.pointers[index].1 = position;
        match self.pointers.len() {
            1 => {
                let delta = position - previous;
                if delta == Vec2::ZERO {
                    return false;
                }
                self.auto_orbit = false;
                self.orbit.yaw -= delta.x * self.yaw_per_pixel;
                self.orbit.set_pitch(self.orbit.pitch() + delta.y * self.pitch_per_pixel);
                self.clamp();
                true
            }
            2 => {
                let other = self.pointers[1 - index].1;
                let before = previous.distance(other);
                let after = position.distance(other);
                if before <= 0.01 || after <= 0.01 || before == after {
                    return false;
                }
                self.orbit.zoom(before / after);
                self.clamp();
                true
            }
            _ => false,
        }
    }

    fn clamp(&mut self) {
        let limits = self.limits;
        self.orbit.set_pitch(self.orbit.pitch().clamp(limits.min_pitch, limits.max_pitch));
        self.orbit.set_distance(self.orbit.distance().clamp(limits.min_distance, limits.max_distance));
    }
}

#[cfg(test)]
mod tests {
    use codebrix_webgpu::CanvasInput;

    use super::{OrbitController, OrbitLimits};
    use crate::drawing3d::{Camera, OrbitCamera};

    fn down(id: i32, x: f64, y: f64) -> CanvasInput {
        CanvasInput::PointerDown { x, y, pointer_id: id }
    }

    fn drag(id: i32, x: f64, y: f64) -> CanvasInput {
        CanvasInput::PointerMove { x, y, dx: 0.0, dy: 0.0, pressed: true, pointer_id: id }
    }

    fn up(id: i32) -> CanvasInput {
        CanvasInput::PointerUp { x: 0.0, y: 0.0, pointer_id: id }
    }

    fn wheel(delta_y: f64) -> CanvasInput {
        CanvasInput::Wheel { delta_x: 0.0, delta_y }
    }

    #[test]
    fn it_auto_orbits_with_the_frame_time_until_a_drag() {
        let mut controller = OrbitController::default();
        controller.update(10.0);
        assert!((controller.orbit.yaw - (0.7 + 1.8)).abs() < 1e-5);
        controller.update(10.0);
        assert!((controller.orbit.yaw - 2.5).abs() < 1e-5, "a paused clock stands still");
        controller.input(&down(1, 100.0, 100.0));
        assert!(controller.input(&drag(1, 110.0, 100.0)));
        assert!(!controller.is_auto_orbiting());
        let yaw = controller.orbit.yaw;
        controller.update(20.0);
        assert_eq!(controller.orbit.yaw, yaw, "after a drag the time no longer turns it");
        controller.set_auto_orbit(true);
        controller.update(20.0);
        assert!((controller.orbit.yaw - yaw).abs() < 1e-5, "it goes on from where it is");
        controller.update(21.0);
        assert!((controller.orbit.yaw - (yaw + 0.18)).abs() < 1e-5);
    }

    #[test]
    fn a_rightward_drag_swings_the_scene_right_and_a_downward_drag_raises_the_camera() {
        let mut controller = OrbitController::default();
        controller.update(0.0);
        let (yaw, pitch) = (controller.orbit.yaw, controller.orbit.pitch());
        let mut camera = Camera::default();
        controller.apply(&mut camera);
        let before = camera.position;
        controller.input(&down(1, 100.0, 100.0));
        controller.input(&drag(1, 150.0, 100.0));
        assert!((controller.orbit.yaw - (yaw - 50.0 * 0.008)).abs() < 1e-5, "the yaw falls: the camera moves the opposite way");
        controller.apply(&mut camera);
        // Seen from the target, the camera went clockwise (to the viewer's left).
        assert!(before.x * camera.position.z - before.z * camera.position.x > 0.0);
        controller.input(&drag(1, 150.0, 120.0));
        assert!((controller.orbit.pitch() - (pitch + 20.0 * 0.006)).abs() < 1e-5);
    }

    #[test]
    fn the_pitch_stays_within_the_limits() {
        let mut controller = OrbitController::default();
        controller.input(&down(1, 0.0, 0.0));
        controller.input(&drag(1, 0.0, 10_000.0));
        assert_eq!(controller.orbit.pitch(), 1.35);
        controller.input(&drag(1, 0.0, -10_000.0));
        assert_eq!(controller.orbit.pitch(), 0.05);
    }

    #[test]
    fn the_wheel_zooms_within_the_limits() {
        let mut controller = OrbitController::default();
        assert_eq!(controller.orbit.distance(), 10.0);
        assert!(controller.input(&wheel(120.0)));
        assert!((controller.orbit.distance() - 11.0).abs() < 1e-5, "towards the user: out");
        controller.input(&wheel(-120.0));
        assert!((controller.orbit.distance() - 9.9).abs() < 1e-4, "away from the user: in");
        for _ in 0..100 {
            controller.input(&wheel(120.0));
        }
        assert_eq!(controller.orbit.distance(), 30.0);
        for _ in 0..100 {
            controller.input(&wheel(-120.0));
        }
        assert_eq!(controller.orbit.distance(), 3.0);
        assert!(!controller.input(&wheel(0.0)));
    }

    #[test]
    fn a_pinch_zooms_and_does_not_orbit() {
        let mut controller = OrbitController::default();
        controller.update(0.0);
        let yaw = controller.orbit.yaw;
        controller.input(&down(1, 100.0, 100.0));
        controller.input(&down(2, 200.0, 100.0));
        assert_eq!(controller.pressed_pointer_count(), 2);
        // Spread from 100 to 200 pixels: half the distance.
        assert!(controller.input(&drag(2, 300.0, 100.0)));
        assert!((controller.orbit.distance() - 5.0).abs() < 1e-4);
        assert_eq!(controller.orbit.yaw, yaw);
        assert!(controller.is_auto_orbiting());
        // Pinched together: farther, up to the limit.
        controller.input(&drag(2, 100.5, 100.0));
        assert_eq!(controller.orbit.distance(), 30.0);
        controller.input(&up(2));
        assert_eq!(controller.pressed_pointer_count(), 1);
        assert!(controller.input(&drag(1, 90.0, 100.0)), "one finger left: it orbits again");
    }

    #[test]
    fn a_move_without_a_press_does_nothing() {
        let mut controller = OrbitController::default();
        let hover = CanvasInput::PointerMove { x: 5.0, y: 5.0, dx: 5.0, dy: 5.0, pressed: false, pointer_id: 1 };
        assert!(!controller.input(&hover));
        assert!(!controller.input(&CanvasInput::KeyDown { key: "a".to_string() }));
        assert_eq!(controller.pressed_pointer_count(), 0);
    }

    #[test]
    fn a_new_controller_brings_the_orbit_within_the_limits() {
        let mut orbit = OrbitCamera::new();
        orbit.set_distance(100.0);
        orbit.set_pitch(-1.0);
        let controller = OrbitController::new(orbit);
        assert_eq!((controller.orbit.distance(), controller.orbit.pitch()), (30.0, 0.05));
        assert_eq!(controller.limits, OrbitLimits { min_pitch: 0.05, max_pitch: 1.35, min_distance: 3.0, max_distance: 30.0 });
    }
}
