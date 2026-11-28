//! `DrawingAnimationsMut`: making and removing a session's bindings (C# `session.Animations.Bind`
//! and its overloads, `BindAlongPath`, `Remove`, `Clear`, `Apply`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::drawing_session::DrawingSession;
use crate::models::drawing_animations::{check_binding, BindingEntry, BindingSource};
use crate::models::{AnimatedProperty, AnimationBinding, BindingMode, BindingTarget, BoundTrack, ElementId, PathTravelOptions};
use crate::paths::PathGeometry;

/// The session's animations, to change them (C#'s `session.Animations`). It borrows the session,
/// so a binding is checked against the element, layer or camera it names when it is made - as C#
/// checks the object it is handed.
pub struct DrawingAnimationsMut<'a> {
    pub(crate) session: &'a mut DrawingSession,
}

impl DrawingAnimationsMut<'_> {
    pub(crate) fn push(&mut self, target: Option<BindingTarget>, property: Option<AnimatedProperty>, source: BindingSource) -> AnimationBinding {
        self.push_with_mode(target, property, source, BindingMode::Replace)
    }

    pub(crate) fn push_with_mode(&mut self, target: Option<BindingTarget>, property: Option<AnimatedProperty>, source: BindingSource, mode: BindingMode) -> AnimationBinding {
        let animations = &mut self.session.animations;
        animations.next_id += 1;
        let binding = AnimationBinding { id: animations.next_id, target, property, mode };
        animations.entries.push(BindingEntry { binding: binding.clone(), source });
        binding
    }

    pub(crate) fn check_target(&self, target: &BindingTarget) -> Result<(), String> {
        match target {
            BindingTarget::Element(id) => match self.session.find_element(*id) {
                Some(_) => Ok(()),
                None => Err("The element is not on any of the session's layers: add it (draw_shape or a layer's add_element) before binding it. An element inside a group counts only once that group is on a layer: add the group first, then bind its elements.".to_string()),
            },
            BindingTarget::Layer(name) => match self.session.get_layer(name) {
                Some(_) => Ok(()),
                None => Err(format!("The session has no layer named '{name}': add it first.")),
            },
            BindingTarget::Camera => match self.session.camera() {
                Some(_) => Ok(()),
                None => Err("The session has no camera: set one with set_camera(Some(DrawingCamera::new())) before binding it.".to_string()),
            },
        }
    }

    /// Binds a track - or a value function of the time - to a property of an element, a layer or
    /// the camera (C#'s `Bind(element | group | layer | camera, property, track)` and
    /// `Bind(element, property, Func<double, T>)`): float tracks for Rotation, Scale, ScaleX,
    /// ScaleY, Opacity, TrimStart / TrimEnd / TrimOffset (path shapes) and MorphFraction (morphing
    /// path shapes); point tracks for Position, Anchor and Scale (both axes); colour tracks for a
    /// group's or layer's Color; bool tracks (or `BoundTrack::bool_fn`) for a layer's IsVisible; on the camera Position (pan),
    /// Anchor (pivot), Scale (zoom) and Rotation (roll). Pictures and sprites: Tint (colour
    /// track); sprites: FrameIndex (float track, rounded down). EFFECTS, on any element or a
    /// layer: BlurRadius, ShadowBlurRadius, GlowRadius (float tracks), ShadowOffset (point track),
    /// ShadowColor, GlowColor (colour tracks) - from no effects they start from
    /// `DrawingEffects::NONE`, as C# does. Opacity and the trims are clamped to 0..1, radii to 0
    /// or more. The error says what the target takes instead.
    pub fn bind(&mut self, target: impl Into<BindingTarget>, property: AnimatedProperty, track: impl Into<BoundTrack>) -> Result<AnimationBinding, String> {
        let target = target.into();
        let track = track.into();
        self.check_target(&target)?;
        let element = match &target {
            BindingTarget::Element(id) => self.session.find_element(*id),
            _ => None,
        };
        check_binding(&target, property, &track, element)?;
        Ok(self.push(Some(target), Some(property), BindingSource::Track(track)))
    }

    /// A function called with the time on every apply, in binding order (C#
    /// `Bind(Action<double> applyAtTime)`). It cannot reach the session (it is the session's);
    /// use it for state of your own that the drawing reads.
    pub fn bind_callback(&mut self, apply_at_time: impl FnMut(f64) + 'static) -> AnimationBinding {
        self.push(None, None, BindingSource::Callback(Rc::new(RefCell::new(apply_at_time))))
    }

    /// Moves an element along a path (C# `BindAlongPath(element, path, progress, orientToPath,
    /// rotationOffset)` and the `PathTravelOptions` overload): on every apply the element's
    /// transform is placed so that its ANCHOR (draw the element around (0, 0) to use its origin)
    /// lies on `path` at the fraction of its length `progress` gives (clamped to 0..1), and - with
    /// `orient_to_path` - turned to face along the path (the path's direction there, degrees, plus
    /// `rotation_offset`). The element's scale is kept. The path is in the element's parent's
    /// coordinates. `progress` is a float track or function. With a `roll_radius` or
    /// `extend_beyond_ends` it ROLLS and/or continues straight past the ends (C#'s
    /// `BindAlongPath(element, path, progress, PathTravelOptions)`; see
    /// [`bind_roll_along_path`](DrawingAnimationsMut::bind_roll_along_path)).
    pub fn bind_along_path(&mut self, element: ElementId, path: &PathGeometry, progress: impl Into<BoundTrack>, options: PathTravelOptions) -> Result<AnimationBinding, String> {
        let target = BindingTarget::Element(element);
        self.check_target(&target)?;
        let progress = progress.into();
        if progress.kind_name() != "float" {
            return Err("The progress along a path is a float track (fractions of the path's length).".to_string());
        }
        if !options.rotation_offset.is_finite() {
            return Err("The rotation offset must be a finite number of degrees.".to_string());
        }
        if options.roll_radius != 0.0 || options.extend_beyond_ends {
            return self.bind_travel_along_path(element, path, progress, options);
        }
        Ok(self.push(Some(target), Some(AnimatedProperty::Position), BindingSource::AlongPath { path: path.clone(), progress, options }))
    }

    /// Removes a binding; false when it is not one of these (C# `Remove`).
    pub fn remove(&mut self, binding: &AnimationBinding) -> bool {
        let entries = &mut self.session.animations.entries;
        match entries.iter().position(|e| e.binding.id == binding.id) {
            Some(i) => {
                let entry = entries.remove(i);
                self.session.animations.release_rests(&entry);
                true
            }
            None => false,
        }
    }

    /// Removes every binding (C# `Clear`).
    pub fn clear(&mut self) {
        self.session.animations.entries.clear();
        self.session.animations.rests.clear();
    }

    /// Applies every binding at a time, in order, without changing the session's time (C#
    /// `Animations.Apply(seconds)`; setting the session's time does this and more).
    pub fn apply(&mut self, seconds: f64) {
        self.session.apply_animations(seconds);
    }
}
