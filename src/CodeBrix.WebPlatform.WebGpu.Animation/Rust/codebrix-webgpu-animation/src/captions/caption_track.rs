//! `CaptionTrack`: timed captions in start order, read from SRT or WebVTT text (C#
//! `CaptionTrack`).

use crate::captions::caption_parser;
use crate::captions::{CaptionCue, CaptionFormat};

/// Captions in start order (cues starting together keep their given order) (C# `CaptionTrack`).
/// Read from the TEXT of an SRT or WebVTT file - this crate reads no files; C#'s `Load(path)` has
/// no counterpart: fetch or embed the text and pass it to [`parse`](CaptionTrack::parse).
#[derive(Clone, Debug, PartialEq)]
pub struct CaptionTrack {
    cues: Vec<CaptionCue>,
    end_time: f64,
}

impl CaptionTrack {
    /// A track of cues, sorted by start (C# `CaptionTrack(IEnumerable<CaptionCue>)`).
    pub fn new(cues: Vec<CaptionCue>) -> CaptionTrack {
        let mut cues = cues;
        cues.sort_by(|a, b| a.start_time().total_cmp(&b.start_time()));
        let end_time = cues.iter().fold(0.0f64, |end, c| end.max(c.end_time()));
        CaptionTrack { cues, end_time }
    }

    /// The cues, in start order.
    pub fn cues(&self) -> &[CaptionCue] {
        &self.cues
    }

    /// The number of cues.
    pub fn count(&self) -> usize {
        self.cues.len()
    }

    /// The latest end of any cue (0 for no cues).
    pub fn end_time(&self) -> f64 {
        self.end_time
    }

    /// The cues showing at a time, in start order (C# `GetActiveCues`).
    pub fn get_active_cues(&self, seconds: f64) -> Vec<&CaptionCue> {
        self.get_active_cue_indices(seconds).into_iter().map(|i| &self.cues[i]).collect()
    }

    /// The indices of the cues showing at a time, in start order (C# `GetActiveCueIndices`).
    pub fn get_active_cue_indices(&self, seconds: f64) -> Vec<usize> {
        let mut active = Vec::new();
        for (i, cue) in self.cues.iter().enumerate() {
            if cue.start_time() > seconds {
                break;
            }
            if cue.is_active_at(seconds) {
                active.push(i);
            }
        }
        active
    }

    /// WebVTT when the text starts with "WEBVTT" (after a byte order mark), else SRT (C#
    /// `DetectFormat`).
    pub fn detect_format(text: &str) -> CaptionFormat {
        if text.trim_start_matches('\u{FEFF}').starts_with("WEBVTT") {
            CaptionFormat::WebVtt
        } else {
            CaptionFormat::Srt
        }
    }

    /// Reads SRT or WebVTT text, telling them apart by [`detect_format`](CaptionTrack::detect_format)
    /// (C# `Parse(text)`). The error (C# FormatException) names the line and what was expected.
    pub fn parse(text: &str) -> Result<CaptionTrack, String> {
        CaptionTrack::parse_as(text, CaptionTrack::detect_format(text))
    }

    /// Reads text in a given format (C# `Parse(text, format)`).
    pub fn parse_as(text: &str, format: CaptionFormat) -> Result<CaptionTrack, String> {
        match format {
            CaptionFormat::Srt => CaptionTrack::parse_srt(text),
            CaptionFormat::WebVtt => CaptionTrack::parse_web_vtt(text),
        }
    }

    /// Reads SubRip text (C# `ParseSrt`): blocks of an optional number line, a timing line
    /// "00:00:01,000 --> 00:00:04,000" and text lines; tags and {\...} overrides are removed and
    /// character references decoded.
    pub fn parse_srt(text: &str) -> Result<CaptionTrack, String> {
        Ok(CaptionTrack::new(caption_parser::parse_srt(text)?))
    }

    /// Reads WebVTT text, basic syntax (C# `ParseWebVtt`): the WEBVTT signature, NOTE, STYLE and
    /// REGION blocks skipped, cue ids, timings, the settings align:, line:N% and position:N%
    /// applied, tags removed and character references decoded.
    pub fn parse_web_vtt(text: &str) -> Result<CaptionTrack, String> {
        Ok(CaptionTrack::new(caption_parser::parse_web_vtt(text)?))
    }
}
