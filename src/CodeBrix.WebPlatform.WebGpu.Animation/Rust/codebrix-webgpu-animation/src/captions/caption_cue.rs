//! `CaptionCue`: one timed caption (C# `CaptionCue`).

use std::fmt;

use crate::captions::CaptionStyle;
use crate::layout::LayoutPosition;
use crate::text::TextAlign;

/// One caption: its text and the half-open span [start, end) it shows in, seconds (C#
/// `CaptionCue`), with an optional place, alignment and style of its own (WebVTT's settings give
/// the first two).
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionCue {
    start_time: f64,
    end_time: f64,
    text: String,
    identifier: Option<String>,
    position: Option<LayoutPosition>,
    align: Option<TextAlign>,
    style: Option<CaptionStyle>,
    settings: Option<String>,
}

impl CaptionCue {
    /// A cue (C# `CaptionCue(startTime, endTime, text, identifier = null)`). The error says what
    /// to do with a time that is not finite or an end before the start.
    pub fn new(start_time: f64, end_time: f64, text: &str, identifier: Option<&str>) -> Result<CaptionCue, String> {
        if !start_time.is_finite() {
            return Err(format!("A caption cue's start time must be a finite number of seconds (it was {start_time})."));
        }
        if !end_time.is_finite() || end_time < start_time {
            return Err(format!("A caption cue's end time must be finite and not before its start ({start_time} s; it was {end_time})."));
        }
        Ok(CaptionCue { start_time, end_time, text: text.to_string(), identifier: identifier.map(str::to_string), position: None, align: None, style: None, settings: None })
    }

    /// The same cue placed by a position of its own (C# `Position { init; }`).
    pub fn with_position(mut self, position: LayoutPosition) -> CaptionCue {
        self.position = Some(position);
        self
    }

    /// The same cue with an alignment of its own (C# `Align { init; }`).
    pub fn with_align(mut self, align: TextAlign) -> CaptionCue {
        self.align = Some(align);
        self
    }

    /// The same cue with a style of its own (C# `Style { init; }`).
    pub fn with_style(mut self, style: CaptionStyle) -> CaptionCue {
        self.style = Some(style);
        self
    }

    /// The same cue with its WebVTT settings text (C# `Settings { init; }`; informational).
    pub fn with_settings(mut self, settings: &str) -> CaptionCue {
        self.settings = Some(settings.to_string());
        self
    }

    /// When it starts showing, seconds.
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    /// When it stops showing, seconds (not shown at this time).
    pub fn end_time(&self) -> f64 {
        self.end_time
    }

    /// How long it shows, seconds.
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// The text, tags removed, lines joined with LF.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The identifier line (an SRT number, a WebVTT cue id), if there was one.
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    /// Its own place (None: the style's).
    pub fn position(&self) -> Option<LayoutPosition> {
        self.position
    }

    /// Its own alignment (None: the style's).
    pub fn align(&self) -> Option<TextAlign> {
        self.align
    }

    /// Its own style (None: the presenter's).
    pub fn style(&self) -> Option<&CaptionStyle> {
        self.style.as_ref()
    }

    /// The WebVTT settings text after the timing, as written (None when there was none).
    pub fn settings(&self) -> Option<&str> {
        self.settings.as_deref()
    }

    /// True when it shows at a time: start <= seconds < end (C# `IsActiveAt`).
    pub fn is_active_at(&self, seconds: f64) -> bool {
        seconds >= self.start_time && seconds < self.end_time
    }
}

impl fmt::Display for CaptionCue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} --> {}: {}", trim_number(self.start_time), trim_number(self.end_time), self.text)
    }
}

/// C#'s "0.###" format.
fn trim_number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".to_string() } else { text.to_string() }
}
