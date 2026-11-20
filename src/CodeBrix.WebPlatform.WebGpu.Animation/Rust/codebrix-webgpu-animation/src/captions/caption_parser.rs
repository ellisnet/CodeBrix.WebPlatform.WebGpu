//! The SRT and WebVTT readers (C# `CaptionParser`, ported line for line): blocks of non-blank
//! lines, timings, markup removal, character references and WebVTT's basic settings.

use crate::captions::CaptionCue;
use crate::layout::{LayoutAnchor, LayoutPosition, LayoutPositionOptions, LayoutReference};
use crate::text::TextAlign;

const ARROW: &str = "-->";

/// A line of the text and its 1-based number.
type NumberedLine = (usize, String);

pub(crate) fn parse_srt(text: &str) -> Result<Vec<CaptionCue>, String> {
    let mut cues = Vec::new();
    for block in blocks(&lines(text)) {
        let timing = if block[0].1.contains(ARROW) { 0 } else { 1 };
        if timing >= block.len() || !block[timing].1.contains(ARROW) {
            return Err(format!("Line {}: expected an SRT timing line (\"00:00:01,000 --> 00:00:04,000\").", block[timing.min(block.len() - 1)].0));
        }
        let (start, end, _) = parse_timing(&block[timing])?;
        let identifier = if timing == 1 { Some(block[0].1.trim().to_string()) } else { None };
        cues.push(CaptionCue::new(start, end, &cue_text(&block, timing + 1, true), identifier.as_deref())?);
    }
    Ok(cues)
}

pub(crate) fn parse_web_vtt(text: &str) -> Result<Vec<CaptionCue>, String> {
    let lines = lines(text);
    if lines.is_empty() || !is_signature(&lines[0].1) {
        return Err("Line 1: a WebVTT file must start with \"WEBVTT\".".to_string());
    }
    let mut cues = Vec::new();
    let mut header = true;
    for block in blocks(&lines) {
        if header {
            // The first block is the header (the signature line and anything up to a blank line).
            header = false;
            if block[0].0 == lines[0].0 {
                continue;
            }
        }
        let first = &block[0].1;
        if starts_word(first, "NOTE") || starts_word(first, "STYLE") || starts_word(first, "REGION") {
            continue;
        }
        let timing = if first.contains(ARROW) { 0 } else { 1 };
        if timing >= block.len() || !block[timing].1.contains(ARROW) {
            return Err(format!("Line {}: expected a WebVTT timing line (\"00:01.000 --> 00:04.000\").", block[timing.min(block.len() - 1)].0));
        }
        let (start, end, settings) = parse_timing(&block[timing])?;
        let identifier = if timing == 1 { Some(first.trim().to_string()) } else { None };
        let (position, align) = apply_settings(&settings);
        let mut cue = CaptionCue::new(start, end, &cue_text(&block, timing + 1, false), identifier.as_deref())?;
        if !settings.trim().is_empty() {
            cue = cue.with_settings(settings.trim());
        }
        if let Some(position) = position {
            cue = cue.with_position(position);
        }
        if let Some(align) = align {
            cue = cue.with_align(align);
        }
        cues.push(cue);
    }
    Ok(cues)
}

fn is_signature(line: &str) -> bool {
    line.strip_prefix("WEBVTT").is_some_and(|rest| rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t'))
}

fn starts_word(line: &str, word: &str) -> bool {
    line.strip_prefix(word).is_some_and(|rest| rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t'))
}

/// The text's lines with 1-based numbers; CR LF and CR count as LF, a leading BOM is dropped.
fn lines(text: &str) -> Vec<NumberedLine> {
    let normal = text.replace("\r\n", "\n").replace('\r', "\n");
    let normal = normal.strip_prefix('\u{FEFF}').unwrap_or(&normal);
    normal.split('\n').enumerate().map(|(i, l)| (i + 1, l.to_string())).collect()
}

/// Runs of non-blank lines.
fn blocks(lines: &[NumberedLine]) -> Vec<Vec<NumberedLine>> {
    let mut all = Vec::new();
    let mut block = Vec::new();
    for line in lines {
        if line.1.trim().is_empty() {
            if !block.is_empty() {
                all.push(std::mem::take(&mut block));
            }
            continue;
        }
        block.push(line.clone());
    }
    if !block.is_empty() {
        all.push(block);
    }
    all
}

/// "start --> end [settings]" -> the times and the settings text.
fn parse_timing(line: &NumberedLine) -> Result<(f64, f64, String), String> {
    let text = &line.1;
    let arrow = text.find(ARROW).unwrap_or(0);
    let left = text[..arrow].trim();
    let right = text[arrow + ARROW.len()..].trim();
    let space = right.find([' ', '\t']);
    let end_text = space.map_or(right, |s| &right[..s]);
    let settings = space.map_or(String::new(), |s| right[s + 1..].to_string());
    let (Some(start), Some(end)) = (try_parse_time(left), try_parse_time(end_text)) else {
        return Err(format!("Line {}: \"{}\" is not a valid timing (expected \"start --> end\").", line.0, text.trim()));
    };
    if end < start {
        return Err(format!("Line {}: the cue ends before it starts.", line.0));
    }
    Ok((start, end, settings))
}

/// [hh:]mm:ss[.,]fff -> seconds, through whole milliseconds so the value is exact (C#
/// `TryParseTime`).
pub(crate) fn try_parse_time(text: &str) -> Option<f64> {
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let mut hours = 0i64;
    let mut index = 0;
    if parts.len() == 3 {
        hours = try_digits(parts[0], 1, 9)?;
        index = 1;
    }
    let minutes = try_digits(parts[index], 1, 2).filter(|m| *m <= 59)?;
    let last = parts[index + 1];
    let separator = last.find(['.', ',']);
    let whole_text = separator.map_or(last, |s| &last[..s]);
    let whole_seconds = try_digits(whole_text, 1, 2).filter(|s| *s <= 59)?;
    let mut millis = 0i64;
    if let Some(s) = separator {
        let fraction_text = &last[s + 1..];
        let fraction = try_digits(fraction_text, 1, 3)?;
        millis = fraction * match fraction_text.len() {
            1 => 100,
            2 => 10,
            _ => 1,
        };
    }
    let total = ((((hours * 60) + minutes) * 60 + whole_seconds) * 1000) + millis;
    Some(total as f64 / 1000.0)
}

/// ASCII digits only, between `min` and `max` of them.
fn try_digits(text: &str, min: usize, max: usize) -> Option<i64> {
    if text.len() < min || text.len() > max || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(text.bytes().fold(0i64, |v, b| (v * 10) + (b - b'0') as i64))
}

fn cue_text(block: &[NumberedLine], from: usize, srt: bool) -> String {
    let mut text = String::new();
    for line in block.iter().skip(from) {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&clean(&line.1, srt));
    }
    text
}

/// Drops markup tags (and SRT {\...} overrides) and decodes character references (C# `Clean`).
pub(crate) fn clean(line: &str, srt: bool) -> String {
    let mut text = String::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        let c = line[i..].chars().next().unwrap_or('\0');
        if c == '<' {
            if let Some(close) = line[i + 1..].find('>') {
                i += 1 + close + 1;
                continue;
            }
        }
        if srt && c == '{' && line[i + 1..].starts_with('\\') {
            if let Some(close) = line[i + 2..].find('}') {
                i += 2 + close + 1;
                continue;
            }
        }
        if c == '&' {
            if let Some(semicolon) = line[i + 1..].find(';').map(|s| i + 1 + s) {
                if semicolon - i <= 10 {
                    if let Some(decoded) = try_decode(&line[i + 1..semicolon]) {
                        text.push_str(&decoded);
                        i = semicolon + 1;
                        continue;
                    }
                }
            }
        }
        text.push(c);
        i += c.len_utf8();
    }
    text.trim().to_string()
}

fn try_decode(name: &str) -> Option<String> {
    let named = match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some("\u{00A0}"),
        "lrm" => Some("\u{200E}"),
        "rlm" => Some("\u{200F}"),
        _ => None,
    };
    if let Some(named) = named {
        return Some(named.to_string());
    }
    let digits = name.strip_prefix('#')?;
    if digits.is_empty() {
        return None;
    }
    let (digits, radix) = match digits.strip_prefix(['x', 'X']) {
        Some(hex) if !hex.is_empty() => (hex, 16),
        _ => (digits, 10),
    };
    if !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = u32::from_str_radix(digits, radix).ok()?;
    if value == 0 {
        return None;
    }
    char::from_u32(value).map(|c| c.to_string())
}

/// WebVTT settings: align (start/left, center/middle, end/right), line:N% (the box's top at N% of
/// the frame height), position:N% (the box's anchor at N% of the width of the caption area).
fn apply_settings(settings: &str) -> (Option<LayoutPosition>, Option<TextAlign>) {
    let mut align = None;
    if settings.trim().is_empty() {
        return (None, None);
    }
    let mut line = None;
    let mut horizontal = None;
    for token in settings.split([' ', '\t']).filter(|t| !t.is_empty()) {
        let Some(colon) = token.find(':').filter(|c| *c > 0) else { continue };
        let key = &token[..colon];
        let mut value = &token[colon + 1..];
        if let Some(comma) = value.find(',') {
            value = &value[..comma];
        }
        match key {
            "align" => {
                align = match value {
                    "start" | "left" => Some(TextAlign::Left),
                    "center" | "middle" => Some(TextAlign::Center),
                    "end" | "right" => Some(TextAlign::Right),
                    _ => align,
                }
            }
            "line" => {
                if let Some(l) = try_percent(value) {
                    line = Some(l);
                }
            }
            "position" => {
                if let Some(p) = try_percent(value) {
                    horizontal = Some(p);
                }
            }
            _ => {}
        }
    }
    let position = if let Some(line) = line {
        let x = horizontal.unwrap_or(0.5);
        LayoutAnchor::new(x, 0.0).ok().and_then(|a| LayoutPosition::new(a, LayoutPositionOptions { reference: LayoutReference::Frame, fraction_y: line, ..LayoutPositionOptions::default() }).ok())
    } else if let Some(x) = horizontal {
        LayoutAnchor::new(x, 1.0).ok().and_then(|a| LayoutPosition::new(a, LayoutPositionOptions { reference: LayoutReference::SafeArea, ..LayoutPositionOptions::default() }).ok())
    } else {
        None
    };
    (position, align)
}

/// "N%" with N in 0..100 (digits and one decimal point, as C#'s AllowDecimalPoint) -> N / 100.
fn try_percent(value: &str) -> Option<f32> {
    let number = value.strip_suffix('%')?;
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit() || c == '.') || number.matches('.').count() > 1 {
        return None;
    }
    let percent: f32 = number.parse().ok()?;
    if !(0.0..=100.0).contains(&percent) {
        return None;
    }
    Some(percent / 100.0)
}
