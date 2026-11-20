//! `PathShape`: a path (or a morph between two paths), stroked with a full stroke style, filled,
//! or both, drawn in part by its TRIM (C# `PathShape`).

use kurbo::Affine;
use peniko::Brush;

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::rendering::VectorScene;
use crate::fills::ShapeFill;
use crate::paths::{PathGeometry, PathMorph, PathStrokeStyle, PathStrokeStyleOptions};
use crate::shapes::basic_shapes::check_thickness;
use crate::shapes::shape_options::ShapeOptions;

/// A path shape: its geometry (with its fill rule), an optional stroke style and stroke colour,
/// and an optional fill (a colour or a [`ShapeFill`]). The fill is drawn first, the stroke over
/// it.
#[derive(Clone, Debug, PartialEq)]
pub struct PathShape {
    pub(crate) geometry: PathGeometry,
    pub(crate) stroke_style: Option<PathStrokeStyle>,
    pub(crate) stroke_color: Option<Color>,
    pub(crate) is_filled: bool,
    pub(crate) fill_color: Option<Color>,
    pub(crate) fill: Option<ShapeFill>,
    pub(crate) trim_start: f32,
    pub(crate) trim_end: f32,
    pub(crate) trim_offset: f32,
    pub(crate) morph: Option<PathMorph>,
    pub(crate) morph_fraction: f32,
}

/// A fraction 0..1, or the error saying so.
fn check_fraction(value: f32, name: &str) -> Result<f32, String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{name} must be a fraction from 0 to 1."))
    }
}

impl PathShape {
    /// An outlined (round caps and joins) or filled path in one colour (C# `PathShape(geometry,
    /// strokeThickness = 15, color = null, isFilled = false)`).
    pub fn new(geometry: PathGeometry, options: ShapeOptions) -> Result<PathShape, String> {
        check_thickness(options.stroke_thickness)?;
        if options.is_filled {
            Ok(PathShape { geometry, stroke_style: None, stroke_color: None, is_filled: true, fill_color: options.color, fill: None, ..PathShape::plain() })
        } else {
            Ok(PathShape { geometry, stroke_style: Some(PathStrokeStyle::new(options.stroke_thickness, PathStrokeStyleOptions::default())?), stroke_color: options.color, is_filled: false, fill_color: None, fill: None, ..PathShape::plain() })
        }
    }

    /// A path filled with a colour (C# `PathShape(geometry, fillColor)`).
    pub fn with_fill_color(geometry: PathGeometry, fill_color: Color) -> PathShape {
        PathShape { geometry, stroke_style: None, stroke_color: None, is_filled: true, fill_color: Some(fill_color), fill: None, ..PathShape::plain() }
    }

    /// A path painted by a fill (C# `PathShape(geometry, fill)`).
    pub fn with_fill(geometry: PathGeometry, fill: ShapeFill) -> PathShape {
        PathShape { geometry, stroke_style: None, stroke_color: None, is_filled: true, fill_color: None, fill: Some(fill), ..PathShape::plain() }
    }

    /// A path stroked with a style, and filled when a fill colour or fill is given (C#
    /// `PathShape(geometry, strokeStyle, strokeColor = null, fillColor = null, fill = null)`).
    pub fn with_style(geometry: PathGeometry, stroke_style: PathStrokeStyle, stroke_color: Option<Color>, fill_color: Option<Color>, fill: Option<ShapeFill>) -> PathShape {
        let is_filled = fill_color.is_some() || fill.is_some();
        PathShape { geometry, stroke_style: Some(stroke_style), stroke_color, is_filled, fill_color, fill, ..PathShape::plain() }
    }

    /// The geometry.
    pub fn geometry(&self) -> &PathGeometry {
        &self.geometry
    }

    /// The defaults of the trim and morph members (whole path, no morph), and an empty path.
    fn plain() -> PathShape {
        PathShape { geometry: PathGeometry::empty(), stroke_style: None, stroke_color: None, is_filled: false, fill_color: None, fill: None, trim_start: 0.0, trim_end: 1.0, trim_offset: 0.0, morph: None, morph_fraction: 0.0 }
    }

    /// A shape drawing a MORPH at a fraction, outlined or filled in one colour (C#
    /// `PathShape(PathMorph morph, float fraction, strokeThickness, color, isFilled)`).
    pub fn from_morph(morph: PathMorph, fraction: f32, options: ShapeOptions) -> Result<PathShape, String> {
        let fraction = check_fraction(fraction, "A morph fraction")?;
        let mut shape = PathShape::new(morph.get_path(fraction), options)?;
        shape.morph = Some(morph);
        shape.morph_fraction = fraction;
        Ok(shape)
    }

    /// A shape drawing a MORPH with a full stroke style, a stroke colour, and a fill colour or
    /// gradient fill (C# `PathShape(PathMorph morph, float fraction, PathStrokeStyle strokeStyle,
    /// strokeColor, fillColor, fill)`).
    pub fn with_morph(morph: PathMorph, fraction: f32, stroke_style: PathStrokeStyle, stroke_color: Option<Color>, fill_color: Option<Color>, fill: Option<ShapeFill>) -> Result<PathShape, String> {
        let fraction = check_fraction(fraction, "A morph fraction")?;
        let mut shape = PathShape::with_style(morph.get_path(fraction), stroke_style, stroke_color, fill_color, fill);
        shape.morph = Some(morph);
        shape.morph_fraction = fraction;
        Ok(shape)
    }

    /// The morph it draws, if it was made from one (C# `Morph`).
    pub fn morph(&self) -> Option<&PathMorph> {
        self.morph.as_ref()
    }

    /// Where between the morph's two paths it is, 0..1 (C# `MorphFraction`).
    pub fn morph_fraction(&self) -> f32 {
        self.morph_fraction
    }

    /// Sets the morph fraction. The error says what to do on a shape without a morph or with a
    /// value outside 0..1.
    pub fn set_morph_fraction(&mut self, value: f32) -> Result<(), String> {
        let Some(morph) = &self.morph else {
            return Err("Only a shape made from a PathMorph has a morph fraction: make it with PathShape::from_morph or with_morph.".to_string());
        };
        let value = check_fraction(value, "A morph fraction")?;
        if value != self.morph_fraction {
            self.geometry = morph.get_path(value);
            self.morph_fraction = value;
        }
        Ok(())
    }

    /// Where the drawn part starts, a fraction of the length 0..1 (C# `TrimStart`, default 0).
    pub fn trim_start(&self) -> f32 {
        self.trim_start
    }

    /// Sets the trim start. The error says what to do with a value outside 0..1.
    pub fn set_trim_start(&mut self, value: f32) -> Result<(), String> {
        self.trim_start = check_fraction(value, "A trim start")?;
        Ok(())
    }

    /// Where the drawn part ends, a fraction of the length 0..1 (C# `TrimEnd`, default 1):
    /// animate it from 0 to 1 to draw the path on.
    pub fn trim_end(&self) -> f32 {
        self.trim_end
    }

    /// Sets the trim end. The error says what to do with a value outside 0..1.
    pub fn set_trim_end(&mut self, value: f32) -> Result<(), String> {
        self.trim_end = check_fraction(value, "A trim end")?;
        Ok(())
    }

    /// How far the trimmed window is moved round the path, in lengths (C# `TrimOffset`, default
    /// 0; it wraps).
    pub fn trim_offset(&self) -> f32 {
        self.trim_offset
    }

    /// Sets the trim offset. The error says what to do with a value that is not finite.
    pub fn set_trim_offset(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err("A trim offset must be a finite number of lengths.".to_string());
        }
        self.trim_offset = value;
        Ok(())
    }

    /// True when only part of the path is drawn (C# `IsTrimmed`).
    pub fn is_trimmed(&self) -> bool {
        self.trim_start != 0.0 || self.trim_end != 1.0
    }

    /// The path as drawn: the morph at its fraction (or the geometry), trimmed (C#
    /// `GetDrawnGeometry`). The trim applies to the fill too.
    pub fn get_drawn_geometry(&self) -> PathGeometry {
        if !self.is_trimmed() {
            return self.geometry.clone();
        }
        self.geometry.trim(self.trim_start, self.trim_end, self.trim_offset).unwrap_or_default()
    }

    /// The stroke style, when stroked.
    pub fn stroke_style(&self) -> Option<&PathStrokeStyle> {
        self.stroke_style.as_ref()
    }

    /// True when stroked.
    pub fn is_stroked(&self) -> bool {
        self.stroke_style.is_some()
    }

    /// True when filled.
    pub fn is_filled(&self) -> bool {
        self.is_filled
    }

    /// The fill colour, when filled with a colour.
    pub fn fill_color(&self) -> Option<Color> {
        self.fill_color
    }

    /// The path's bounds, grown by half the stroke where it is stroked (C# `GetBounds`).
    pub fn get_bounds(&self) -> RectangleF {
        let drawn = self.get_drawn_geometry();
        if drawn.is_empty() {
            let start = self.geometry.get_point_at(self.trim_start).position;
            return RectangleF::new(start.x, start.y, 0.0, 0.0);
        }
        let b = drawn.get_bounds();
        match &self.stroke_style {
            None => b,
            Some(style) => {
                let half = style.width() / 2.0 * style.reach_factor();
                RectangleF::new(b.x - half, b.y - half, b.width + 2.0 * half, b.height + 2.0 * half)
            }
        }
    }

    /// Records the fill, then the stroke.
    pub(crate) fn encode(&self, scene: &mut VectorScene, transform: Affine, inherited: Color, opacity: f32) {
        let drawn = if self.is_trimmed() { Some(self.get_drawn_geometry()) } else { None };
        let geometry = drawn.as_ref().unwrap_or(&self.geometry);
        if geometry.is_empty() {
            return;
        }
        let path = geometry.bez_path();
        if self.is_filled {
            let brush = match &self.fill {
                Some(fill) => fill.to_brush(opacity),
                None => Brush::Solid(self.fill_color.or(self.stroke_color).unwrap_or(inherited).to_peniko(opacity)),
            };
            scene.fill(geometry.fill_rule().to_peniko(), transform, &brush, None, path);
        }
        if let Some(style) = &self.stroke_style {
            scene.stroke(&style.to_kurbo(), transform, &Brush::Solid(self.stroke_color.unwrap_or(inherited).to_peniko(opacity)), None, path);
        }
    }
}
