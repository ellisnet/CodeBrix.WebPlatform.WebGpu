//! `DrawingSession`: the root of a drawing - calibrated units, layers of shapes, the session
//! time (which APPLIES its animations), its camera, opacity, fonts and debug overlay (C#
//! `DrawingSession`, its animation side; the freehand-stroke and export members are not part of
//! this crate - see the documents).

use std::rc::Rc;

use kurbo::{Affine, Rect};
use peniko::{BlendMode, Brush, Fill};

use crate::captions::{CaptionPresenter, CaptionStyle, CaptionTrack};
use crate::color::Color;
use crate::drawing_session_blend::SessionBlendMode;
use crate::drawing_session_options::DrawingSessionOptions;
use crate::drawing_session_paint::SessionPaint;
use crate::rendering::scene_look::SceneLook;
use crate::effects::effect_sizes::EffectParams;
use crate::geometry::{PointF, RectangleF};
use crate::models::bound_track::BoundValue;
use crate::models::drawing_animations::{apply_to_camera, apply_to_element, apply_to_layer, BindingSource};
use crate::models::{BindingTarget, DrawingAnimations, DrawingAnimationsMut, DrawingCamera, DrawingLayer, ElementId};
use crate::rendering::{CanvasCalibration, DebugOverlay, VectorScene};
use crate::images::{ImageSampling, ImageSource, SpriteSheet};
use crate::layout::DrawingLayout;
use crate::safe_areas::SafeAreaOverlay;
use crate::shapes::{encode_masked, AnimatedShape, ArrowShape, ArrowShapeOptions, CircleShape, DrawingShape, EllipseShape, EncodeContext, ImageShape, ImageShapeOptions, LineShape, LineShapeOptions, PathShape, PolylineShape, PolylineShapeOptions, RectangleShape, RectangleShapeOptions, ShapeKind, ShapeOptions, SpriteShape, SpriteShapeOptions, TextBlockShape, TextShape, TextShapeOptions};
use crate::text::{FontRegistry, TextLayoutEngine, TextLayoutOptions, TextLayoutResult, TextRunDescriptor};

/// A drawing: a calibrated drawing space (aspect-fit into whatever canvas it is drawn on),
/// layers of shapes in drawing order, the session TIME (set it from the frame every frame - the
/// session never reads a clock: setting it applies every binding of its
/// [`animations`](DrawingSession::animations_mut), rebuilds its animated shapes and decides which
/// elements are inside their time windows), an optional [`DrawingCamera`] and an overall opacity.
#[derive(Clone, Debug)]
pub struct DrawingSession {
    options: DrawingSessionOptions,
    layers: Vec<DrawingLayer>,
    active_layer: Option<usize>,
    time: f64,
    opacity: f32,
    stroke_width: f32,
    fonts: FontRegistry,
    debug_overlay: Option<DebugOverlay>,
    safe_area_overlay: Option<SafeAreaOverlay>,
    camera: Option<DrawingCamera>,
    pub(crate) animations: DrawingAnimations,
    reports: std::cell::RefCell<SessionReports>,
    pub(crate) transition: Option<crate::drawing_session_transitions::LayerTransition>,
}

/// Problems the session found while drawing (an unknown matte layer name, for example), each
/// reported ONCE: kept until [`DrawingSession::take_reports`] takes them.
#[derive(Clone, Debug, Default)]
pub(crate) struct SessionReports {
    pending: Vec<String>,
    seen: std::collections::HashSet<String>,
}

impl SessionReports {
    /// Reports a problem unless the same message was reported before.
    pub(crate) fn report(&mut self, message: String) {
        if self.seen.insert(message.clone()) {
            self.pending.push(message);
        }
    }
}

/// How [`DrawingSession::render_with`] draws (C#'s export arguments, for a live canvas): with or
/// without the background, into the whole canvas or into a part of it; and (Not in C#) how the
/// session's picture combines with what was drawn before it - a blend mode and an opacity for
/// the whole session.
///
/// THE BACKGROUND IS LEFT OUT BY DEFAULT - a deliberate difference from C#, whose
/// `includeBackground` defaults to true: a session drawn as a LAYER of a canvas floats over what
/// lies below it (a 3D scene, or the web page itself on a transparent canvas). Ask for the
/// background with `include_background: true`.
///
/// The options are read at each render and change nothing in the session, so one session can be
/// drawn differently from frame to frame (or twice in one frame).
#[derive(Clone, Debug, PartialEq)]
pub struct SessionRenderOptions {
    /// True to fill the background colour under the layers (C# `includeBackground: true`, C#'s
    /// default); false (THE DEFAULT HERE) to draw the layers alone, so what is below - lower
    /// canvas layers, or the page behind a transparent canvas - shows through.
    pub include_background: bool,
    /// The part of the canvas, in canvas pixels, that the drawing is aspect-fitted into and
    /// clipped to; None (the default) for the whole canvas.
    pub target: Option<RectangleF>,
    /// How the session's whole picture (the background when drawn, and the layers) combines with
    /// what was drawn into the scene before it (default [`SessionBlendMode::Normal`]). Not in C#.
    pub blend: SessionBlendMode,
    /// The opacity of the session's whole picture, 0..1 (default 1; clamped, NaN counts as 0).
    /// Unlike [`DrawingSession::set_opacity`] it fades the background too (when drawn); the two
    /// multiply. The overlays (safe areas, debug) are not faded or blended. Not in C#.
    pub opacity: f32,
    /// How the shapes are painted (default [`SessionPaint::AsAuthored`]): as authored, as a line
    /// drawing in one stroke style ([`SessionPaint::Line`]), or in flat colours
    /// ([`SessionPaint::Flat`]). The background fill and the overlays are never repainted. Not
    /// in C#.
    pub paint: SessionPaint,
    /// False to draw every element, group and layer WITHOUT its effects (blur, glow, drop
    /// shadow; an SVG document's blur); default true. A layer's whole-frame operations and its
    /// layer transition are kept. Not in C#.
    pub effects: bool,
    /// The names of layers drawn AS AUTHORED whatever `paint` and `effects` say (empty by
    /// default): the exceptions to the look. Not in C#.
    pub authored_layers: Vec<String>,
    /// The session's whole picture (the background when drawn, and the layers) seen THROUGH WATER
    /// (default None): one effect group around it - inside the blend's group when the session is
    /// blended or faded (two levels then, three inside a [`SessionWipe`](crate::SessionWipe)). The water's coordinates
    /// are calibrated units under the camera (parallax 1). An unusable refraction is not drawn and
    /// is reported once ([`DrawingSession::take_reports`]). No C# equivalent.
    pub refraction: Option<crate::water::WaterRefraction>,
}

impl Default for SessionRenderOptions {
    fn default() -> SessionRenderOptions {
        SessionRenderOptions { include_background: false, target: None, blend: SessionBlendMode::Normal, opacity: 1.0, paint: SessionPaint::AsAuthored, effects: true, authored_layers: Vec::new(), refraction: None }
    }
}

impl Default for DrawingSession {
    fn default() -> DrawingSession {
        DrawingSession::new(DrawingSessionOptions::default())
    }
}

impl DrawingSession {
    /// A session with no layers.
    pub fn new(options: DrawingSessionOptions) -> DrawingSession {
        let stroke_width = options.stroke_width;
        let fonts = options.fonts.clone().unwrap_or_default();
        DrawingSession { options, layers: Vec::new(), active_layer: None, time: 0.0, opacity: 1.0, stroke_width, fonts, debug_overlay: None, safe_area_overlay: None, camera: None, animations: DrawingAnimations::default(), reports: std::cell::RefCell::new(SessionReports::default()), transition: None }
    }

    /// The options it was made with (the layer opacity and background as they are now).
    pub fn options(&self) -> &DrawingSessionOptions {
        &self.options
    }

    /// The calibrated size.
    pub fn calibration_size(&self) -> (u32, u32) {
        self.options.calibration_size
    }

    /// The session time, seconds: shapes with a time window are shown by it. Set it from the
    /// frame (`frame.info.time`, or a scene's local time) before drawing.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Shows the scene at a moment (C# `Time = seconds`): every binding of the animations is
    /// applied in the order bound, every [`AnimatedShape`](crate::shapes::AnimatedShape) on the
    /// layers (and inside groups) is rebuilt for the time, and from then on elements are drawn only
    /// inside their time windows. Setting the same time again applies everything again; seeking
    /// back needs nothing more. A time that is not finite is ignored (C# throws).
    pub fn set_time(&mut self, seconds: f64) {
        if !seconds.is_finite() {
            return;
        }
        self.time = seconds;
        self.apply_animations(seconds);
        for layer in &mut self.layers {
            for element in layer.get_elements_mut() {
                element.visit_mut(&mut |e| {
                    if let ShapeKind::Animated(a) = e.kind_mut() {
                        a.update(seconds);
                    }
                });
            }
        }
    }

    /// The session's animations: its bindings (C# `Animations`, to read).
    pub fn animations(&self) -> &DrawingAnimations {
        &self.animations
    }

    /// The session's animations, to bind tracks to its elements, layers and camera (C#
    /// `session.Animations.Bind(...)`).
    pub fn animations_mut(&mut self) -> DrawingAnimationsMut<'_> {
        DrawingAnimationsMut { session: self }
    }

    /// Applies every binding at a time, in order (C# `Animations.Apply`), without changing the
    /// session time or the animated shapes.
    pub(crate) fn apply_animations(&mut self, seconds: f64) {
        crate::models::binding_combine::apply_animations(self, seconds);
    }

    /// One binding at a time (the per-binding part of C#'s `Apply`; the order and the combining
    /// modes are `binding_combine::apply_animations`'s).
    pub(crate) fn apply_binding(&mut self, entry: &crate::models::drawing_animations::BindingEntry, seconds: f64) {
        let entries = std::slice::from_ref(entry);
        for entry in entries {
            let property = entry.binding.property;
            match (&entry.source, &entry.binding.target) {
                (BindingSource::Callback(callback), _) => (callback.borrow_mut())(seconds),
                (BindingSource::Track(track), Some(target)) => {
                    let (Some(value), Some(property)) = (track.value_at(seconds), property) else { continue };
                    match target {
                        BindingTarget::Element(id) => {
                            if let Some(e) = self.find_element_mut(*id) {
                                apply_to_element(e, property, value);
                            }
                        }
                        BindingTarget::Layer(name) => {
                            if let Some(l) = self.get_layer_mut(name) {
                                apply_to_layer(l, property, value);
                            }
                        }
                        BindingTarget::Camera => {
                            if let Some(c) = self.camera.as_mut() {
                                apply_to_camera(c, property, value);
                            }
                        }
                    }
                }
                (BindingSource::AlongPath { path, progress, options }, Some(BindingTarget::Element(id))) => {
                    let Some(BoundValue::Float(fraction)) = progress.value_at(seconds) else { continue };
                    let point = path.get_point_at(fraction);
                    if let Some(e) = self.find_element_mut(*id) {
                        let current = e.transform();
                        let translation = PointF::new(point.position.x - current.anchor().x, point.position.y - current.anchor().y);
                        let mut next = current.with_translation(translation);
                        if options.orient_to_path {
                            next = next.with_rotation(point.angle() + options.rotation_offset);
                        }
                        e.set_transform(next);
                    }
                }
                (BindingSource::Combine { track, slots }, Some(target)) => {
                    crate::models::binding_combine::apply_combine(self, target, track, slots, entry.binding.mode, seconds);
                }
                (BindingSource::Driver(driver), _) => driver.apply(self, seconds),
                _ => {}
            }
        }
    }

    /// The camera over the drawing (C# `Camera`; None by default).
    pub fn camera(&self) -> Option<&DrawingCamera> {
        self.camera.as_ref()
    }

    /// The camera, to change it.
    pub fn camera_mut(&mut self) -> Option<&mut DrawingCamera> {
        self.camera.as_mut()
    }

    /// Sets (or removes) the camera.
    pub fn set_camera(&mut self, camera: Option<DrawingCamera>) {
        self.camera = camera;
    }

    /// The element with this id on any layer (inside groups too).
    pub fn find_element(&self, id: ElementId) -> Option<&DrawingShape> {
        self.layers.iter().find_map(|l| l.find_element(id))
    }

    /// The element with this id, to change it.
    pub fn find_element_mut(&mut self, id: ElementId) -> Option<&mut DrawingShape> {
        self.layers.iter_mut().find_map(|l| l.find_element_mut(id))
    }

    /// The fonts this session's text uses: the registry of the options, or an empty one of its
    /// own. Register at least one font before drawing text - system fonts are never used.
    pub fn fonts(&self) -> &FontRegistry {
        &self.fonts
    }

    /// The debug overlay drawn over the frame, if any (off by default).
    pub fn debug_overlay(&self) -> Option<&DebugOverlay> {
        self.debug_overlay.as_ref()
    }

    /// The debug overlay, to change what it shows.
    pub fn debug_overlay_mut(&mut self) -> Option<&mut DebugOverlay> {
        self.debug_overlay.as_mut()
    }

    /// Switches the debug overlay on (Some) or off (None).
    pub fn set_debug_overlay(&mut self, overlay: Option<DebugOverlay>) {
        self.debug_overlay = overlay;
    }

    /// The safe-area overlay of a preview (C# `SafeAreaOverlay`; None by default): drawn over
    /// the layers and under the debug overlay, in the pixels of the frame as it is shown (the
    /// drawing's aspect-fitted rectangle), not faded with the session.
    pub fn safe_area_overlay(&self) -> Option<&SafeAreaOverlay> {
        self.safe_area_overlay.as_ref()
    }

    /// The safe-area overlay, to change it.
    pub fn safe_area_overlay_mut(&mut self) -> Option<&mut SafeAreaOverlay> {
        self.safe_area_overlay.as_mut()
    }

    /// Shows a safe-area overlay (Some) or hides it (None).
    pub fn set_safe_area_overlay(&mut self, overlay: Option<SafeAreaOverlay>) {
        self.safe_area_overlay = overlay;
    }

    /// The opacity of the whole drawing, 0..1: every layer, composited together as ONE group,
    /// faded over the background fill (which is not faded) - a whole-frame fade. Over an opaque
    /// background that is a fade from (or to) that colour.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Sets the opacity (clamped to 0..1).
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// The opacity every layer is composited at, 0..255.
    pub fn layer_opacity(&self) -> u8 {
        self.options.layer_opacity
    }

    /// Sets the layer opacity.
    pub fn set_layer_opacity(&mut self, value: u8) {
        self.options.layer_opacity = value;
    }

    /// The default thickness of the Draw* calls.
    pub fn stroke_width(&self) -> f32 {
        self.stroke_width
    }

    /// Sets the default thickness of the Draw* calls.
    pub fn set_stroke_width(&mut self, value: f32) {
        self.stroke_width = value;
    }

    /// Sets the background fill colour.
    pub fn set_background_fill_color(&mut self, color: Color) {
        self.options.background_fill_color = color;
    }

    /// The background fill colour.
    pub fn background_fill_color(&self) -> Color {
        self.options.background_fill_color
    }

    /// Adds a layer on top and makes it the active layer (C# `AddLayer(name, color)`). A name
    /// already used gets the existing layer back (made active), as C# does.
    pub fn add_layer(&mut self, name: &str, color: Color) -> &mut DrawingLayer {
        let index = match self.layers.iter().position(|l| l.name() == name) {
            Some(index) => index,
            None => {
                self.layers.push(DrawingLayer::new(name, color));
                self.layers.len() - 1
            }
        };
        self.active_layer = Some(index);
        &mut self.layers[index]
    }

    /// The layer with this name.
    pub fn get_layer(&self, name: &str) -> Option<&DrawingLayer> {
        self.layers.iter().find(|l| l.name() == name)
    }

    /// The layer with this name, to change it.
    pub fn get_layer_mut(&mut self, name: &str) -> Option<&mut DrawingLayer> {
        self.layers.iter_mut().find(|l| l.name() == name)
    }

    /// Removes the layer with this name; false when there was none.
    pub fn remove_layer(&mut self, name: &str) -> bool {
        let Some(index) = self.layers.iter().position(|l| l.name() == name) else {
            return false;
        };
        self.layers.remove(index);
        self.active_layer = match self.active_layer {
            Some(a) if a == index => self.layers.len().checked_sub(1),
            Some(a) if a > index => Some(a - 1),
            other => other,
        };
        true
    }

    /// The layers, bottom first.
    pub fn layers(&self) -> &[DrawingLayer] {
        &self.layers
    }

    /// The active layer (where the Draw* calls add shapes).
    pub fn active_layer(&self) -> Option<&DrawingLayer> {
        self.active_layer.map(|i| &self.layers[i])
    }

    /// Makes the named layer the active one; false when there is none.
    pub fn set_active_layer(&mut self, name: &str) -> bool {
        match self.layers.iter().position(|l| l.name() == name) {
            Some(i) => {
                self.active_layer = Some(i);
                true
            }
            None => false,
        }
    }

    /// Adds a shape to the active layer (a layer named "Layer 1" in black is made when there is
    /// none) and returns its id (C# `DrawShape`, which returns the shape).
    pub fn draw_shape(&mut self, shape: impl Into<DrawingShape>) -> ElementId {
        if self.active_layer.is_none() {
            self.add_layer("Layer 1", Color::BLACK);
        }
        let index = self.active_layer.unwrap();
        self.layers[index].add_element(shape)
    }

    /// A line on the active layer (C# `DrawLine(x1, y1, x2, y2, thickness = StrokeWidth,
    /// color = null)`; None for the thickness takes the session's stroke width).
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: Option<f32>, color: Option<Color>) -> Result<ElementId, String> {
        let shape = LineShape::new(x1, y1, x2, y2, LineShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color })?;
        Ok(self.draw_shape(shape))
    }

    /// A circle on the active layer.
    pub fn draw_circle(&mut self, center_x: f32, center_y: f32, radius: f32, thickness: Option<f32>, color: Option<Color>, filled: bool) -> Result<ElementId, String> {
        let shape = CircleShape::new(center_x, center_y, radius, ShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color, is_filled: filled })?;
        Ok(self.draw_shape(shape))
    }

    /// An ellipse on the active layer.
    pub fn draw_ellipse(&mut self, center_x: f32, center_y: f32, radius_x: f32, radius_y: f32, thickness: Option<f32>, color: Option<Color>, filled: bool) -> Result<ElementId, String> {
        let shape = EllipseShape::new(center_x, center_y, radius_x, radius_y, ShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color, is_filled: filled })?;
        Ok(self.draw_shape(shape))
    }

    /// A rectangle on the active layer.
    pub fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, thickness: Option<f32>, color: Option<Color>, filled: bool, corner_radius: f32) -> Result<ElementId, String> {
        let shape = RectangleShape::new(x, y, width, height, RectangleShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color, is_filled: filled, corner_radius })?;
        Ok(self.draw_shape(shape))
    }

    /// A polyline on the active layer.
    pub fn draw_polyline(&mut self, points: &[(f32, f32)], thickness: Option<f32>, color: Option<Color>, closed: bool, filled: bool) -> Result<ElementId, String> {
        let points: Vec<PointF> = points.iter().map(|p| PointF::from(*p)).collect();
        let shape = PolylineShape::new(&points, PolylineShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color, is_closed: closed, is_filled: filled })?;
        Ok(self.draw_shape(shape))
    }

    /// A path on the active layer.
    pub fn draw_path(&mut self, shape: PathShape) -> ElementId {
        self.draw_shape(shape)
    }

    /// An arrow on the active layer (C# `DrawArrow(x1, y1, x2, y2, thickness = StrokeWidth,
    /// color = null, headLength = null)`; None for the thickness takes the session's stroke
    /// width).
    pub fn draw_arrow(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: Option<f32>, color: Option<Color>, head_length: Option<f32>) -> Result<ElementId, String> {
        let shape = ArrowShape::new(x1, y1, x2, y2, ArrowShapeOptions { stroke_thickness: thickness.unwrap_or(self.stroke_width), color, head_length })?;
        Ok(self.draw_shape(shape))
    }

    /// A line of text on the active layer in the session's fonts, its anchor on the baseline at
    /// (x, y) (C# `DrawText(text, x, y, size, color = null, align = Left, tracking = 0,
    /// familyName = null)` and the overload with `kerning`). Returns the shape (a copy of what
    /// the layer holds), so its width and bounds can be used. The error says what to do.
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, options: TextShapeOptions) -> Result<TextShape, String> {
        let shape = TextShape::new(text, x, y, size, &self.fonts, options)?;
        self.draw_shape(shape.clone());
        Ok(shape)
    }

    /// Lays a block of text out in the session's fonts (C# `LayoutText(text, size, familyName =
    /// null, options = null)`): wrapped, aligned and trimmed as `options` say, with NoSkia's line
    /// breaker. Draw it with [`draw_text_block`](DrawingSession::draw_text_block). The error says
    /// what to do.
    pub fn layout_text(&self, text: &str, size: f32, family_name: Option<&str>, options: Option<&TextLayoutOptions>) -> Result<Rc<TextLayoutResult>, String> {
        TextLayoutEngine::layout(&self.fonts, text, size, family_name, options)
    }

    /// Lays out styled runs (each its own font, size, tracking, colour or fill) as one block (C#
    /// `LayoutText(runs, options = null)`).
    pub fn layout_text_runs(&self, runs: &[TextRunDescriptor], options: Option<&TextLayoutOptions>) -> Result<Rc<TextLayoutResult>, String> {
        TextLayoutEngine::layout_runs(&self.fonts, runs, options)
    }

    /// A laid-out block on the active layer with its top-left at (x, y), in a colour (None for
    /// the layer's) (C# `DrawTextBlock(layout, x, y, color = null)`). Returns its id - what reveal
    /// and stagger bindings name it by (C# returns the shape object). For an outlined or gradient
    /// block, or a block with a reveal set, make a [`TextBlockShape`] and use
    /// [`draw_shape`](DrawingSession::draw_shape).
    pub fn draw_text_block(&mut self, layout: Rc<TextLayoutResult>, x: f32, y: f32, color: Option<Color>) -> Result<ElementId, String> {
        let shape = TextBlockShape::new(layout, x, y, color)?;
        Ok(self.draw_shape(shape))
    }

    /// Captions on the active layer (C# `DrawCaptions(track, layout = null, style = null)`): the
    /// cues showing at the session time, each a laid-out block with its box, placed by the style
    /// AGAINST THE FRAME, kept the style's margin from its edges (the default; `layout` None is
    /// the whole calibrated frame). To place them inside a safe-area guide instead, opt in with
    /// [`draw_captions_with`](DrawingSession::draw_captions_with) and a presenter made
    /// `with_placement(CaptionPlacement::from_guide(...))`. Returns the id of the animated shape
    /// that shows them. The error says what to do.
    pub fn draw_captions(&mut self, track: CaptionTrack, layout: Option<DrawingLayout>, style: Option<CaptionStyle>) -> Result<ElementId, String> {
        let layout = match layout {
            Some(l) => l,
            None => DrawingLayout::new(self.options.calibration_size)?,
        };
        let presenter = CaptionPresenter::new(track, &self.fonts, layout, style)?;
        Ok(self.draw_captions_with(presenter))
    }

    /// Captions shown by a presenter of your own (its placement, its layout and style) on the
    /// active layer; returns the id of the animated shape that shows them.
    pub fn draw_captions_with(&mut self, presenter: CaptionPresenter) -> ElementId {
        let presenter = Rc::new(presenter);
        let factory = move |t: f64| -> Option<DrawingShape> {
            let placed = presenter.get_placed_captions_at(t).ok()?;
            let mut group = crate::shapes::DrawingGroup::new(None);
            for p in placed.iter() {
                if p.has_box() {
                    let rect = RectangleShape::new(p.box_bounds.x, p.box_bounds.y, p.box_bounds.width, p.box_bounds.height, RectangleShapeOptions { stroke_thickness: 1.0, color: p.background_color, is_filled: true, corner_radius: p.corner_radius }).ok()?;
                    group.add(rect);
                }
                for s in p.create_text_shapes().ok()? {
                    group.add(s);
                }
            }
            Some(DrawingShape::from(group))
        };
        let time = self.time;
        self.draw_shape(AnimatedShape::new(factory, time))
    }

    /// A picture on the active layer, stretched into `destination` (C# `DrawImage(source,
    /// destination, sampling = Linear)`); returns its id. The picture is shared, not copied.
    pub fn draw_image(&mut self, source: &ImageSource, destination: RectangleF, sampling: ImageSampling) -> Result<ElementId, String> {
        let shape = ImageShape::new(source.clone(), destination, ImageShapeOptions { sampling, ..ImageShapeOptions::default() })?;
        Ok(self.draw_shape(shape))
    }

    /// A sprite on the active layer: the sheet's frames shown one at a time in `destination`,
    /// advancing with the session's time at a frame rate (C# `DrawSprite(sheet, destination,
    /// framesPerSecond = 0, playback = Loop, sampling = Linear)`); returns its id.
    pub fn draw_sprite(&mut self, sheet: &SpriteSheet, destination: RectangleF, options: SpriteShapeOptions) -> Result<ElementId, String> {
        let shape = SpriteShape::new(sheet.clone(), destination, options)?;
        Ok(self.draw_shape(shape))
    }

    /// The rectangle the drawing occupies in a view of this size (aspect-fit, centred).
    pub fn get_drawing_rect(&self, view_width: f32, view_height: f32) -> RectangleF {
        CanvasCalibration::get_drawing_rect(view_width, view_height, self.options.calibration_size)
    }

    /// A calibrated length in view units for a view of this size.
    pub fn scale_to_view(&self, calibrated_length: f32, view_width: f32, view_height: f32) -> f32 {
        let rect = self.get_drawing_rect(view_width, view_height);
        if rect.is_empty() || calibrated_length <= 0.0 {
            return calibrated_length;
        }
        calibrated_length * (rect.width / self.options.calibration_size.0 as f32)
    }

    /// The transform from calibrated units to the pixels of a canvas of this size.
    pub fn calibration_transform(&self, width: u32, height: u32) -> Affine {
        let rect = self.get_drawing_rect(width as f32, height as f32);
        if rect.is_empty() {
            return Affine::scale(0.0);
        }
        Affine::translate((rect.x as f64, rect.y as f64)) * Affine::scale(rect.width as f64 / self.options.calibration_size.0 as f64)
    }

    /// Draws the session into a scene for a canvas of `width` x `height` pixels (C#
    /// `Render(canvas, info)`, but WITHOUT the background by default - see
    /// [`SessionRenderOptions`]; [`DrawingSession::render_with_background`] draws it): the layers - each
    /// shown layer as a group at the layer opacity times its own, its shapes shown at the session
    /// time, all of them in one group at the session opacity - then the debug overlay, if on.
    pub fn render(&self, scene: &mut VectorScene, width: u32, height: u32) {
        self.render_with(scene, width, height, &SessionRenderOptions::default());
    }

    /// Takes the problems found while drawing since the last call, each reported ONCE in the
    /// session's life: a layer matte ([`DrawingMask::from_layer`](crate::effects::DrawingMask::from_layer))
    /// naming a layer the session does not have, for example (it covers nothing). Take them after
    /// drawing and show or log them beside the device's errors
    /// (`GpuContext::subscribe_errors`) - the WebGPU error queue itself takes only WebGPU's.
    pub fn take_reports(&self) -> Vec<String> {
        std::mem::take(&mut self.reports.borrow_mut().pending)
    }

    /// Draws the session WITH its background fill under the layers (C#'s `Render` as it is): the
    /// same as [`DrawingSession::render_with`] with `include_background: true`.
    pub fn render_with_background(&self, scene: &mut VectorScene, width: u32, height: u32) {
        self.render_with(scene, width, height, &SessionRenderOptions { include_background: true, ..SessionRenderOptions::default() });
    }

    /// Draws the session with options: without its background (what is below shows through
    /// where nothing is drawn - C#'s `includeBackground: false`), or into a part of the canvas
    /// (aspect-fitted into it and clipped to it - how one session is placed inside another's
    /// frame).
    pub fn render_with(&self, scene: &mut VectorScene, width: u32, height: u32, options: &SessionRenderOptions) {
        let (cw, ch) = self.options.calibration_size;
        let (area, transform) = match options.target {
            Some(t) => {
                let rect = CanvasCalibration::get_drawing_rect(t.width, t.height, self.options.calibration_size);
                let transform = if rect.is_empty() { Affine::scale(0.0) } else { Affine::translate(((t.x + rect.x) as f64, (t.y + rect.y) as f64)) * Affine::scale(rect.width as f64 / cw as f64) };
                (Rect::new(t.x as f64, t.y as f64, (t.x + t.width) as f64, (t.y + t.height) as f64), transform)
            }
            None => (Rect::new(0.0, 0.0, width as f64, height as f64), self.calibration_transform(width, height)),
        };
        let clipped = options.target.is_some();
        if clipped {
            scene.push_clip_layer(Affine::IDENTITY, &area);
        }
        let background = self.options.background_fill_color;
        let draws_background = options.include_background && background.a > 0;
        // The session's whole picture through one group when it is blended or faded (the render
        // options' opacity fades the background too); the session's own opacity, which leaves
        // the background unfaded, is folded into that group when there is no background.
        let picture_alpha = if options.opacity.is_nan() { 0.0 } else { options.opacity.clamp(0.0, 1.0) };
        let picture_group = options.blend != SessionBlendMode::Normal || picture_alpha < 1.0;
        let fold_opacity = picture_group && !draws_background;
        let layers_alpha = if fold_opacity { 1.0 } else { self.opacity };
        if picture_alpha > 0.0 {
            if picture_group {
                let alpha = if fold_opacity { picture_alpha * self.opacity } else { picture_alpha };
                scene.push_layer(options.blend.to_peniko(), alpha, Affine::IDENTITY, &area);
            }
            // The session seen through water (Not in C#): one effect group around its picture.
            let water = self.session_water(scene, transform, options);
            if let Some(w) = water {
                scene.push_effects(EffectParams { water: Some(w), ..EffectParams::default() }, 1.0);
            }
            if draws_background {
                scene.fill(Fill::NonZero, transform, &Brush::Solid(background.to_peniko(1.0)), None, &Rect::new(0.0, 0.0, cw as f64, ch as f64));
            }
            self.render_layers(scene, transform, area, layers_alpha, options);
            if water.is_some() {
                scene.pop_layer();
            }
            if picture_group {
                scene.pop_layer();
            }
        }
        self.render_overlays(scene, width, height, options, area);
        if clipped {
            scene.pop_layer();
        }
    }

    /// A session refraction's water for this frame (None without one, or when it is unusable -
    /// reported once): calibrated units under the camera at parallax 1.
    fn session_water(&self, scene: &mut VectorScene, transform: Affine, options: &SessionRenderOptions) -> Option<crate::effects::effect_sizes::WaterParams> {
        let refraction = options.refraction.as_ref()?;
        if let Err(e) = refraction.validate() {
            self.reports.borrow_mut().report(format!("The session's refraction is not drawn: {e}"));
            return None;
        }
        let camera = self.camera.as_ref().map_or(Affine::IDENTITY, |c| c.get_matrix(self.options.calibration_size, 1.0));
        let reports = &self.reports;
        crate::effects::effect_sizes::record_water(scene, Some(refraction), None, transform * camera, self.time, &mut |m| reports.borrow_mut().report(m))
    }

    /// The layers (each a group at its alpha, through its effects, mask and transition), all of
    /// them in one group at `opacity` when it is below 1; each layer's content under the options'
    /// look (paint and effects) unless the layer is one of the authored exceptions.
    fn render_layers(&self, scene: &mut VectorScene, transform: Affine, area: Rect, opacity: f32, options: &SessionRenderOptions) {
        let outer_look = scene.look.take();
        let has_look = options.paint != SessionPaint::AsAuthored || !options.effects;
        let (cw, ch) = self.options.calibration_size;
        // The frame as drawn (device pixels): the extent of transitions and vignettes.
        let frame_rect = transform.transform_rect_bbox(Rect::new(0.0, 0.0, cw as f64, ch as f64));
        if opacity > 0.0 {
            let whole = opacity < 1.0;
            if whole {
                scene.push_layer(BlendMode::default(), opacity, Affine::IDENTITY, &area);
            }
            for layer in &self.layers {
                if !layer.is_visible() {
                    continue;
                }
                let alpha = (self.options.layer_opacity as f32 * layer.opacity()).round() / 255.0;
                if alpha <= 0.0 {
                    continue;
                }
                // calibration, then the camera as this layer feels it, then the layer's transform.
                let camera = self.camera.as_ref().map_or(Affine::IDENTITY, |c| c.get_matrix(self.options.calibration_size, layer.parallax()));
                // A slide or push moves the layer by whole device pixels, inside the frame.
                let (shift, pass, moved) = self.transition_part(layer.name(), frame_rect);
                let layer_transform = shift * transform * camera * layer.transform().to_matrix();
                if moved {
                    scene.push_clip_layer(Affine::IDENTITY, &frame_rect);
                }
                let c = camera.as_coeffs();
                let drawing = shift * transform * camera;
                let ctx = EncodeContext { time: self.time, area, scene_rotation: c[1].atan2(c[0]), layers: &self.layers, drawing, reports: &self.reports };
                // The look applies to the layer's group (its effects) and its content.
                let exempt = options.authored_layers.iter().any(|n| n == layer.name());
                scene.look = if has_look && !exempt { Some(SceneLook { paint: options.paint.clone(), effects: options.effects, drawing }) } else { None };
                // A layer is one group at its alpha - or, with effects, one group through its
                // effects at that alpha (C#'s ApplyToLayer); its mask applies to its content first.
                let effects = layer.effects().and_then(|e| EffectParams::new(&e, layer_transform, ctx.scene_rotation));
                // The whole-frame operations and a transition's pass ride on the effects' group.
                let extent = [frame_rect.x0 as f32, frame_rect.y0 as f32, frame_rect.x1 as f32, frame_rect.y1 as f32];
                let frame = crate::effects::effect_sizes::frame_params(layer.frame_operations().as_ref(), pass, transform, extent);
                // So does the surface finish (Not in C#) - but not under the line or flat look.
                let paints_as_authored = scene.look.as_ref().map_or(true, |l| l.paint == SessionPaint::AsAuthored);
                let finish = layer.surface_finish().filter(|_| paints_as_authored).and_then(|f| crate::effects::surface_finish::finish_params(&f, layer_transform));
                // And the layer's water (Not in C#): its refraction and reflection, in the layer's
                // drawing space (under every look and with effects off: they are passes).
                let reports = &self.reports;
                let water = crate::effects::effect_sizes::record_water(scene, layer.refraction(), layer.reflection(), drawing, self.time, &mut |m| reports.borrow_mut().report(m));
                let effects = match (effects, frame, finish, water) {
                    (None, None, None, None) => None,
                    (e, f, finish, water) => {
                        let mut p = e.unwrap_or_default();
                        p.frame = f;
                        p.finish = finish;
                        p.water = water;
                        Some(p)
                    }
                };
                match effects {
                    Some(params) => scene.push_effects(params, alpha),
                    None => scene.push_layer(BlendMode::default(), alpha, Affine::IDENTITY, &area),
                }
                encode_masked(scene, layer.mask(), layer_transform, &ctx, true, &mut |scene| {
                    for shape in layer.get_elements() {
                        if shape.is_visible_at(self.time) {
                            shape.encode(scene, layer_transform, layer.color(), &ctx);
                        }
                    }
                });
                scene.pop_layer();
                scene.look = None;
                if moved {
                    scene.pop_layer();
                }
            }
            if whole {
                scene.pop_layer();
            }
        }
        scene.look = outer_look;
    }

    /// The safe-area overlay and the debug overlay, over the session's picture (never blended or
    /// faded with it).
    fn render_overlays(&self, scene: &mut VectorScene, width: u32, height: u32, options: &SessionRenderOptions, area: Rect) {
        if let Some(overlay) = &self.safe_area_overlay {
            // In the pixels of the frame as shown (the aspect-fitted drawing rectangle), over the
            // layers, not faded with them; overlapping zones never darken (one group per part).
            let frame = match options.target {
                Some(t) => {
                    let r = CanvasCalibration::get_drawing_rect(t.width, t.height, self.options.calibration_size);
                    RectangleF::new(t.x + r.x, t.y + r.y, r.width, r.height)
                }
                None => self.get_drawing_rect(width as f32, height as f32),
            };
            if !frame.is_empty() {
                let plan = overlay.get_draw_plan((frame.width, frame.height), self.time);
                let place = Affine::translate((frame.x as f64, frame.y as f64));
                let parts = plan.mask.iter().map(|(c, a, rects)| (*c, *a, rects.clone())).chain(plan.outline.iter().map(|(c, a, rects)| (*c, *a, rects.to_vec())));
                for (color, alpha, rects) in parts {
                    scene.push_layer(BlendMode::default(), alpha as f32 / 255.0, Affine::IDENTITY, &area);
                    let brush = Brush::Solid(color.to_peniko(1.0));
                    for r in rects.iter().filter(|r| r.width > 0.0 && r.height > 0.0) {
                        scene.fill(Fill::NonZero, place, &brush, None, &Rect::new(r.x as f64, r.y as f64, r.right() as f64, r.bottom() as f64));
                    }
                    scene.pop_layer();
                }
            }
        }
        if let Some(overlay) = &self.debug_overlay {
            overlay.render_at(scene, area, self.time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> DrawingSession {
        let mut s = DrawingSession::new(DrawingSessionOptions { calibration_size: (100, 100), layer_opacity: 255, background_fill_color: Color::BLACK, ..DrawingSessionOptions::default() });
        s.add_layer("A", Color::WHITE);
        s.draw_circle(50.0, 50.0, 10.0, None, None, true).unwrap();
        s
    }

    #[test]
    fn the_session_opacity_fades_the_layers_as_one_group_over_an_unfaded_background() {
        let mut s = session();
        let mut scene = VectorScene::new();
        s.render_with_background(&mut scene, 100, 100);
        assert_eq!(scene.item_kinds(), "FLFP", "background, then the layer's group");
        s.set_opacity(0.5);
        scene.reset();
        s.render_with_background(&mut scene, 100, 100);
        assert_eq!(scene.item_kinds(), "FLLFPP", "the background stays outside the session's group");
        s.set_opacity(0.0);
        scene.reset();
        s.render_with_background(&mut scene, 100, 100);
        assert_eq!(scene.item_kinds(), "F", "fully faded: the background alone");
        s.get_layer_mut("A").unwrap().set_opacity(0.0);
        s.set_opacity(1.0);
        scene.reset();
        s.render_with_background(&mut scene, 100, 100);
        assert_eq!(scene.item_kinds(), "F", "a layer at opacity 0 is skipped");
    }

    #[test]
    fn a_session_drawn_as_a_layer_leaves_its_background_out_by_default() {
        let s = session();
        let mut scene = VectorScene::new();
        s.render(&mut scene, 100, 100);
        assert_eq!(scene.item_kinds(), "LFP", "no background fill: the page or the layers below show");
        assert!(!SessionRenderOptions::default().include_background);
    }

    #[test]
    fn render_with_leaves_the_background_out_or_draws_into_a_part_of_the_canvas() {
        let s = session();
        let mut scene = VectorScene::new();
        s.render_with(&mut scene, 100, 100, &SessionRenderOptions { include_background: false, target: None, ..SessionRenderOptions::default() });
        assert_eq!(scene.item_kinds(), "LFP");
        scene.reset();
        s.render_with(&mut scene, 1000, 1000, &SessionRenderOptions { include_background: true, target: Some(RectangleF::new(100.0, 200.0, 400.0, 200.0)), ..SessionRenderOptions::default() });
        assert_eq!(scene.item_kinds(), "CFLFPP", "clipped to the part");
    }
}
