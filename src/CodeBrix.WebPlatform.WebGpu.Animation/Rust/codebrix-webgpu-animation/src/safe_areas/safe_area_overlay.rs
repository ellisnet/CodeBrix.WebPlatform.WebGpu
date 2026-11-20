//! `SafeAreaOverlay`: the preview mask over a guide's unsafe zones and a ring round its safe
//! rectangle (C# `SafeAreaOverlay`) - its settings, and the rectangles a renderer fills to draw
//! it ([`SafeAreaOverlayPlan`]). The drawing itself is the renderer's.

use crate::color::Color;
use crate::geometry::RectangleF;
use crate::safe_areas::{SafeAreaGuide, SafeAreaZone};

/// What a renderer fills to draw an overlay in a frame (no C# type: C#'s `Draw` paints these on
/// its canvas): each group of rectangles is painted OPAQUE in its colour into one layer
/// composited once at its alpha, so overlaps never darken and corners are never painted twice
/// (C#'s `SaveLayer` "highlighter"); no anti-aliasing. Frame pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaOverlayPlan {
    /// The unsafe zones: (opaque colour, layer alpha, rectangles); None when nothing is masked.
    pub mask: Option<(Color, u8, Vec<RectangleF>)>,
    /// The ring just outside the safe rectangle as four bands (top, bottom, left, right; top and
    /// bottom span the corners): (opaque colour, layer alpha, bands); None without a ring.
    pub outline: Option<(Color, u8, [RectangleF; 4])>,
}

/// C#'s `DrawingRenderer.ScaleAlpha`: an alpha times a factor, rounded half away from zero.
fn scale_alpha(alpha: u8, factor: f32) -> u8 {
    if factor >= 1.0 {
        alpha
    } else {
        (alpha as f64 * factor as f64).round() as u8
    }
}

/// The safe-area overlay of a preview (C# `SafeAreaOverlay`): a translucent mask over the
/// guide's unsafe zones and a ring just outside its safe rectangle, at the frame's pixel size and
/// the session's time (or a fixed `time`). A PREVIEW AID: it is drawn over the frame, after the
/// layers and before the debug overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaOverlay {
    guide: SafeAreaGuide,
    /// The mask's colour (C# `MaskColor`; a translucent pink by default).
    pub mask_color: Color,
    opacity: f32,
    /// The ring's colour (C# `OutlineColor`).
    pub outline_color: Color,
    outline_width: f32,
    /// A fixed time for the guide's time rule (None: the session's) (C# `Time`).
    pub time: Option<f64>,
    /// The video's duration, for final-seconds rules (C# `Duration`).
    pub duration: Option<f64>,
}

impl SafeAreaOverlay {
    /// The default mask colour (C# `DefaultMaskColor`: 255, 64, 112 at 40).
    pub const DEFAULT_MASK_COLOR: Color = Color::from_rgba(255, 64, 112, 40);
    /// The default ring colour (C# `DefaultOutlineColor`: 255, 64, 112 at 230).
    pub const DEFAULT_OUTLINE_COLOR: Color = Color::from_rgba(255, 64, 112, 230);
    /// The default ring width, frame pixels (C# `DefaultOutlineWidth`).
    pub const DEFAULT_OUTLINE_WIDTH: f32 = 2.0;

    /// An overlay of a guide with the default look (C#'s constructor).
    pub fn new(guide: SafeAreaGuide) -> SafeAreaOverlay {
        SafeAreaOverlay { guide, mask_color: SafeAreaOverlay::DEFAULT_MASK_COLOR, opacity: 1.0, outline_color: SafeAreaOverlay::DEFAULT_OUTLINE_COLOR, outline_width: SafeAreaOverlay::DEFAULT_OUTLINE_WIDTH, time: None, duration: None }
    }

    /// The guide (C# `Guide`).
    pub fn guide(&self) -> &SafeAreaGuide {
        &self.guide
    }

    /// Sets the guide.
    pub fn set_guide(&mut self, guide: SafeAreaGuide) {
        self.guide = guide;
    }

    /// The mask's opacity, 0..1, on top of its colour's alpha (C# `Opacity`).
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Sets the opacity; the error says what to do with a value outside 0..1.
    pub fn set_opacity(&mut self, value: f32) -> Result<(), String> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err("The overlay's opacity must be 0..1.".to_string());
        }
        self.opacity = value;
        Ok(())
    }

    /// The ring's width, frame pixels; 0 draws no ring (C# `OutlineWidth`).
    pub fn outline_width(&self) -> f32 {
        self.outline_width
    }

    /// Sets the ring's width; the error says what to do with a negative or non-finite value.
    pub fn set_outline_width(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() || value < 0.0 {
            return Err("The ring's width must be a finite number of pixels, 0 or more.".to_string());
        }
        self.outline_width = value;
        Ok(())
    }

    /// The unsafe zones in a frame at a time (C# `GetZones`).
    pub fn get_zones(&self, frame_size: (f32, f32), seconds: f64) -> Vec<SafeAreaZone> {
        self.guide.get_unsafe_zones(frame_size, Some(seconds), self.duration)
    }

    /// What to fill for a frame of `frame_size` pixels shown at `renderer_time` (C#'s `Draw`,
    /// without the canvas): the zones at the mask colour's alpha times the opacity, and the ring
    /// at the ring colour's alpha.
    pub fn get_draw_plan(&self, frame_size: (f32, f32), renderer_time: f64) -> SafeAreaOverlayPlan {
        let seconds = self.time.unwrap_or(renderer_time);
        let zones = self.get_zones(frame_size, seconds);
        let mut plan = SafeAreaOverlayPlan { mask: None, outline: None };
        if zones.is_empty() {
            return plan;
        }
        let alpha = scale_alpha(self.mask_color.a, self.opacity);
        if alpha > 0 {
            plan.mask = Some((self.mask_color.with_alpha(255), alpha, zones.iter().map(|z| z.bounds()).collect()));
        }
        let width = self.outline_width;
        if width <= 0.0 || self.outline_color.a == 0 {
            return plan;
        }
        let safe = self.guide.get_safe_rectangle(frame_size, Some(seconds), self.duration);
        if safe.width <= 0.0 || safe.height <= 0.0 {
            return plan;
        }
        let (left, top, right, bottom) = (safe.x, safe.y, safe.right(), safe.bottom());
        let ltrb = |l: f32, t: f32, r: f32, b: f32| RectangleF::new(l, t, r - l, b - t);
        plan.outline = Some((
            self.outline_color.with_alpha(255),
            self.outline_color.a,
            [ltrb(left - width, top - width, right + width, top), ltrb(left - width, bottom, right + width, bottom + width), ltrb(left - width, top, left, bottom), ltrb(right, top, right + width, bottom)],
        ));
        plan
    }
}
