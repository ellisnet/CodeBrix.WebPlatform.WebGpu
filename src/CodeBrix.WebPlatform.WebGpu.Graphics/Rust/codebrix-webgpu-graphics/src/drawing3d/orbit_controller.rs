//! [`OrbitController`]: an [`OrbitCamera`] driven by the canvas host's input - drag to orbit,
//! right/middle/Shift drag to pan, wheel to zoom, one finger to orbit and two to pan and
//! pinch-zoom - within [`OrbitLimits`], with the model viewer's auto-orbit until the user drags,
//! and a helper that frames a bounding box. The C# library leaves this to each viewer (its
//! desktop and Android hosts); here it is one type, with the viewers' numbers as defaults.

use std::cell::Cell;

use codebrix_webgpu::{CanvasInput, ModifierKeys, PointerButtons};
use glam::{Vec2, Vec3};

use crate::drawing3d::{Camera, OrbitCamera, ProjectionKind};

/// The limits an [`OrbitController`] holds the camera to (the model viewers' on Android, which
/// suit a pointer and a touch screen).
///
/// The pitch limits may be NEGATIVE (a camera that may look up from below the target); the
/// orbit itself never goes beyond [`OrbitCamera::MAX_PITCH`] either way.
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
/// - DRAG (the left mouse button, a pen's tip, or ONE finger): orbits. THE CONTENT FOLLOWS THE
///   POINTER: a drag to the right swings the scene to the right under the pointer (the camera
///   moves the other way: the yaw falls by `yaw_per_pixel` per CSS pixel), and a drag down tips
///   the scene's near side down (the camera rises: the pitch grows by `pitch_per_pixel`). The
///   desktop viewer's numbers are the defaults.
/// - PAN (not in C#; on while `pan_enabled`, the default): a drag with the RIGHT or the MIDDLE
///   mouse button, or with SHIFT held (left button, pen tip or one finger), or a pen with its
///   barrel button pressed, moves the target in the screen plane. The content follows the
///   pointer exactly at the target's depth, at any zoom: the move is scaled by the distance and
///   the camera's field of view (or orthographic height) over the canvas's height, which the
///   controller learns from [`OrbitController::set_viewport`] - call it once per frame (until
///   then it assumes the default camera on a canvas 600 CSS pixels high). With `pan_bounds` set,
///   the target never leaves that box.
/// - WHICH ONE A DRAG DOES is decided at EVERY move from the buttons and the Shift key held at
///   that move: pressing or releasing a second button, or Shift, in the middle of a drag
///   switches between orbit and pan from the next move, without a jump. Ctrl, Alt and Meta
///   change nothing (they are the page's and the browser's). The back and forward buttons alone
///   orbit, as any button did before panning existed. With `pan_enabled` false every drag
///   orbits, exactly as before panning existed.
/// - WHEEL: one step towards the user zooms out by `wheel_zoom_out`, away from the user zooms in
///   by `wheel_zoom_in` (the desktop viewer's 1.1 and 0.9 per step). (A trackpad's pinch arrives
///   from the browser as a wheel and zooms the same way; a two-finger trackpad scroll does
///   too.)
/// - TWO fingers: the distance follows the fingers' spread (apart zooms in), as on the Android
///   viewer, AND - while panning is on - the target follows their midpoint, both at once, as on
///   a map. (The zoom is about the target, the middle of the view, not about the fingers.)
/// - ORTHOGRAPHIC ZOOM (not in C#): a distance alone does not change an orthographic picture, so
///   for a camera with [`ProjectionKind::Orthographic`], [`OrbitController::apply`] also sets
///   the camera's `orthographic_height` IN PROPORTION TO THE DISTANCE: a wheel step or a pinch
///   scales the height by the same factor as the distance, the picture grows and shrinks as a
///   perspective one does, and the distance limits bound it (no limits of its own). The ratio
///   of height to distance is taken from the camera at the first `apply` to it as an
///   orthographic camera (so the picture does not jump), set by
///   [`OrbitController::frame_box`], and taken again whenever the application has changed the
///   height itself since the last `apply` (its value wins, at the present distance). The camera
///   still moves with the distance, as for perspective, so the near and far planes clip as
///   they would there. A perspective camera's `orthographic_height` is never touched.
/// - AUTO-ORBIT: until the user first drags, the yaw is
///   `auto_orbit_start_yaw + auto_orbit_speed * time` (the viewer's 0.7 + 0.18 t), from the
///   frame's time - so it stops with the clock.
///   A drag stops it, and so does a pan (also a two-finger one whose midpoint moves); the wheel
///   and a pinch with panning off do not. [`OrbitController::set_auto_orbit`] starts it again
///   from where the camera is, around the (possibly panned) target.
///
/// The pitch and the distance always stay within [`OrbitController::limits`], and the target
/// within `pan_bounds` when it is set. Call [`OrbitController::input`] from the layer's `input`;
/// once per frame [`OrbitController::update`] with the frame's time,
/// [`OrbitController::set_viewport`] with the camera and the canvas's CSS height, then
/// [`OrbitController::apply`] to the camera. [`OrbitController::frame_box`] aims the controller
/// at a model's bounds.
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
    /// Whether the user can pan: right/middle-button drags, Shift drags and two-finger drags
    /// move the target (default true). False gives the controller without panning: every drag
    /// orbits and two fingers only zoom. Not in C#.
    pub pan_enabled: bool,
    /// The box the target must stay inside, as its minimum and maximum corner (default `None`:
    /// anywhere). [`OrbitController::frame_box`] sets it to the framed box. The target is held
    /// inside it after every input and every framing; set it to `None` to pan freely. Not in C#.
    pub pan_bounds: Option<(Vec3, Vec3)>,
    auto_orbit: bool,
    user_yaw: f32,
    time: f64,
    pointers: Vec<(i32, Vec2)>,
    view: Camera,
    canvas_height: f32,
    orthographic: Cell<OrthographicZoom>,
}

/// How an orthographic camera's height follows the distance (see "ORTHOGRAPHIC ZOOM" on
/// [`OrbitController`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct OrthographicZoom {
    /// The height per unit of distance; `None` until taken from a camera or set by framing.
    height_per_distance: Option<f32>,
    /// The height the last `apply` wrote (`None` after framing: write without taking it again).
    written: Option<f32>,
}

impl Default for OrbitController {
    fn default() -> OrbitController {
        OrbitController::new(OrbitCamera::default())
    }
}

impl OrbitController {
    /// The canvas height (CSS pixels) a pan assumes until [`OrbitController::set_viewport`] is
    /// called. Not in C#.
    pub const ASSUMED_CANVAS_HEIGHT: f32 = 600.0;

    /// The nearest distance [`OrbitController::frame_box`] allows, as a multiple of the framing
    /// distance (0.3: the default limits' 3 for the default distance of 10). Not in C#.
    pub const FRAME_MIN_DISTANCE_FACTOR: f32 = 0.3;

    /// The farthest distance [`OrbitController::frame_box`] allows, as a multiple of the framing
    /// distance (3: the default limits' 30 for the default distance of 10). Not in C#.
    pub const FRAME_MAX_DISTANCE_FACTOR: f32 = 3.0;

    /// A controller for `orbit` with the viewers' numbers and the default [`OrbitLimits`],
    /// auto-orbiting, panning on; the orbit's pitch and distance are brought within the limits.
    pub fn new(orbit: OrbitCamera) -> OrbitController {
        OrbitController::with_limits(orbit, OrbitLimits::default())
    }

    /// A controller for `orbit` held to `limits` (otherwise as [`OrbitController::new`]); the
    /// orbit's pitch and distance are brought within them at once. Not in C#.
    pub fn with_limits(orbit: OrbitCamera, limits: OrbitLimits) -> OrbitController {
        let mut controller = OrbitController {
            orbit,
            limits,
            yaw_per_pixel: 0.008,
            pitch_per_pixel: 0.006,
            wheel_zoom_in: 0.9,
            wheel_zoom_out: 1.1,
            auto_orbit_start_yaw: 0.7,
            auto_orbit_speed: 0.18,
            pan_enabled: true,
            pan_bounds: None,
            auto_orbit: true,
            user_yaw: 0.0,
            time: 0.0,
            pointers: Vec::new(),
            view: Camera::default(),
            canvas_height: OrbitController::ASSUMED_CANVAS_HEIGHT,
            orthographic: Cell::new(OrthographicZoom::default()),
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

    /// Tells the controller how the scene is seen, so that a pan moves the content exactly with
    /// the pointer: the camera it drives (its projection, field of view or orthographic height -
    /// not its position) and the canvas's height in CSS pixels (in a layer:
    /// `frame.info.height as f32 / frame.info.pixel_ratio as f32`; 0 or less counts as 1). Call
    /// it once per frame, or whenever either changes. Not in C#.
    pub fn set_viewport(&mut self, camera: &Camera, canvas_height: f32) {
        self.view = *camera;
        self.canvas_height = if canvas_height > 0.0 { canvas_height } else { 1.0 };
    }

    /// Aims the controller at the axis-aligned box from `min` to `max` (a model's bounds, in
    /// world space), seen by `camera` on a view of `aspect_ratio` (width / height). Not in C#.
    ///
    /// - The target becomes the box's centre, and `pan_bounds` the box (a pan cannot lose the
    ///   model).
    /// - PERSPECTIVE: the distance becomes the one at which the box's bounding sphere fits in
    ///   view ([`Camera::fit_distance`]), so the whole box stays in view from every direction the
    ///   user can orbit to; the distance limits become that distance times
    ///   [`OrbitController::FRAME_MIN_DISTANCE_FACTOR`] and
    ///   [`OrbitController::FRAME_MAX_DISTANCE_FACTOR`].
    /// - ORTHOGRAPHIC: the view's HEIGHT fits the sphere - the next [`OrbitController::apply`]
    ///   sets `orthographic_height` to [`Camera::fit_orthographic_height`] - and the distance is
    ///   [`Camera::fit_distance`] divided by `FRAME_MIN_DISTANCE_FACTOR`, so that zooming in to
    ///   the nearest limit (`fit_distance` itself) still keeps the box in front of the near
    ///   plane; the limits are the same factors of the distance, so the zoom scales the height
    ///   between the same factors as a perspective zoom scales the picture.
    /// - The yaw, the pitch, the pitch limits and the auto-orbit are left as they are; the
    ///   camera is remembered as by [`OrbitController::set_viewport`] (not the canvas height).
    /// - The corners may come in either order; a box of zero size is framed as a sphere of the
    ///   camera's near-plane radius; a model smaller than the orbit's own minimum distance
    ///   (0.01) lowers it. Returns false, changing nothing, when a corner is not finite.
    /// - The camera's planes are not changed: keep `far_plane` beyond the farthest distance plus
    ///   the box's radius.
    pub fn frame_box(&mut self, min: Vec3, max: Vec3, camera: &Camera, aspect_ratio: f32) -> bool {
        if !min.is_finite() || !max.is_finite() {
            return false;
        }
        let (low, high) = (min.min(max), min.max(max));
        let radius = ((high - low) * 0.5).length();
        let radius = if radius > 0.0 { radius } else { camera.near_plane.abs() };
        let distance = match camera.projection {
            ProjectionKind::Perspective => camera.fit_distance(radius, aspect_ratio),
            ProjectionKind::Orthographic => {
                let distance = camera.fit_distance(radius, aspect_ratio) / OrbitController::FRAME_MIN_DISTANCE_FACTOR;
                let height = camera.fit_orthographic_height(radius, aspect_ratio);
                self.orthographic.set(OrthographicZoom { height_per_distance: Some(height / distance), written: None });
                distance
            }
        };
        let nearest = distance * OrbitController::FRAME_MIN_DISTANCE_FACTOR;
        self.limits.min_distance = nearest;
        self.limits.max_distance = distance * OrbitController::FRAME_MAX_DISTANCE_FACTOR;
        if nearest < self.orbit.min_distance() {
            // A model smaller than the orbit's own floor (0.01 by default).
            self.orbit.set_min_distance(nearest);
        }
        self.orbit.target = (low + high) * 0.5;
        self.orbit.set_distance(distance);
        self.pan_bounds = Some((low, high));
        self.view = *camera;
        self.clamp();
        true
    }

    /// Hands one input event of the canvas to the controller; true when it changed the camera.
    pub fn input(&mut self, input: &CanvasInput) -> bool {
        match *input {
            CanvasInput::PointerDown { x, y, pointer_id, .. } => {
                self.pointers.retain(|(id, _)| *id != pointer_id);
                self.pointers.push((pointer_id, Vec2::new(x as f32, y as f32)));
                false
            }
            CanvasInput::PointerMove { x, y, pressed, pointer_id, buttons, modifiers, .. } => self.pointer_moved(pointer_id, Vec2::new(x as f32, y as f32), pressed, buttons, modifiers),
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

    /// Points the camera from the orbit (C#: `OrbitCamera.Apply`); for an orthographic camera
    /// it also sets `orthographic_height` in proportion to the distance, so the zoom shows (see
    /// "ORTHOGRAPHIC ZOOM" on [`OrbitController`]; not in C#).
    pub fn apply(&self, camera: &mut Camera) {
        self.orbit.apply(camera);
        if camera.projection != ProjectionKind::Orthographic {
            return;
        }
        let distance = self.orbit.distance();
        let mut zoom = self.orthographic.get();
        let take_from_camera = match (zoom.height_per_distance, zoom.written) {
            (None, _) => true,
            (Some(_), Some(written)) => camera.orthographic_height != written,
            (Some(_), None) => false,
        };
        if take_from_camera {
            let height = camera.orthographic_height;
            zoom.height_per_distance = if height > 0.0 && height.is_finite() && distance > 0.0 { Some(height / distance) } else { None };
        }
        if let Some(ratio) = zoom.height_per_distance {
            let height = ratio * distance;
            camera.orthographic_height = height;
            zoom.written = Some(height);
        } else {
            zoom.written = None;
        }
        self.orthographic.set(zoom);
    }

    fn auto_yaw(&self, time: f64) -> f32 {
        self.auto_orbit_start_yaw + self.auto_orbit_speed * time as f32
    }

    /// Whether a one-pointer drag with these buttons and keys pans (rather than orbits).
    fn drag_pans(&self, buttons: PointerButtons, modifiers: ModifierKeys) -> bool {
        self.pan_enabled && (buttons.secondary() || buttons.middle() || modifiers.shift)
    }

    fn pointer_moved(&mut self, id: i32, position: Vec2, pressed: bool, buttons: PointerButtons, modifiers: ModifierKeys) -> bool {
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
                if self.drag_pans(buttons, modifiers) {
                    return self.pan(delta);
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
                let mut changed = false;
                if before > 0.01 && after > 0.01 && before != after {
                    self.orbit.zoom(before / after);
                    self.clamp();
                    changed = true;
                }
                if self.pan_enabled {
                    // The midpoint of the two fingers moves by half of this finger's move.
                    changed |= self.pan((position - previous) * 0.5);
                }
                changed
            }
            _ => false,
        }
    }

    /// Moves the target so the content at its depth follows a pointer move of `delta` CSS pixels.
    fn pan(&mut self, delta: Vec2) -> bool {
        if delta == Vec2::ZERO {
            return false;
        }
        let per_pixel = match self.view.projection {
            ProjectionKind::Perspective => 2.0 * self.orbit.distance() * (self.view.field_of_view * 0.5).tan() / self.canvas_height,
            // The height the next apply gives the camera (the zoom has moved it since).
            ProjectionKind::Orthographic => self.orthographic.get().height_per_distance.map_or(self.view.orthographic_height, |ratio| ratio * self.orbit.distance()) / self.canvas_height,
        };
        if !per_pixel.is_finite() {
            return false;
        }
        let (right, up) = self.screen_axes();
        // The content follows the pointer, so the target (the middle of the view) moves the
        // other way: left for a move to the right, up for a move down (y grows downwards).
        self.orbit.target += (up * delta.y - right * delta.x) * per_pixel;
        self.auto_orbit = false;
        self.clamp();
        true
    }

    /// The world directions of the screen's right and up for the orbit's yaw and pitch.
    fn screen_axes(&self) -> (Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.orbit.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.orbit.pitch().sin_cos();
        let right = Vec3::new(cos_yaw, 0.0, -sin_yaw);
        let back = Vec3::new(cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw);
        (right, back.cross(right))
    }

    fn clamp(&mut self) {
        let limits = self.limits;
        self.orbit.set_pitch(self.orbit.pitch().clamp(limits.min_pitch, limits.max_pitch));
        self.orbit.set_distance(self.orbit.distance().clamp(limits.min_distance, limits.max_distance));
        if let Some((min, max)) = self.pan_bounds {
            self.orbit.target = self.orbit.target.clamp(min.min(max), min.max(max));
        }
    }
}

#[cfg(test)]
mod tests {
    use codebrix_webgpu::{CanvasInput, ModifierKeys, PointerButton, PointerButtons, PointerKind};
    use glam::Vec3;

    use super::{OrbitController, OrbitLimits};
    use crate::drawing3d::{Camera, OrbitCamera, ProjectionKind};

    fn down(id: i32, x: f64, y: f64) -> CanvasInput {
        CanvasInput::PointerDown { x, y, pointer_id: id, button: PointerButton::Primary, buttons: PointerButtons::PRIMARY, kind: PointerKind::Mouse, modifiers: ModifierKeys::NONE, tap_count: 1 }
    }

    fn drag(id: i32, x: f64, y: f64) -> CanvasInput {
        drag_with(id, x, y, PointerButtons::PRIMARY, ModifierKeys::NONE, PointerKind::Mouse)
    }

    fn drag_with(id: i32, x: f64, y: f64, buttons: PointerButtons, modifiers: ModifierKeys, kind: PointerKind) -> CanvasInput {
        CanvasInput::PointerMove { x, y, dx: 0.0, dy: 0.0, pressed: !buttons.is_empty(), pointer_id: id, buttons, kind, modifiers }
    }

    fn touch_down(id: i32, x: f64, y: f64) -> CanvasInput {
        CanvasInput::PointerDown { x, y, pointer_id: id, button: PointerButton::Primary, buttons: PointerButtons::PRIMARY, kind: PointerKind::Touch, modifiers: ModifierKeys::NONE, tap_count: 1 }
    }

    fn touch_drag(id: i32, x: f64, y: f64) -> CanvasInput {
        drag_with(id, x, y, PointerButtons::PRIMARY, ModifierKeys::NONE, PointerKind::Touch)
    }

    fn up(id: i32) -> CanvasInput {
        CanvasInput::PointerUp { x: 0.0, y: 0.0, pointer_id: id, button: PointerButton::Primary, buttons: PointerButtons::NONE, kind: PointerKind::Mouse, modifiers: ModifierKeys::NONE }
    }

    fn wheel(delta_y: f64) -> CanvasInput {
        CanvasInput::Wheel { delta_x: 0.0, delta_y }
    }

    const SHIFT: ModifierKeys = ModifierKeys { shift: true, ctrl: false, alt: false, meta: false };

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
    fn negative_pitch_limits_let_the_camera_look_up_from_below() {
        let limits = OrbitLimits { min_pitch: -0.8, max_pitch: -0.1, min_distance: 1.0, max_distance: 5.0 };
        let mut controller = OrbitController::with_limits(OrbitCamera::new(), limits);
        assert_eq!(controller.limits, limits);
        // The default orbit (pitch 0.4, distance 10) is brought within them at once.
        assert_eq!((controller.orbit.pitch(), controller.orbit.distance()), (-0.1, 5.0));
        let mut camera = Camera::default();
        controller.apply(&mut camera);
        assert!(camera.position.y < 0.0, "below the target");
        controller.input(&down(1, 0.0, 0.0));
        controller.input(&drag(1, 0.0, -100.0));
        assert!((controller.orbit.pitch() - (-0.1 - 100.0 * 0.006)).abs() < 1e-5, "a drag up lowers it further");
        controller.input(&drag(1, 0.0, -10_000.0));
        assert_eq!(controller.orbit.pitch(), -0.8);
        controller.input(&drag(1, 0.0, 10_000.0));
        assert_eq!(controller.orbit.pitch(), -0.1);
        // Limits beyond straight down are still held to the orbit's own maximum.
        let steep = OrbitController::with_limits(OrbitCamera::new(), OrbitLimits { min_pitch: -3.0, max_pitch: -2.0, ..limits });
        assert_eq!(steep.orbit.pitch(), -OrbitCamera::MAX_PITCH);
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
    fn a_pinch_without_panning_zooms_and_does_not_orbit() {
        let mut controller = OrbitController { pan_enabled: false, ..OrbitController::default() };
        controller.update(0.0);
        let yaw = controller.orbit.yaw;
        controller.input(&touch_down(1, 100.0, 100.0));
        controller.input(&touch_down(2, 200.0, 100.0));
        assert_eq!(controller.pressed_pointer_count(), 2);
        // Spread from 100 to 200 pixels: half the distance.
        assert!(controller.input(&touch_drag(2, 300.0, 100.0)));
        assert!((controller.orbit.distance() - 5.0).abs() < 1e-4);
        assert_eq!(controller.orbit.yaw, yaw);
        assert_eq!(controller.orbit.target, Vec3::ZERO);
        assert!(controller.is_auto_orbiting());
        // Pinched together: farther, up to the limit.
        controller.input(&touch_drag(2, 100.5, 100.0));
        assert_eq!(controller.orbit.distance(), 30.0);
        controller.input(&up(2));
        assert_eq!(controller.pressed_pointer_count(), 1);
        assert!(controller.input(&touch_drag(1, 90.0, 100.0)), "one finger left: it orbits again");
    }

    #[test]
    fn two_fingers_pan_by_their_midpoint_while_the_spread_zooms() {
        let mut controller = OrbitController::default();
        controller.update(0.0);
        let yaw = controller.orbit.yaw;
        controller.input(&touch_down(1, 100.0, 100.0));
        controller.input(&touch_down(2, 200.0, 100.0));
        // Both fingers move down 40 pixels, one after the other: the spread stays, no zoom.
        assert!(controller.input(&touch_drag(1, 100.0, 140.0)));
        assert!(controller.input(&touch_drag(2, 200.0, 140.0)));
        assert!((controller.orbit.distance() - 10.0).abs() < 1e-4);
        assert!(controller.orbit.target.y > 0.0, "content down: the target goes up");
        assert_eq!(controller.orbit.yaw, yaw, "no orbit");
        assert!(!controller.is_auto_orbiting(), "a pan stops the auto-orbit");
        // A spread with a moving midpoint does both.
        let target = controller.orbit.target;
        assert!(controller.input(&touch_drag(2, 300.0, 140.0)));
        assert!((controller.orbit.distance() - 5.0).abs() < 1e-4);
        assert_ne!(controller.orbit.target, target);
    }

    #[test]
    fn right_middle_and_shift_drags_pan_and_the_content_follows_the_pointer() {
        for (buttons, modifiers, kind) in [
            (PointerButtons::SECONDARY, ModifierKeys::NONE, PointerKind::Mouse),
            (PointerButtons::MIDDLE, ModifierKeys::NONE, PointerKind::Mouse),
            (PointerButtons::PRIMARY, SHIFT, PointerKind::Mouse),
            (PointerButtons::PRIMARY.with(PointerButtons::SECONDARY), ModifierKeys::NONE, PointerKind::Pen), // tip + barrel button
            (PointerButtons::PRIMARY, SHIFT, PointerKind::Touch),
        ] {
            for (projection, canvas_height) in [(ProjectionKind::Perspective, 480.0), (ProjectionKind::Orthographic, 300.0)] {
                let mut controller = OrbitController::default();
                controller.update(3.0);
                let mut camera = Camera { projection, ..Camera::default() };
                controller.apply(&mut camera);
                controller.set_viewport(&camera, canvas_height);
                let (yaw, pitch, distance) = (controller.orbit.yaw, controller.orbit.pitch(), controller.orbit.distance());
                // A point at the target's depth: the old target itself.
                let anchor = controller.orbit.target;
                let width = canvas_height * 1.5;
                let before = camera.world_to_canvas(anchor, width, canvas_height);
                controller.input(&down(7, 100.0, 100.0));
                assert!(controller.input(&drag_with(7, 130.0, 80.0, buttons, modifiers, kind)), "{buttons:?} {kind:?}");
                controller.apply(&mut camera);
                let after = camera.world_to_canvas(anchor, width, canvas_height);
                assert!((after.x - before.x - 30.0).abs() < 1e-2 && (after.y - before.y + 20.0).abs() < 1e-2, "{buttons:?} {projection:?}: {before:?} -> {after:?}");
                assert_eq!((controller.orbit.yaw, controller.orbit.pitch(), controller.orbit.distance()), (yaw, pitch, distance), "a pan does not orbit or zoom");
                assert!(!controller.is_auto_orbiting());
            }
        }
    }

    #[test]
    fn ctrl_alt_and_meta_change_nothing_and_back_alone_orbits() {
        for modifiers in [
            ModifierKeys { ctrl: true, ..ModifierKeys::NONE },
            ModifierKeys { alt: true, ..ModifierKeys::NONE },
            ModifierKeys { meta: true, ..ModifierKeys::NONE },
        ] {
            let mut controller = OrbitController::default();
            controller.input(&down(1, 0.0, 0.0));
            assert!(controller.input(&drag_with(1, 10.0, 0.0, PointerButtons::PRIMARY, modifiers, PointerKind::Mouse)));
            assert_eq!(controller.orbit.target, Vec3::ZERO, "{modifiers:?} orbits");
        }
        let mut controller = OrbitController::default();
        let yaw = controller.orbit.yaw;
        controller.input(&down(1, 0.0, 0.0));
        controller.input(&drag_with(1, 10.0, 0.0, PointerButtons::BACK, ModifierKeys::NONE, PointerKind::Mouse));
        assert_ne!(controller.orbit.yaw, yaw);
        assert_eq!(controller.orbit.target, Vec3::ZERO);
    }

    #[test]
    fn a_button_or_shift_changed_mid_drag_switches_mode_without_a_jump() {
        let mut controller = OrbitController::default();
        controller.update(0.0);
        controller.input(&down(1, 100.0, 100.0));
        controller.input(&drag(1, 110.0, 100.0));
        let yaw = controller.orbit.yaw;
        // The right button joins: the browser reports it as a move with both buttons.
        let both = PointerButtons::PRIMARY.with(PointerButtons::SECONDARY);
        controller.input(&drag_with(1, 110.0, 100.0, both, ModifierKeys::NONE, PointerKind::Mouse));
        assert_eq!(controller.orbit.target, Vec3::ZERO, "no move, no jump");
        controller.input(&drag_with(1, 120.0, 100.0, both, ModifierKeys::NONE, PointerKind::Mouse));
        assert_eq!(controller.orbit.yaw, yaw, "it pans now");
        let target = controller.orbit.target;
        assert_ne!(target, Vec3::ZERO);
        // The right button goes up again, and later Shift is pressed during the same drag.
        controller.input(&drag(1, 130.0, 100.0));
        assert_ne!(controller.orbit.yaw, yaw, "it orbits again");
        assert_eq!(controller.orbit.target, target);
        let yaw = controller.orbit.yaw;
        controller.input(&drag_with(1, 140.0, 100.0, PointerButtons::PRIMARY, SHIFT, PointerKind::Mouse));
        assert_eq!(controller.orbit.yaw, yaw);
        assert_ne!(controller.orbit.target, target);
    }

    #[test]
    fn with_panning_off_every_drag_orbits() {
        let mut controller = OrbitController { pan_enabled: false, ..OrbitController::default() };
        let yaw = controller.orbit.yaw;
        controller.input(&down(1, 0.0, 0.0));
        controller.input(&drag_with(1, 10.0, 0.0, PointerButtons::SECONDARY, SHIFT, PointerKind::Mouse));
        assert_ne!(controller.orbit.yaw, yaw);
        assert_eq!(controller.orbit.target, Vec3::ZERO);
    }

    #[test]
    fn the_pan_bounds_hold_the_target() {
        let mut controller = OrbitController { pan_bounds: Some((Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 2.0, 1.0))), ..OrbitController::default() };
        controller.input(&down(1, 0.0, 0.0));
        controller.input(&drag_with(1, -100_000.0, 100_000.0, PointerButtons::SECONDARY, ModifierKeys::NONE, PointerKind::Mouse));
        let target = controller.orbit.target;
        assert!(target.cmpge(Vec3::new(-1.0, -1.0, -1.0)).all() && target.cmple(Vec3::new(1.0, 2.0, 1.0)).all(), "{target}");
        assert_eq!(target.y, 2.0);
        controller.pan_bounds = None;
        controller.input(&drag_with(1, -200_000.0, 200_000.0, PointerButtons::SECONDARY, ModifierKeys::NONE, PointerKind::Mouse));
        assert!(controller.orbit.target.y > 2.0, "no bounds: anywhere");
    }

    #[test]
    fn frame_box_aims_at_the_box_and_scales_the_limits_to_it() {
        let mut controller = OrbitController::default();
        let camera = Camera::default();
        let (min, max) = (Vec3::new(8.0, 0.0, -2.0), Vec3::new(12.0, 3.0, 2.0)); // half-diagonal sqrt(4 + 2.25 + 4) = 3.2016
        assert!(controller.frame_box(max, min, &camera, 16.0 / 9.0), "corners in either order");
        let radius = ((max - min) * 0.5).length();
        let distance = camera.fit_distance(radius, 16.0 / 9.0);
        assert!((distance - 2.0 * radius).abs() < 1e-4);
        assert_eq!(controller.orbit.target, Vec3::new(10.0, 1.5, 0.0));
        assert!((controller.orbit.distance() - distance).abs() < 1e-4);
        assert!((controller.limits.min_distance - 0.3 * distance).abs() < 1e-4);
        assert!((controller.limits.max_distance - 3.0 * distance).abs() < 1e-4);
        assert_eq!(controller.pan_bounds, Some((min, max)));
        assert_eq!((controller.limits.min_pitch, controller.limits.max_pitch), (0.05, 1.35), "pitch limits kept");
        // Every corner is in view from the framed position, whatever the yaw.
        for yaw in [0.0f32, 1.0, 2.5, 4.0] {
            controller.orbit.yaw = yaw;
            let mut seen = camera;
            controller.apply(&mut seen);
            for corner in [min, max, Vec3::new(min.x, max.y, min.z), Vec3::new(max.x, min.y, max.z)] {
                assert!(seen.world_to_canvas(corner, 1600.0, 900.0).in_view, "yaw {yaw}: {corner}");
            }
        }
        // A tiny model is framed just beyond the near plane; one below the orbit's own floor of
        // 0.01 lowers that floor; a non-finite box is refused.
        assert!(controller.frame_box(Vec3::ZERO, Vec3::splat(0.001), &camera, 1.0));
        assert!((controller.orbit.distance() - (0.001 * 3f32.sqrt() * 0.5 + 0.1)).abs() < 1e-5);
        let close_up = Camera { near_plane: 0.0001, ..camera };
        assert!(controller.frame_box(Vec3::ZERO, Vec3::splat(0.001), &close_up, 1.0));
        assert!(controller.limits.min_distance < 0.01);
        assert!((controller.orbit.min_distance() - controller.limits.min_distance).abs() < 1e-7);
        assert!((controller.orbit.distance() - 0.001 * 3f32.sqrt()).abs() < 1e-5);
        let before = controller.clone();
        assert!(!controller.frame_box(Vec3::ZERO, Vec3::splat(f32::INFINITY), &camera, 1.0));
        assert_eq!(controller, before);
        // Orthographic: the nearest limit keeps the box in front of the near plane.
        let ortho = Camera { projection: ProjectionKind::Orthographic, ..camera };
        controller.frame_box(min, max, &ortho, 1.0);
        assert!((controller.limits.min_distance - (radius + ortho.near_plane)).abs() < 1e-4);
    }

    fn orthographic() -> Camera {
        Camera { projection: ProjectionKind::Orthographic, ..Camera::default() }
    }

    #[test]
    fn an_orthographic_wheel_zoom_scales_the_view_height_within_the_distance_limits() {
        let mut controller = OrbitController::default(); // distance 10, limits 3..30
        let mut camera = orthographic(); // height 10
        controller.apply(&mut camera);
        assert_eq!(camera.orthographic_height, 10.0, "the first apply takes the ratio: no jump");
        controller.input(&wheel(120.0));
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 11.0).abs() < 1e-4, "out: the view grows by the distance's factor");
        controller.input(&wheel(-120.0));
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 9.9).abs() < 1e-4);
        for _ in 0..100 {
            controller.input(&wheel(120.0));
        }
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 30.0).abs() < 1e-3, "held by max_distance");
        for _ in 0..100 {
            controller.input(&wheel(-120.0));
        }
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 3.0).abs() < 1e-4, "held by min_distance");
        // The camera still moves with the distance, as for perspective.
        assert!((camera.position.distance(camera.target) - 3.0).abs() < 1e-4);
    }

    #[test]
    fn an_orthographic_pinch_scales_the_view_height() {
        let mut controller = OrbitController { pan_enabled: false, ..OrbitController::default() };
        let mut camera = Camera { orthographic_height: 6.0, ..orthographic() };
        controller.apply(&mut camera);
        controller.input(&touch_down(1, 100.0, 100.0));
        controller.input(&touch_down(2, 200.0, 100.0));
        controller.input(&touch_drag(2, 300.0, 100.0)); // spread doubled: half
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 3.0).abs() < 1e-4);
        controller.input(&touch_drag(2, 100.5, 100.0)); // pinched together: up to the limit (distance 30)
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 18.0).abs() < 1e-3);
    }

    #[test]
    fn the_applications_own_orthographic_height_wins_and_the_zoom_goes_on_from_it() {
        let mut controller = OrbitController::default();
        let mut camera = orthographic();
        controller.apply(&mut camera);
        camera.orthographic_height = 4.0;
        controller.apply(&mut camera);
        assert_eq!(camera.orthographic_height, 4.0);
        controller.input(&wheel(120.0));
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 4.4).abs() < 1e-4);
        // Switching to perspective and back keeps the zoom made meanwhile.
        camera.projection = ProjectionKind::Perspective;
        controller.input(&wheel(120.0));
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 4.4).abs() < 1e-4, "a perspective apply leaves the height alone");
        camera.projection = ProjectionKind::Orthographic;
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 4.84).abs() < 1e-4);
    }

    #[test]
    fn a_perspective_camera_is_applied_as_before() {
        let mut controller = OrbitController::default();
        let mut camera = Camera { orthographic_height: 7.0, ..Camera::default() };
        controller.update(2.0);
        controller.input(&wheel(120.0));
        controller.apply(&mut camera);
        let mut expected = Camera { orthographic_height: 7.0, ..Camera::default() };
        controller.orbit.apply(&mut expected);
        assert_eq!(camera, expected, "exactly OrbitCamera::apply");
    }

    #[test]
    fn a_pan_after_an_orthographic_zoom_still_follows_the_pointer() {
        let mut controller = OrbitController::default();
        let mut camera = orthographic();
        controller.apply(&mut camera);
        controller.set_viewport(&camera, 400.0);
        // Zoom in twice and out once; no set_viewport since.
        controller.input(&wheel(-120.0));
        controller.input(&wheel(-120.0));
        controller.input(&wheel(120.0));
        controller.apply(&mut camera);
        assert!((camera.orthographic_height - 10.0 * 0.81 * 1.1).abs() < 1e-4);
        let anchor = controller.orbit.target;
        let before = camera.world_to_canvas(anchor, 600.0, 400.0);
        controller.input(&down(1, 100.0, 100.0));
        assert!(controller.input(&drag_with(1, 140.0, 125.0, PointerButtons::SECONDARY, ModifierKeys::NONE, PointerKind::Mouse)));
        controller.apply(&mut camera);
        let after = camera.world_to_canvas(anchor, 600.0, 400.0);
        assert!((after.x - before.x - 40.0).abs() < 1e-2 && (after.y - before.y - 25.0).abs() < 1e-2, "{before:?} -> {after:?}");
    }

    #[test]
    fn frame_box_fits_a_box_for_an_orthographic_camera() {
        let mut controller = OrbitController::default();
        let (min, max) = (Vec3::new(-3.0, 0.0, -1.0), Vec3::new(5.0, 2.0, 1.0));
        let radius = ((max - min) * 0.5).length();
        for aspect in [16.0f32 / 9.0, 0.5] {
            let mut camera = Camera { orthographic_height: 1.0, far_plane: 1000.0, ..orthographic() };
            assert!(controller.frame_box(min, max, &camera, aspect));
            controller.apply(&mut camera);
            assert!((camera.orthographic_height - 2.0 * radius / aspect.min(1.0)).abs() < 1e-4, "the framing height, not the camera's own");
            let distance = (radius + camera.near_plane) / 0.3;
            assert!((controller.orbit.distance() - distance).abs() < 1e-3);
            assert!((controller.limits.min_distance - (radius + camera.near_plane)).abs() < 1e-4);
            assert!((controller.limits.max_distance - 3.0 * distance).abs() < 1e-3);
            let (width, height) = (aspect * 500.0, 500.0);
            for yaw in [0.0f32, 1.3, 3.0] {
                controller.orbit.yaw = yaw;
                controller.apply(&mut camera);
                for corner in [min, max, Vec3::new(min.x, max.y, max.z), Vec3::new(max.x, min.y, min.z)] {
                    assert!(camera.world_to_canvas(corner, width, height).in_view, "aspect {aspect} yaw {yaw}: {corner}");
                }
            }
            // Zoomed fully in: the height is the framing height times 0.3, and the box is still
            // in front of the near plane.
            for _ in 0..100 {
                controller.input(&wheel(-120.0));
            }
            controller.apply(&mut camera);
            assert!((camera.orthographic_height - 0.3 * 2.0 * radius / aspect.min(1.0)).abs() < 1e-3);
            for corner in [min, max] {
                assert!(camera.world_to_canvas(corner, width, height).depth >= 0.0);
            }
        }
        assert!((orthographic().fit_orthographic_height(2.0, 0.0) - 4.0).abs() < 1e-6, "an aspect of 0 counts as 1");
    }

    #[test]
    fn a_move_without_a_press_does_nothing() {
        let mut controller = OrbitController::default();
        let hover = CanvasInput::PointerMove { x: 5.0, y: 5.0, dx: 5.0, dy: 5.0, pressed: false, pointer_id: 1, buttons: PointerButtons::NONE, kind: PointerKind::Mouse, modifiers: ModifierKeys::NONE };
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
        assert!(controller.pan_enabled);
        assert_eq!(controller.pan_bounds, None);
        assert_eq!(controller, OrbitController::with_limits(orbit, OrbitLimits::default()));
    }
}
