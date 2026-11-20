//! `DrawingLayer`: a named, coloured layer of a drawing session holding shapes, with its own
//! transform, parallax, mask and effects (C# `DrawingLayer`).

use crate::color::Color;
use crate::effects::{DrawingEffects, DrawingMask};
use crate::models::{DrawingTransform, ElementId};
use crate::shapes::DrawingShape;

/// A layer of a [`DrawingSession`](crate::DrawingSession): its name, its colour (what its shapes
/// use when they have no colour of their own), its opacity, whether it is shown, its transform
/// (the whole layer moved, turned, scaled), its PARALLAX (how much it follows the session's
/// camera: 1 fully, 0 not at all - a HUD) and its shapes in drawing order. The session composites
/// each shown layer as one group at the session's `layer_opacity` times the layer's `opacity`.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingLayer {
    name: String,
    color: Color,
    opacity: f32,
    is_visible: bool,
    transform: DrawingTransform,
    parallax: f32,
    mask: Option<DrawingMask>,
    effects: Option<DrawingEffects>,
    frame_operations: Option<crate::effects::DrawingFrameOperations>,
    is_safe_area_checked: bool,
    elements: Vec<DrawingShape>,
}

impl DrawingLayer {
    /// An empty layer.
    pub fn new(name: &str, color: Color) -> DrawingLayer {
        DrawingLayer { name: name.to_string(), color, opacity: 1.0, is_visible: true, transform: DrawingTransform::IDENTITY, parallax: 1.0, mask: None, effects: None, frame_operations: None, is_safe_area_checked: false, elements: Vec::new() }
    }

    /// The name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The layer's colour.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Sets the layer's colour (C# `SetColor`).
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// The layer's opacity, 0..1.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Sets the opacity (clamped to 0..1; NaN counts as 0).
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = if opacity.is_nan() { 0.0 } else { opacity.clamp(0.0, 1.0) };
    }

    /// The layer's transform (C# `Transform`; identity by default), applied to all its shapes.
    pub fn transform(&self) -> DrawingTransform {
        self.transform
    }

    /// Sets the layer's transform.
    pub fn set_transform(&mut self, transform: DrawingTransform) {
        self.transform = transform;
    }

    /// How much the layer follows the session's camera (C# `Parallax`, default 1): 1 moves with
    /// the scene, 0 stays fixed to the frame, 0.15 is a far background.
    pub fn parallax(&self) -> f32 {
        self.parallax
    }

    /// Sets the parallax. The error says what to do with a value that is not finite.
    pub fn set_parallax(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("A layer's parallax must be a finite number (1 follows the camera, 0 stays fixed).".to_string());
        }
        self.parallax = value;
        Ok(())
    }

    /// The layer's mask (C# `Mask`; None by default), in the layer's element coordinates (it
    /// follows the layer's transform and the camera, as its elements do).
    pub fn mask(&self) -> Option<&DrawingMask> {
        self.mask.as_ref()
    }

    /// Sets (or removes) the mask; replace it every frame to move it (a spotlight).
    pub fn set_mask(&mut self, mask: Option<DrawingMask>) {
        self.mask = mask;
    }

    /// The layer's effects (C# `Effects`; None by default): one blur, glow and shadow for
    /// everything on the layer, its sizes following the layer's transform under the camera.
    pub fn effects(&self) -> Option<DrawingEffects> {
        self.effects
    }

    /// Sets (or removes) the effects (bindable like an element's).
    pub fn set_effects(&mut self, effects: Option<DrawingEffects>) {
        self.effects = effects;
    }

    /// The whole-frame operations drawn over the layer on the GPU (tone curve, gamma, vignette,
    /// bloom), or None.
    pub fn frame_operations(&self) -> Option<crate::effects::DrawingFrameOperations> {
        self.frame_operations
    }

    /// Sets (or removes) the whole-frame operations: the GPU versions of the planes' tone curve,
    /// gamma, vignette and bloom over everything the layer draws (no C# equivalent: C# applies
    /// them to a plane image on the CPU).
    pub fn set_frame_operations(&mut self, operations: Option<crate::effects::DrawingFrameOperations>) {
        self.frame_operations = operations.filter(|o| !o.is_empty());
    }

    /// True when the layer is drawn.
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Shows or hides the layer.
    pub fn set_is_visible(&mut self, value: bool) {
        self.is_visible = value;
    }

    /// True when every element of the layer is checked by
    /// [`DrawingSession::check_safe_area`](crate::DrawingSession::check_safe_area) (C#
    /// `IsSafeAreaChecked`; false by default).
    pub fn is_safe_area_checked(&self) -> bool {
        self.is_safe_area_checked
    }

    /// Sets whether every element of the layer is checked against safe areas.
    pub fn set_is_safe_area_checked(&mut self, value: bool) {
        self.is_safe_area_checked = value;
    }

    /// Adds a shape on top. Always true (C# returns false only for a disposed session); use
    /// [`DrawingLayer::add_element`] for the shape's id.
    pub fn add_shape(&mut self, shape: impl Into<DrawingShape>) -> bool {
        self.elements.push(shape.into());
        true
    }

    /// Adds a shape on top and returns its id (what C# code keeps as the element's reference,
    /// to bind it or find it again).
    pub fn add_element(&mut self, shape: impl Into<DrawingShape>) -> ElementId {
        let shape = shape.into();
        let id = shape.id();
        self.elements.push(shape);
        id
    }

    /// The element with this id, on the layer or inside one of its groups.
    pub fn find_element(&self, id: ElementId) -> Option<&DrawingShape> {
        self.elements.iter().find_map(|e| e.find(id))
    }

    /// The element with this id, to change it.
    pub fn find_element_mut(&mut self, id: ElementId) -> Option<&mut DrawingShape> {
        self.elements.iter_mut().find_map(|e| e.find_mut(id))
    }

    /// The shapes, to change them.
    pub fn get_elements_mut(&mut self) -> &mut [DrawingShape] {
        &mut self.elements
    }

    /// Removes the top shape; false when there was none.
    pub fn remove_last_element(&mut self) -> bool {
        self.elements.pop().is_some()
    }

    /// Removes every shape.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// The number of shapes.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// The shapes, bottom first (C# `GetElements`).
    pub fn get_elements(&self) -> &[DrawingShape] {
        &self.elements
    }
}
