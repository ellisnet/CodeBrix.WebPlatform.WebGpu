//! `CaptionPresenter`: which cues show at a time and where their boxes and text go (C#
//! `CaptionPresenter`, the calculation half: the shapes are drawn from its placed captions).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::captions::{CaptionPlacement, CaptionStyle, CaptionTrack, PlacedCaption};
use crate::geometry::RectangleF;
use crate::layout::{DrawingLayout, LayoutPosition, LayoutReference};
use crate::text::{FontRegistry, TextLayoutEngine, TextLayoutOptions, TextLayoutResult, TextTrimming};

/// A cue laid out once: its text, style, size, position, the rectangle its position is measured
/// in, and its box's size.
#[derive(Debug)]
struct CueBox {
    text: Rc<TextLayoutResult>,
    style: CaptionStyle,
    size: f32,
    position: LayoutPosition,
    reference: RectangleF,
    box_size: (f32, f32),
}

/// A column of cues sharing a position and caption area: the position, the area, the cues.
type Column = (LayoutPosition, RectangleF, Vec<(usize, Rc<CueBox>)>);

/// Shows a caption track (C# `CaptionPresenter`): for any time passed in, the cues showing and
/// where their boxes and text go - cues sharing a position stack into one column in start order,
/// the earliest on top, `cue_spacing` apart, the column's box anchored at the position. Each cue
/// is laid out once and kept; the placed captions of a set of showing cues are kept too (up to
/// 256 sets), so a frame costs a lookup while the same cues show. Nothing here reads a clock.
///
/// PLACEMENT (the owner's rule - a deliberate difference from C#): by default captions sit
/// against the FRAME of the layout, the style's margin from its edges
/// ([`CaptionPlacement::Frame`]); [`with_placement`](CaptionPresenter::with_placement) with
/// [`CaptionPlacement::SafeAreaGuide`] places them inside a safe-area guide as C# does.
#[derive(Debug)]
pub struct CaptionPresenter {
    track: CaptionTrack,
    fonts: FontRegistry,
    layout: DrawingLayout,
    style: CaptionStyle,
    placement: CaptionPlacement,
    cue_boxes: RefCell<HashMap<usize, Rc<CueBox>>>,
    placed: RefCell<HashMap<Vec<usize>, Rc<Vec<PlacedCaption>>>>,
}

impl CaptionPresenter {
    /// A presenter of a track, laid out in a registry's fonts against a layout's frame, in a
    /// style - None for the defaults (C# `CaptionPresenter(track, fonts, layout, style = null)`).
    /// The error says which style member is out of range.
    pub fn new(track: CaptionTrack, fonts: &FontRegistry, layout: DrawingLayout, style: Option<CaptionStyle>) -> Result<CaptionPresenter, String> {
        let style = style.unwrap_or_default();
        style.validate()?;
        Ok(CaptionPresenter { track, fonts: fonts.clone(), layout, style, placement: CaptionPlacement::Frame, cue_boxes: RefCell::new(HashMap::new()), placed: RefCell::new(HashMap::new()) })
    }

    /// The same presenter with another caption area (no C# counterpart): the frame with the
    /// style's margin (the default) or inside a safe-area guide.
    pub fn with_placement(mut self, placement: CaptionPlacement) -> CaptionPresenter {
        self.placement = placement;
        self.cue_boxes.borrow_mut().clear();
        self.placed.borrow_mut().clear();
        self
    }

    /// The track.
    pub fn track(&self) -> &CaptionTrack {
        &self.track
    }

    /// The fonts (the same registry handle).
    pub fn fonts(&self) -> &FontRegistry {
        &self.fonts
    }

    /// The layout.
    pub fn layout(&self) -> &DrawingLayout {
        &self.layout
    }

    /// The style (a copy, as C#'s `Style`).
    pub fn style(&self) -> CaptionStyle {
        self.style.clone()
    }

    /// The caption area choice.
    pub fn placement(&self) -> CaptionPlacement {
        self.placement
    }

    /// The captions showing at a time, placed (C# `GetShapeAt`, which returns them as shapes):
    /// empty between cues; the SAME `Rc` while the same cues show. The error is a layout error of
    /// a cue (an unknown family, for example).
    pub fn get_placed_captions_at(&self, seconds: f64) -> Result<Rc<Vec<PlacedCaption>>, String> {
        let active = self.track.get_active_cue_indices(seconds);
        if let Some(found) = self.placed.borrow().get(&active) {
            return Ok(found.clone());
        }
        let built = Rc::new(self.build(&active)?);
        let mut placed = self.placed.borrow_mut();
        if placed.len() >= 256 {
            placed.clear();
        }
        placed.insert(active, built.clone());
        Ok(built)
    }

    /// A cue's laid-out text (C# `GetCueLayout`). The error says what is wrong: an index past the
    /// last cue, or the cue's layout error.
    pub fn get_cue_layout(&self, cue_index: usize) -> Result<Rc<TextLayoutResult>, String> {
        if cue_index >= self.track.count() {
            return Err(format!("There is no cue {cue_index}: the track has {} cues (indices from 0).", self.track.count()));
        }
        Ok(self.get_cue_box(cue_index)?.text.clone())
    }

    /// The caption area for a cue of a style and size.
    fn caption_area(&self, style: &CaptionStyle, size: f32) -> RectangleF {
        match self.placement {
            CaptionPlacement::SafeAreaGuide(rect) => rect,
            CaptionPlacement::Frame => {
                let f = self.layout.frame();
                let m = style.margin * size;
                RectangleF::new(f.x + m, f.y + m, (f.width - 2.0 * m).max(0.0), (f.height - 2.0 * m).max(0.0))
            }
        }
    }

    fn get_cue_box(&self, index: usize) -> Result<Rc<CueBox>, String> {
        if let Some(found) = self.cue_boxes.borrow().get(&index) {
            return Ok(found.clone());
        }
        let cue = &self.track.cues()[index];
        let style = match cue.style() {
            Some(own) => {
                own.validate()?;
                own.clone()
            }
            None => self.style.clone(),
        };
        let size = if style.size > 0.0 { style.size } else { (self.layout.short_side() * 0.045).max(1.0) };
        let position = cue.position().unwrap_or(style.position);
        let reference = match position.reference() {
            LayoutReference::Frame => self.layout.frame(),
            LayoutReference::SafeArea => self.caption_area(&style, size),
        };
        let (pad_x, pad_y) = (style.padding_x * size, style.padding_y * size);
        let options = TextLayoutOptions {
            max_width: Some(size.max((reference.width * style.max_width_fraction) - (2.0 * pad_x))),
            align: cue.align().unwrap_or(style.align),
            line_spacing: style.line_spacing,
            max_lines: style.max_lines,
            trimming: if style.max_lines > 0 { TextTrimming::WordEllipsis } else { TextTrimming::None },
            missing_characters: style.missing_characters,
            ..TextLayoutOptions::default()
        };
        let text = TextLayoutEngine::layout(&self.fonts, cue.text(), size, style.family_name.as_deref(), Some(&options))?;
        let content = text.content_bounds();
        let built = Rc::new(CueBox { box_size: (content.width + (2.0 * pad_x), content.height + (2.0 * pad_y)), text, style, size, position, reference });
        self.cue_boxes.borrow_mut().insert(index, built.clone());
        Ok(built)
    }

    fn build(&self, active: &[usize]) -> Result<Vec<PlacedCaption>, String> {
        // Cues sharing a position (and caption area) stack into one column, in start order.
        let mut columns: Vec<Column> = Vec::new();
        for &index in active {
            let cue_box = self.get_cue_box(index)?;
            match columns.iter_mut().find(|(p, r, _)| *p == cue_box.position && *r == cue_box.reference) {
                Some(column) => column.2.push((index, cue_box)),
                None => columns.push((cue_box.position, cue_box.reference, vec![(index, cue_box)])),
            }
        }
        let mut placed = Vec::new();
        for (position, reference, boxes) in &columns {
            let spacing = self.style.cue_spacing * boxes[0].1.size;
            let width = boxes.iter().fold(0.0f32, |w, (_, b)| w.max(b.box_size.0));
            let height = boxes.iter().map(|(_, b)| b.box_size.1).sum::<f32>() + spacing * (boxes.len() as f32 - 1.0);
            let stack = place_box(*reference, (width, height), position);
            let mut y = stack.y;
            for (index, b) in boxes {
                let x = stack.x + ((stack.width - b.box_size.0) * position.anchor().x());
                let rect = RectangleF::new(x, y, b.box_size.0, b.box_size.1);
                let content = b.text.content_bounds();
                let style = &b.style;
                placed.push(PlacedCaption {
                    cue_index: *index,
                    box_bounds: rect,
                    background_color: style.background_color,
                    corner_radius: style.corner_radius * b.size,
                    text: b.text.clone(),
                    text_x: rect.x + (style.padding_x * b.size) - content.x,
                    text_y: rect.y + (style.padding_y * b.size) - content.y,
                    text_color: style.text_color,
                    text_fill: style.text_fill.clone(),
                    outline: match style.outline_color {
                        Some(color) if style.outline_width > 0.0 => Some((color, 2.0 * style.outline_width * b.size)),
                        _ => None,
                    },
                });
                y += b.box_size.1 + spacing;
            }
        }
        Ok(placed)
    }
}

/// A box of a size whose own anchor (the position's) lies at the position measured in a rectangle
/// (C# `DrawingLayout.PlaceBox`, with the rectangle given).
fn place_box(reference: RectangleF, size: (f32, f32), position: &LayoutPosition) -> RectangleF {
    let a = position.anchor().point_in(reference);
    let (px, py) = (a.x + (position.fraction_offset().x * reference.width) + position.offset().x, a.y + (position.fraction_offset().y * reference.height) + position.offset().y);
    RectangleF::new(px - (position.anchor().x() * size.0), py - (position.anchor().y() * size.1), size.0, size.1)
}
