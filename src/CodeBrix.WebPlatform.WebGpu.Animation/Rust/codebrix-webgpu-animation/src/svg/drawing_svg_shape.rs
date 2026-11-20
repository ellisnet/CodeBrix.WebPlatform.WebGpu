//! `DrawingSvgShape`: a loaded document placed in a destination rectangle with per-part
//! overrides and part bindings (C# `DrawingSvgShape`), as DATA: what it draws at a time is
//! [`draw_list`](DrawingSvgShape::draw_list). Making it an element of a drawing session is the
//! drawing half's job (see the integration notes).

use std::sync::Arc;

use kurbo::{Affine, Rect};

use crate::animation::{FloatTrack, PointTrack};
use crate::geometry::{PointF, RectangleF};
use crate::models::AnimatedProperty;
use crate::svg::drawing_svg::DrawingSvg;
use crate::svg::drawing_svg_aspect::DrawingSvgAspect;
use crate::svg::drawing_svg_part::DrawingSvgPart;
use crate::svg::svg_draw_command::{SvgClipShape, SvgDrawCommand, SvgDrawList};
use crate::images::ImageSource;
use crate::svg::svg_leaf_nodes::SvgImageSource;
use crate::svg::svg_paint::SvgFillRule;
use crate::svg::svg_scene::SvgNodeKind;

#[derive(Clone, Debug)]
enum PartTrack {
    Float(FloatTrack),
    Point(PointTrack),
}

#[derive(Clone, Debug)]
struct PartBinding {
    part: usize,
    property: AnimatedProperty,
    track: PartTrack,
}

/// A document in a destination, its parts, and their bindings. As a session element
/// ([`DrawingSession::draw_svg`](crate::DrawingSession::draw_svg)) it is drawn at the session's
/// time: the bindings evaluated, the document's own animations played.
#[derive(Clone, Debug)]
pub struct DrawingSvgShape {
    svg: Arc<DrawingSvg>,
    destination: RectangleF,
    aspect: DrawingSvgAspect,
    parts: Vec<DrawingSvgPart>,
    bindings: Vec<PartBinding>,
    /// Plays the document's OWN animations (SMIL, CSS) at the time passed to
    /// [`draw_list`](DrawingSvgShape::draw_list); default true.
    pub play_document_animations: bool,
    /// Seconds subtracted from the time before the document's own animations are evaluated
    /// (the document's time 0 = this time in the caller's time).
    pub document_time_offset: f64,
    /// The decoded pixels the application supplied for the document's images.
    images: Vec<(SvgImageSource, ImageSource)>,
}

/// Two shapes are equal when they show the same loaded document (the same `Arc`) in the same
/// place with the same parts, settings, pictures and number of bindings (tracks have no
/// equality of their own).
impl PartialEq for DrawingSvgShape {
    fn eq(&self, other: &DrawingSvgShape) -> bool {
        Arc::ptr_eq(&self.svg, &other.svg)
            && self.destination == other.destination
            && self.aspect == other.aspect
            && self.parts == other.parts
            && self.bindings.len() == other.bindings.len()
            && self.play_document_animations == other.play_document_animations
            && self.document_time_offset == other.document_time_offset
            && self.images == other.images
    }
}

impl DrawingSvgShape {
    /// A document fitted into `destination` (C# `new DrawingSvgShape(svg, destination,
    /// aspect = Meet)`).
    pub fn new(svg: Arc<DrawingSvg>, destination: RectangleF, aspect: DrawingSvgAspect) -> DrawingSvgShape {
        let parts = svg.new_parts();
        DrawingSvgShape { svg, destination, aspect, parts, bindings: Vec::new(), play_document_animations: true, document_time_offset: 0.0, images: Vec::new() }
    }

    /// The document.
    pub fn svg(&self) -> &Arc<DrawingSvg> {
        &self.svg
    }

    /// The destination (C# `Destination`).
    pub fn destination(&self) -> RectangleF {
        self.destination
    }

    /// The fit (C# `Aspect`).
    pub fn aspect(&self) -> DrawingSvgAspect {
        self.aspect
    }

    /// The document's box (C# `DocumentBounds`).
    pub fn document_bounds(&self) -> RectangleF {
        self.svg.bounds()
    }

    /// The matrix from document coordinates to calibrated units.
    pub fn content_transform(&self) -> Affine {
        let (w, h) = self.svg.scene().size();
        self.aspect.fit(w, h, self.destination)
    }

    /// Where the document box lands (C# `ContentBounds`).
    pub fn content_bounds(&self) -> RectangleF {
        let (w, h) = self.svg.scene().size();
        let r = self.content_transform().transform_rect_bbox(Rect::new(0.0, 0.0, w, h));
        RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32)
    }

    /// The fitted box (C# `GetBounds`; Slice: the destination).
    pub fn get_bounds(&self) -> RectangleF {
        if self.aspect == DrawingSvgAspect::Slice {
            self.destination
        } else {
            self.content_bounds()
        }
    }

    /// Every part id (C# `GetPartIds`).
    pub fn get_part_ids(&self) -> Vec<String> {
        self.svg.get_part_ids()
    }

    /// True when the id names a part (C# `HasPart`).
    pub fn has_part(&self, id: &str) -> bool {
        self.svg.has_part(id)
    }

    /// Every part's override (C# `Parts`).
    pub fn parts(&self) -> &[DrawingSvgPart] {
        &self.parts
    }

    /// One part (C# `GetPart`; C# throws ArgumentException for an unknown id - here the error
    /// lists how to find the ids).
    pub fn get_part(&self, id: &str) -> Result<&DrawingSvgPart, String> {
        let i = self.position(id)?;
        Ok(&self.parts[i])
    }

    /// One part, to change.
    pub fn get_part_mut(&mut self, id: &str) -> Result<&mut DrawingSvgPart, String> {
        let i = self.position(id)?;
        Ok(&mut self.parts[i])
    }

    fn position(&self, id: &str) -> Result<usize, String> {
        self.svg.scene().part_position(id).ok_or_else(|| format!("The document has no drawn part with the id '{id}'; get_part_ids() lists the ids it has."))
    }

    /// Every part back to its defaults (C# `ResetParts`); bindings stay.
    pub fn reset_parts(&mut self) {
        for p in &mut self.parts {
            p.reset();
        }
    }

    /// A document point in calibrated units (C# `MapToCalibrated`).
    pub fn map_to_calibrated(&self, document_point: PointF) -> PointF {
        let p = self.content_transform() * kurbo::Point::new(document_point.x as f64, document_point.y as f64);
        PointF::new(p.x as f32, p.y as f32)
    }

    /// Binds a part property to a float track (C# `BindPart(animations, id, property,
    /// FloatTrack)`): Rotation, Scale, ScaleX, ScaleY, Opacity, TrimStart, TrimEnd (0..1 values
    /// clamped). The track is evaluated at the time passed to
    /// [`draw_list`](DrawingSvgShape::draw_list).
    pub fn bind_part(&mut self, id: &str, property: AnimatedProperty, track: FloatTrack) -> Result<(), String> {
        let part = self.position(id)?;
        match property {
            AnimatedProperty::Rotation | AnimatedProperty::Scale | AnimatedProperty::ScaleX | AnimatedProperty::ScaleY | AnimatedProperty::Opacity | AnimatedProperty::TrimStart | AnimatedProperty::TrimEnd => {}
            other => return Err(format!("A part's {other:?} cannot take a float track; use Rotation, Scale, ScaleX, ScaleY, Opacity, TrimStart or TrimEnd.")),
        }
        self.bindings.push(PartBinding { part, property, track: PartTrack::Float(track) });
        Ok(())
    }

    /// Binds a part property to a point track (C# `BindPart(..., PointTrack)`): Position
    /// (translation), Anchor (pivot), Scale (x, y).
    pub fn bind_part_point(&mut self, id: &str, property: AnimatedProperty, track: PointTrack) -> Result<(), String> {
        let part = self.position(id)?;
        match property {
            AnimatedProperty::Position | AnimatedProperty::Anchor | AnimatedProperty::Scale => {}
            other => return Err(format!("A part's {other:?} cannot take a point track; use Position, Anchor or Scale.")),
        }
        self.bindings.push(PartBinding { part, property, track: PartTrack::Point(track) });
        Ok(())
    }

    /// Applies the bindings at `time` to the parts.
    pub fn apply_bindings(&mut self, time: f64) {
        DrawingSvgShape::bind_into(&self.bindings, &mut self.parts, time);
    }

    /// The images the document wants drawn (the external hrefs and embedded pictures, in
    /// document order, each once): supply their pixels with
    /// [`set_image`](DrawingSvgShape::set_image).
    pub fn image_requests(&self) -> Vec<SvgImageSource> {
        let mut out: Vec<SvgImageSource> = Vec::new();
        for n in self.svg.scene().nodes() {
            if let SvgNodeKind::Image(i) = &n.kind {
                if !out.contains(&i.source) {
                    out.push(i.source.clone());
                }
            }
        }
        out
    }

    /// Supplies the decoded pixels for one of the document's images (an entry of
    /// [`image_requests`](DrawingSvgShape::image_requests)); the crate decodes and fetches
    /// nothing itself. The picture is stretched over the image's box as the document places it.
    pub fn set_image(&mut self, source: &SvgImageSource, pixels: ImageSource) {
        match self.images.iter_mut().find(|(s, _)| s == source) {
            Some(entry) => entry.1 = pixels,
            None => self.images.push((source.clone(), pixels)),
        }
    }

    /// Supplies the decoded pixels for every image of the document that names this external
    /// `href`; false when the document has no such image.
    pub fn set_image_href(&mut self, href: &str, pixels: ImageSource) -> bool {
        let source = SvgImageSource::Href(href.to_string());
        if !self.image_requests().contains(&source) {
            return false;
        }
        self.set_image(&source, pixels);
        true
    }

    /// The pixels supplied for an image, if any.
    pub fn image(&self, source: &SvgImageSource) -> Option<&ImageSource> {
        self.images.iter().find(|(s, _)| match (s, source) {
            (SvgImageSource::Encoded { bytes: a, .. }, SvgImageSource::Encoded { bytes: b, .. }) => Arc::ptr_eq(a, b) || a == b,
            _ => s == source,
        }).map(|(_, p)| p)
    }

    /// What the shape draws at `time` WITHOUT changing its parts: the bindings are applied to a
    /// copy (what the session element draws each frame).
    pub fn draw_list_at(&self, time: f64) -> SvgDrawList {
        if self.bindings.is_empty() {
            return self.list_for(&self.parts, time);
        }
        let mut parts = self.parts.clone();
        DrawingSvgShape::bind_into(&self.bindings, &mut parts, time);
        self.list_for(&parts, time)
    }

    fn bind_into(bindings: &[PartBinding], parts: &mut [DrawingSvgPart], time: f64) {
        for b in bindings {
            let p = &mut parts[b.part];
            match &b.track {
                PartTrack::Float(t) => {
                    let Some(v) = t.evaluate(time) else { continue };
                    match b.property {
                        AnimatedProperty::Rotation => p.rotation = v,
                        AnimatedProperty::Scale => {
                            p.scale_x = v;
                            p.scale_y = v;
                        }
                        AnimatedProperty::ScaleX => p.scale_x = v,
                        AnimatedProperty::ScaleY => p.scale_y = v,
                        AnimatedProperty::Opacity => p.opacity = v.clamp(0.0, 1.0),
                        AnimatedProperty::TrimStart => p.trim_start = v.clamp(0.0, 1.0),
                        AnimatedProperty::TrimEnd => p.trim_end = v.clamp(0.0, 1.0),
                        _ => {}
                    }
                }
                PartTrack::Point(t) => {
                    let Some(v) = t.evaluate(time) else { continue };
                    match b.property {
                        AnimatedProperty::Position => p.translation = v,
                        AnimatedProperty::Anchor => p.pivot = Some(v),
                        AnimatedProperty::Scale => {
                            p.scale_x = v.x;
                            p.scale_y = v.y;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// What the shape draws at `time`, in calibrated units: the bindings applied, the document's
    /// own animations evaluated (when played), the scene walked with both, and the Slice clip.
    pub fn draw_list(&mut self, time: f64) -> SvgDrawList {
        self.apply_bindings(time);
        self.list_for(&self.parts, time)
    }

    fn list_for(&self, parts: &[DrawingSvgPart], time: f64) -> SvgDrawList {
        let frame = if self.play_document_animations && self.svg.has_animations() { Some(self.svg.evaluate_animations(time - self.document_time_offset)) } else { None };
        let mut list = self.svg.walk(frame.as_ref(), parts, self.content_transform());
        if self.aspect == DrawingSvgAspect::Slice {
            let d = self.destination;
            let mut rect = kurbo::BezPath::new();
            rect.move_to((d.x as f64, d.y as f64));
            rect.line_to((d.right() as f64, d.y as f64));
            rect.line_to((d.right() as f64, d.bottom() as f64));
            rect.line_to((d.x as f64, d.bottom() as f64));
            rect.close_path();
            list.commands.insert(0, SvgDrawCommand::PushClip { shapes: vec![SvgClipShape { path: Arc::new(rect), transform: Affine::IDENTITY, rule: SvgFillRule::NonZero }] });
            list.commands.push(SvgDrawCommand::Pop);
        }
        list
    }

    /// Where a part is drawn at `time`, in calibrated units (C# `GetPartBounds`; None for an
    /// unknown id or a part that draws nothing then).
    pub fn get_part_bounds(&mut self, id: &str, time: f64) -> Option<RectangleF> {
        let i = self.svg.scene().part_position(id)?;
        let list = self.draw_list(time);
        list.part_bounds[i].map(|r| RectangleF::new(r.x0 as f32, r.y0 as f32, r.width() as f32, r.height() as f32))
    }
}
