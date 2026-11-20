//! `SvgStroke`: a path's stroke with everything kurbo's `Stroke` takes, and the paint order.

use crate::svg::svg_paint::SvgPaint;

/// How a stroke's ends look (`stroke-linecap`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgLineCap {
    /// `butt`.
    #[default]
    Butt,
    /// `round`.
    Round,
    /// `square`.
    Square,
}

/// How a stroke's corners look (`stroke-linejoin`; `miter-clip` arrives as `Miter`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgLineJoin {
    /// `miter` (and `miter-clip`, drawn as miter).
    #[default]
    Miter,
    /// `round`.
    Round,
    /// `bevel`.
    Bevel,
}

/// A stroke: paint, opacity, width, caps, joins, miter limit and dashes - everything kurbo's
/// `Stroke` takes.
#[derive(Clone, Debug, PartialEq)]
pub struct SvgStroke {
    /// The paint.
    pub paint: SvgPaint,
    /// `stroke-opacity`, 0..1.
    pub opacity: f32,
    /// `stroke-width`, in the path's own units.
    pub width: f32,
    /// `stroke-linecap`.
    pub cap: SvgLineCap,
    /// `stroke-linejoin`.
    pub join: SvgLineJoin,
    /// `stroke-miterlimit`.
    pub miter_limit: f32,
    /// `stroke-dasharray` (an even count; None: solid).
    pub dash_array: Option<Vec<f32>>,
    /// `stroke-dashoffset`.
    pub dash_offset: f32,
}

impl SvgStroke {
    /// The stroke as kurbo's `Stroke` (what the renderer's `stroke` takes).
    pub fn to_kurbo(&self) -> kurbo::Stroke {
        let mut s = kurbo::Stroke::new(self.width as f64)
            .with_caps(match self.cap {
                SvgLineCap::Butt => kurbo::Cap::Butt,
                SvgLineCap::Round => kurbo::Cap::Round,
                SvgLineCap::Square => kurbo::Cap::Square,
            })
            .with_join(match self.join {
                SvgLineJoin::Miter => kurbo::Join::Miter,
                SvgLineJoin::Round => kurbo::Join::Round,
                SvgLineJoin::Bevel => kurbo::Join::Bevel,
            })
            .with_miter_limit(self.miter_limit as f64);
        if let Some(d) = &self.dash_array {
            s = s.with_dashes(self.dash_offset as f64, d.iter().map(|v| *v as f64));
        }
        s
    }
}

/// Whether a path is filled first (the SVG default) or stroked first (`paint-order: stroke`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SvgPaintOrder {
    /// Fill, then stroke.
    #[default]
    FillAndStroke,
    /// Stroke, then fill.
    StrokeAndFill,
}
