//! `DebugOverlay`: the timecode, frame number and active cues on a translucent plate in a
//! corner of a preview (C# `DebugOverlay`) - and `DebugOverlayCorner`, where it sits.

use std::cell::RefCell;

use kurbo::{Affine, RoundedRect};
use peniko::{Brush, Fill};

use crate::animation::AnimationTimeline;
use crate::color::Color;
use crate::rendering::VectorScene;
use crate::shapes::{TextShape, TextShapeOptions};
use crate::text::FontRegistry;

/// The corner of the frame a [`DebugOverlay`] read-out sits in (C# `DebugOverlayCorner`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DebugOverlayCorner {
    /// The top-left corner (the default).
    #[default]
    TopLeft = 0,
    /// The top-right corner.
    TopRight = 1,
    /// The bottom-left corner.
    BottomLeft = 2,
    /// The bottom-right corner.
    BottomRight = 3,
}

/// A debugging read-out drawn over a preview: the time (a timecode with a timeline, seconds
/// without), "frame N / COUNT" and, if asked, the names of the active cues, on a translucent
/// plate in a corner, in OUTPUT PIXELS (its size does not follow the calibration). OFF by
/// default: a session draws it last, over everything, only while
/// [`DrawingSession::set_debug_overlay`](crate::DrawingSession::set_debug_overlay) holds one.
///
/// It FAILS SOFT: with no registry, no font registered, or an unknown family name, it draws
/// nothing and never fails ([`can_draw_text`](DebugOverlay::can_draw_text) says which) - so a
/// page can switch it on before its font has arrived.
#[derive(Clone, Debug)]
pub struct DebugOverlay {
    /// The registry whose font draws the text; None draws nothing.
    pub fonts: Option<FontRegistry>,
    /// The family; None (the default) for the registry's first font.
    pub family_name: Option<String>,
    /// The timeline for the timecode, frame numbers and cues; None for seconds only.
    pub timeline: Option<AnimationTimeline>,
    /// The moment to show, seconds; None (the default) for the drawing session's time.
    pub time: Option<f64>,
    /// Whether to show the time (default true).
    pub show_timecode: bool,
    /// Whether to show "frame N / COUNT" (needs a timeline; default true).
    pub show_frame_number: bool,
    /// Whether to show the names of the active cues (needs a timeline; default false).
    pub show_cues: bool,
    /// The corner (default top left).
    pub corner: DebugOverlayCorner,
    text_size: f32,
    /// The text colour (default white).
    pub text_color: Color,
    /// The plate's colour, its alpha its opacity (default black at 150 of 255).
    pub plate_color: Color,
    /// The lines shaped for the last read-out, kept so that a line is shaped again only when
    /// its text (or the font, size or colour) changes.
    shaped: RefCell<ShapedLines>,
}

/// The overlay's shaped lines and what they were shaped with.
#[derive(Clone, Debug, Default)]
struct ShapedLines {
    /// The registry, its font count, the family, the size's bits and the colour.
    with: Option<(FontRegistry, usize, Option<String>, u32, Color)>,
    /// Each line's text and its shape.
    lines: Vec<(String, TextShape)>,
}

#[cfg(test)]
thread_local! {
    /// Lines the overlay has shaped (tests).
    pub(crate) static OVERLAY_LINES_SHAPED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl DebugOverlay {
    /// The default text colour: white.
    pub const DEFAULT_TEXT_COLOR: Color = Color::from_rgb(255, 255, 255);
    /// The default plate colour: black at a little over half opacity.
    pub const DEFAULT_PLATE_COLOR: Color = Color::from_rgba(0, 0, 0, 150);

    /// An overlay (C# `DebugOverlay(fonts, timeline = null)`).
    pub fn new(fonts: Option<FontRegistry>, timeline: Option<AnimationTimeline>) -> DebugOverlay {
        DebugOverlay {
            fonts,
            family_name: None,
            timeline,
            time: None,
            show_timecode: true,
            show_frame_number: true,
            show_cues: false,
            corner: DebugOverlayCorner::TopLeft,
            text_size: 0.0,
            text_color: DebugOverlay::DEFAULT_TEXT_COLOR,
            plate_color: DebugOverlay::DEFAULT_PLATE_COLOR,
            shaped: RefCell::new(ShapedLines::default()),
        }
    }

    /// The text size in output pixels; 0 (the default) picks one from the frame's height (a
    /// thirty-sixth of it, at least 10).
    pub fn text_size(&self) -> f32 {
        self.text_size
    }

    /// Sets the text size. The error (C# ArgumentOutOfRangeException) says what to do.
    pub fn set_text_size(&mut self, value: f32) -> Result<(), String> {
        if !value.is_finite() || value < 0.0 {
            return Err("A debug overlay's text size must be 0 (automatic) or a positive number of pixels.".to_string());
        }
        self.text_size = value;
        Ok(())
    }

    /// True when the overlay can draw: a registry is set and holds the font it would use.
    pub fn can_draw_text(&self) -> bool {
        self.fonts.as_ref().is_some_and(|f| f.count() > 0 && f.contains(self.family_name.as_deref()))
    }

    /// The lines it shows at a moment, top to bottom, whether or not a font is available.
    pub fn get_lines(&self, seconds: f64) -> Vec<String> {
        let mut lines = Vec::new();
        let time = format!("{seconds:.3} s");
        if self.show_timecode {
            lines.push(match &self.timeline {
                Some(t) => format!("{}   {time}", t.format_timecode(seconds)),
                None => time,
            });
        }
        if let Some(timeline) = &self.timeline {
            if self.show_frame_number {
                lines.push(format!("frame {} / {}", timeline.frame_at(seconds), timeline.frame_count()));
            }
            if self.show_cues {
                let names: Vec<&str> = timeline.get_active_cues(seconds).iter().map(|c| c.name()).collect();
                lines.push(if names.is_empty() { "cues: none".to_string() } else { format!("cues: {}", names.join(", ")) });
            }
        }
        lines
    }

    /// Records the read-out over a frame of `width` x `height` output pixels, for the moment
    /// `session_time` (unless [`time`](DebugOverlay::time) is set). Draws nothing when it cannot.
    pub fn render(&self, scene: &mut VectorScene, width: u32, height: u32, session_time: f64) {
        self.render_at(scene, kurbo::Rect::new(0.0, 0.0, width as f64, height as f64), session_time);
    }

    /// Records the read-out over the part `area` of the canvas (output pixels).
    pub(crate) fn render_at(&self, scene: &mut VectorScene, area: kurbo::Rect, session_time: f64) {
        let Some(fonts) = self.fonts.as_ref().filter(|_| self.can_draw_text()) else {
            return;
        };
        let (width, height) = (area.width() as f32, area.height() as f32);
        if width < 1.0 || height < 1.0 {
            return;
        }
        let lines = self.get_lines(self.time.unwrap_or(session_time));
        if lines.is_empty() {
            return;
        }
        let size = if self.text_size > 0.0 { self.text_size } else { (height / 36.0).max(10.0) };
        let line_height = size * 1.3;
        let pad = size * 0.45;
        let margin = size * 0.6;
        // Each line shaped once: a line whose text is the same as last time's keeps its shape.
        let mut shaped = self.shaped.borrow_mut();
        let with = Some((fonts.clone(), fonts.count(), self.family_name.clone(), size.to_bits(), self.text_color));
        if shaped.with != with {
            shaped.with = with;
            shaped.lines.clear();
        }
        let mut kept = std::mem::take(&mut shaped.lines);
        for line in &lines {
            if let Some(i) = kept.iter().position(|(text, _)| text == line) {
                let text = kept.swap_remove(i);
                shaped.lines.push(text);
                continue;
            }
            #[cfg(test)]
            OVERLAY_LINES_SHAPED.with(|n| n.set(n.get() + 1));
            match TextShape::new(line, 0.0, 0.0, size, fonts, TextShapeOptions { color: Some(self.text_color), family_name: self.family_name.clone(), ..TextShapeOptions::default() }) {
                Ok(text) => shaped.lines.push((line.clone(), text)),
                Err(_) => {
                    shaped.lines.clear();
                    return;
                }
            }
        }
        let texts: Vec<&TextShape> = shaped.lines.iter().map(|(_, text)| text).collect();
        let text_width = texts.iter().map(|t| t.width()).fold(0.0f32, f32::max);
        let plate_width = text_width + 2.0 * pad;
        let plate_height = lines.len() as f32 * line_height + 2.0 * pad - (line_height - size);
        let right = matches!(self.corner, DebugOverlayCorner::TopRight | DebugOverlayCorner::BottomRight);
        let bottom = matches!(self.corner, DebugOverlayCorner::BottomLeft | DebugOverlayCorner::BottomRight);
        let left = area.x0 as f32 + if right { width - margin - plate_width } else { margin };
        let top = area.y0 as f32 + if bottom { height - margin - plate_height } else { margin };
        if self.plate_color.a > 0 {
            let plate = RoundedRect::new(left as f64, top as f64, (left + plate_width) as f64, (top + plate_height) as f64, (pad * 0.6) as f64);
            scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(self.plate_color.to_peniko(1.0)), None, &plate);
        }
        for (i, text) in texts.iter().enumerate() {
            let at = Affine::translate(((left + pad) as f64, (top + pad + size * 0.8 + i as f32 * line_height) as f64));
            text.encode(scene, at, self.text_color, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::EasingKind;

    /// The lines overlays have shaped on this thread.
    fn shaped() -> usize {
        OVERLAY_LINES_SHAPED.with(|n| n.get())
    }

    /// An overlay's read-out at a time: its elements and the geometry keys of its fills.
    fn record(overlay: &DebugOverlay, time: f64) -> (Vec<kurbo::PathEl>, Vec<u64>) {
        let mut scene = VectorScene::new();
        overlay.render(&mut scene, 1280, 720, time);
        (scene.elements.clone(), scene.fill_keys())
    }

    #[test]
    fn the_debug_overlay_shapes_a_line_only_when_its_text_changes() {
        let Some(fonts) = crate::text::layout_tests::fonts() else { return };
        //Arrange
        let mut timeline = AnimationTimeline::new(10.0, 30.0).unwrap();
        timeline.add_cue("intro", 0.0, 2.0, EasingKind::Linear).unwrap();
        let mut overlay = DebugOverlay::new(Some(fonts), Some(timeline));
        overlay.show_cues = true;
        let start = shaped();
        //Act
        let first = record(&overlay, 1.5);
        let still: Vec<_> = (0..5).map(|_| record(&overlay, 1.5)).collect();
        let after_still = shaped();
        let moved = record(&overlay, 1.6);
        let after_move = shaped();
        let fresh = record(&DebugOverlay { shaped: RefCell::new(ShapedLines::default()), ..overlay.clone() }, 1.6);
        overlay.set_text_size(30.0).unwrap();
        let after_fresh = shaped();
        record(&overlay, 1.6);
        //Assert
        assert_eq!(after_still - start, 3, "three lines, shaped once");
        assert!(still.iter().all(|f| *f == first), "the same read-out every frame");
        assert_eq!(after_move - after_still, 2, "the time and frame lines changed, the cue line did not");
        assert_eq!(moved, fresh, "kept lines draw as freshly shaped ones");
        assert_eq!(shaped() - after_fresh, 3, "a new size shapes every line again");
    }
}
