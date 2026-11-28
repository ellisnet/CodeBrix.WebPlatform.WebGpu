//! `DrawingLayer`: a named, coloured layer of a drawing session holding shapes, with its own
//! transform, parallax, mask and effects (C# `DrawingLayer`), and (Not in C#) a surface finish, a
//! refraction and a reflection.

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
    surface_finish: Option<crate::effects::SurfaceFinish>,
    refraction: Option<crate::water::WaterRefraction>,
    reflection: Option<crate::water::WaterReflection>,
    is_safe_area_checked: bool,
    start_time: Option<f64>,
    end_time: Option<f64>,
    blend: crate::drawing_session_blend::SessionBlendMode,
    elements: Vec<DrawingShape>,
}

impl DrawingLayer {
    /// An empty layer.
    pub fn new(name: &str, color: Color) -> DrawingLayer {
        DrawingLayer { name: name.to_string(), color, opacity: 1.0, is_visible: true, transform: DrawingTransform::IDENTITY, parallax: 1.0, mask: None, effects: None, frame_operations: None, surface_finish: None, refraction: None, reflection: None, is_safe_area_checked: false, start_time: None, end_time: None, blend: crate::drawing_session_blend::SessionBlendMode::Normal, elements: Vec::new() }
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

    /// The layer's surface finish (None by default): the canvas weave and the paint's thickness,
    /// lit (Not in C#; see [`SurfaceFinish`](crate::effects::SurfaceFinish)).
    pub fn surface_finish(&self) -> Option<crate::effects::SurfaceFinish> {
        self.surface_finish
    }

    /// Sets (or removes) the surface finish (Not in C#). A finished layer is drawn through its
    /// effect group (one nesting level, shared with its effects and frame operations); its stroke
    /// sets give the paint its thickness. The error says which value of the finish is unusable
    /// (the layer keeps its finish then).
    pub fn set_surface_finish(&mut self, finish: Option<crate::effects::SurfaceFinish>) -> Result<(), String> {
        if let Some(f) = &finish {
            f.validate()?;
        }
        self.surface_finish = finish;
        Ok(())
    }

    /// The layer's refraction (None by default): its picture seen through water (Not in C#; see
    /// [`WaterRefraction`](crate::water::WaterRefraction)).
    pub fn refraction(&self) -> Option<&crate::water::WaterRefraction> {
        self.refraction.as_ref()
    }

    /// Sets (or removes) the refraction (Not in C#). The layer is drawn through its effect group
    /// (one nesting level, shared with its effects, frame operations, finish and reflection). The
    /// error says which value is unusable (the layer keeps its refraction then). Replace it each
    /// frame to animate its settings; a ripple field moves by itself with the session time.
    pub fn set_refraction(&mut self, refraction: Option<crate::water::WaterRefraction>) -> Result<(), String> {
        if let Some(r) = &refraction {
            r.validate()?;
        }
        self.refraction = refraction;
        Ok(())
    }

    /// The layer's reflection (None by default): the layer mirrored below a waterline (Not in C#;
    /// see [`WaterReflection`](crate::water::WaterReflection)).
    pub fn reflection(&self) -> Option<&crate::water::WaterReflection> {
        self.reflection.as_ref()
    }

    /// Sets (or removes) the reflection (Not in C#): drawn from the layer's own picture in its
    /// effect group (shared with its refraction - nothing is drawn twice). The error says which
    /// value is unusable (the layer keeps its reflection then).
    pub fn set_reflection(&mut self, reflection: Option<crate::water::WaterReflection>) -> Result<(), String> {
        if let Some(r) = &reflection {
            r.validate()?;
        }
        self.reflection = reflection;
        Ok(())
    }

    /// True when the layer is drawn.
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Shows or hides the layer.
    pub fn set_is_visible(&mut self, value: bool) {
        self.is_visible = value;
    }

    /// The session time from which the layer is shown (None, the default: from the start). Not
    /// in C# (an element's [`start_time`](crate::shapes::DrawingShape::start_time) for a whole
    /// layer).
    pub fn start_time(&self) -> Option<f64> {
        self.start_time
    }

    /// The session time from which the layer is no longer shown (None, the default: never
    /// hidden). Not in C#.
    pub fn end_time(&self) -> Option<f64> {
        self.end_time
    }

    /// Sets the layer's time window, in session seconds (Not in C#; as an element's
    /// [`set_time_window`](crate::shapes::DrawingShape::set_time_window)): the layer is drawn
    /// from `start_time` (None: from the start) until before `end_time` (None: never hidden), and
    /// checked by [`check_safe_area`](crate::DrawingSession::check_safe_area) only then. Its
    /// elements' own windows still apply inside it. A layer used as a matte
    /// ([`DrawingMask::from_layer`]) ignores its window, as it ignores its visibility.
    pub fn set_time_window(&mut self, start_time: Option<f64>, end_time: Option<f64>) {
        self.start_time = start_time;
        self.end_time = end_time;
    }

    /// The same layer with a time window (see [`set_time_window`](DrawingLayer::set_time_window)).
    pub fn with_time_window(mut self, start_time: Option<f64>, end_time: Option<f64>) -> DrawingLayer {
        self.set_time_window(start_time, end_time);
        self
    }

    /// How the layer combines with the layers below it (default
    /// [`SessionBlendMode::Normal`](crate::SessionBlendMode::Normal)). Not in C#.
    pub fn blend_mode(&self) -> crate::drawing_session_blend::SessionBlendMode {
        self.blend
    }

    /// Sets how the layer's group combines with the layers below it in its session (Not in C#;
    /// the session's own [`SessionRenderOptions::blend`](crate::SessionRenderOptions::blend) is
    /// how the whole session combines with what is below IT). Every layer is already one group,
    /// so a blend costs no extra level - except with effects (blur, glow, shadow, frame
    /// operations, a surface finish, water), where the blend's group goes around the effects'
    /// group: one more level of the renderer's nesting allowance (a scene already as deep as
    /// the renderer goes draws its deepest groups as plain clips, counted in
    /// [`RenderStats::unsupported`](crate::rendering::RenderStats::unsupported)). A mode other
    /// than `Normal` reads a copy of the backdrop when the layer is composited. A layer used as
    /// a matte ignores its blend mode.
    pub fn set_blend_mode(&mut self, blend: crate::drawing_session_blend::SessionBlendMode) {
        self.blend = blend;
    }

    /// The same layer with a blend mode (see [`set_blend_mode`](DrawingLayer::set_blend_mode)).
    pub fn with_blend_mode(mut self, blend: crate::drawing_session_blend::SessionBlendMode) -> DrawingLayer {
        self.blend = blend;
        self
    }

    /// True when the layer is drawn at a session time: it is visible
    /// ([`is_visible`](DrawingLayer::is_visible)) and the time is at or after its start and
    /// before its end. Not in C#.
    pub fn is_visible_at(&self, seconds: f64) -> bool {
        self.is_visible && self.start_time.is_none_or(|s| seconds >= s) && self.end_time.is_none_or(|e| seconds < e)
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
