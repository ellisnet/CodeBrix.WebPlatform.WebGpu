//! `TextRunDescriptor`: one styled run of a text block (C# `TextRunDescriptor`).

use crate::color::Color;
use crate::fills::ShapeFill;

/// One run of a text block: its text and the font, size and tracking it is laid out in, and the
/// colour or fill it is painted in (C# `TextRunDescriptor`). The runs' texts are joined in order
/// into the block's text; kerning applies inside a run, not across a run boundary; a line's
/// height and baseline come from its tallest run. The colour and fill never change the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRunDescriptor {
    text: String,
    family_name: Option<String>,
    size: f32,
    color: Option<Color>,
    fill: Option<ShapeFill>,
    tracking: f32,
}

impl TextRunDescriptor {
    /// A run (C# `TextRunDescriptor(text, size = 12, familyName = null)`): the family None for
    /// the registry's first font. The error says what to do with a size that is not positive and
    /// finite.
    pub fn new(text: &str, size: f32, family_name: Option<&str>) -> Result<TextRunDescriptor, String> {
        if !(size > 0.0) || !size.is_finite() {
            return Err(format!("A text run's size must be positive and finite (it was {size}): pass the em size in calibrated units."));
        }
        Ok(TextRunDescriptor { text: text.to_string(), family_name: family_name.map(str::to_string), size, color: None, fill: None, tracking: 0.0 })
    }

    /// The same run painted in its own colour (C# `Color { init; }`).
    pub fn with_color(mut self, color: Color) -> TextRunDescriptor {
        self.color = Some(color);
        self
    }

    /// The same run painted by its own fill (C# `Fill { init; }`); the fill wins over a colour.
    pub fn with_fill(mut self, fill: ShapeFill) -> TextRunDescriptor {
        self.fill = Some(fill);
        self
    }

    /// The same run with tracking - extra space after each of its clusters, as a fraction of its
    /// size (C# `Tracking { init; }`). The error says what to do with a value that is not finite.
    pub fn with_tracking(mut self, tracking: f32) -> Result<TextRunDescriptor, String> {
        if !tracking.is_finite() {
            return Err("A text run's tracking must be a finite number: pass the extra space as a fraction of the size (0 for none).".to_string());
        }
        self.tracking = tracking;
        Ok(self)
    }

    /// The run's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The family (None: the registry's first font).
    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    /// The em size, calibrated units.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// The run's own colour (None: the block's).
    pub fn color(&self) -> Option<Color> {
        self.color
    }

    /// The run's own fill (None: the block's).
    pub fn fill(&self) -> Option<&ShapeFill> {
        self.fill.as_ref()
    }

    /// The tracking, a fraction of the size.
    pub fn tracking(&self) -> f32 {
        self.tracking
    }
}
