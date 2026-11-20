//! `SafeAreaGuide`: what a platform covers of a frame - margins, crops, a time rule - or several
//! guides at once (C# `SafeAreaGuide`).

use crate::geometry::RectangleF;
use crate::safe_areas::{SafeAreaCrop, SafeAreaMargins, SafeAreaTimeRule, SafeAreaZone};

/// The optional arguments of [`SafeAreaGuide::new`] (C#'s `source = null, crops = null,
/// timeRule = null`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SafeAreaGuideOptions {
    /// Where the figures come from (C# `Source`; empty by default).
    pub source: String,
    /// The crops the platform shows (C# `Crops`).
    pub crops: Vec<SafeAreaCrop>,
    /// When the guide applies (None: always) (C# `TimeRule`).
    pub time_rule: Option<SafeAreaTimeRule>,
}

/// A safe-area guide (C# `SafeAreaGuide`): its name, margins, crops and time rule - or the
/// parts of a combined guide. The SAFE RECTANGLE is what is left of the frame; the UNSAFE ZONES
/// are the margins and what the crops cut off. The presets are [`SafeAreaGuides`](crate::safe_areas::SafeAreaGuides).
#[derive(Clone, Debug, PartialEq)]
pub struct SafeAreaGuide {
    name: String,
    source: String,
    margins: SafeAreaMargins,
    crops: Vec<SafeAreaCrop>,
    time_rule: Option<SafeAreaTimeRule>,
    parts: Vec<SafeAreaGuide>,
}

fn intersect(a: RectangleF, b: RectangleF) -> RectangleF {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    if right > left && bottom > top {
        RectangleF::new(left, top, right - left, bottom - top)
    } else {
        RectangleF::default()
    }
}

impl SafeAreaGuide {
    /// The zone name of a guide whose margins cover everything (C# `WholeFrameZoneName`).
    pub const WHOLE_FRAME_ZONE_NAME: &'static str = "whole frame";

    /// A guide (C# `SafeAreaGuide(name, margins, source = null, crops = null, timeRule = null)`).
    /// The error says what to do with an empty name.
    pub fn new(name: &str, margins: SafeAreaMargins, options: SafeAreaGuideOptions) -> Result<SafeAreaGuide, String> {
        if name.trim().is_empty() {
            return Err("A guide name is required.".to_string());
        }
        Ok(SafeAreaGuide { name: name.trim().to_string(), source: options.source, margins, crops: options.crops, time_rule: options.time_rule, parts: Vec::new() })
    }

    /// Several guides at once - unsafe wherever any of them is (C# `Combine(name, guides)`); a
    /// combined guide contributes its own parts, so combining never nests. The error says what is
    /// missing.
    pub fn combine(name: &str, guides: &[SafeAreaGuide]) -> Result<SafeAreaGuide, String> {
        if name.trim().is_empty() {
            return Err("A guide name is required.".to_string());
        }
        if guides.is_empty() {
            return Err("At least one guide is required.".to_string());
        }
        let mut parts = Vec::new();
        for g in guides {
            if g.parts.is_empty() {
                parts.push(g.clone());
            } else {
                parts.extend(g.parts.iter().cloned());
            }
        }
        let source = parts.iter().filter(|p| !p.source.is_empty()).map(|p| format!("{}: {}", p.name, p.source)).collect::<Vec<_>>().join("\n");
        Ok(SafeAreaGuide { name: name.trim().to_string(), source, margins: SafeAreaMargins::NONE, crops: Vec::new(), time_rule: None, parts })
    }

    /// The name (C# `Name`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the figures come from (C# `Source`; a combined guide's lists its parts', one a line).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The margins (C# `Margins`).
    pub fn margins(&self) -> SafeAreaMargins {
        self.margins
    }

    /// The crops (C# `Crops`).
    pub fn crops(&self) -> &[SafeAreaCrop] {
        &self.crops
    }

    /// The time rule (C# `TimeRule`).
    pub fn time_rule(&self) -> Option<SafeAreaTimeRule> {
        self.time_rule
    }

    /// A combined guide's parts (C# `Parts`; empty otherwise).
    pub fn parts(&self) -> &[SafeAreaGuide] {
        &self.parts
    }

    /// The unsafe zones of a frame of `frame_size` pixels at a time of a video of a duration (C#
    /// `GetUnsafeZones(frameSize, seconds = null, duration = null)`): the margins, then what each
    /// crop cuts off (slivers below a hundredth of a pixel are left out); none when the time rule
    /// does not apply; the whole frame when the margins cover it.
    pub fn get_unsafe_zones(&self, frame_size: (f32, f32), seconds: Option<f64>, duration: Option<f64>) -> Vec<SafeAreaZone> {
        let mut zones = Vec::new();
        if frame_size.0 <= 0.0 || frame_size.1 <= 0.0 {
            return zones;
        }
        if !self.parts.is_empty() {
            for part in &self.parts {
                zones.extend(part.get_unsafe_zones(frame_size, seconds, duration));
            }
            return zones;
        }
        if let Some(rule) = &self.time_rule {
            if !rule.applies_at(seconds, duration) {
                return zones;
            }
        }
        let (width, height) = frame_size;
        let m = self.margins;
        if m.top() + m.bottom() >= 1.0 || m.left() + m.right() >= 1.0 {
            zones.push(SafeAreaZone { guide_name: self.name.clone(), zone_name: SafeAreaGuide::WHOLE_FRAME_ZONE_NAME.to_string(), bounds: RectangleF::new(0.0, 0.0, width, height) });
            return zones;
        }
        self.add_zone(&mut zones, "top margin", 0.0, 0.0, width, m.top() * height);
        self.add_zone(&mut zones, "bottom margin", 0.0, height - (m.bottom() * height), width, m.bottom() * height);
        self.add_zone(&mut zones, "left margin", 0.0, 0.0, m.left() * width, height);
        self.add_zone(&mut zones, "right margin", width - (m.right() * width), 0.0, m.right() * width, height);
        for crop in &self.crops {
            let kept = crop.get_kept_rectangle(frame_size);
            self.add_zone(&mut zones, crop.name(), 0.0, 0.0, width, kept.y);
            self.add_zone(&mut zones, crop.name(), 0.0, kept.bottom(), width, height - kept.bottom());
            self.add_zone(&mut zones, crop.name(), 0.0, kept.y, kept.x, kept.height);
            self.add_zone(&mut zones, crop.name(), kept.right(), kept.y, width - kept.right(), kept.height);
        }
        zones
    }

    /// The safe rectangle of a frame of `frame_size` pixels (C# `GetSafeRectangle(frameSize,
    /// seconds = null, duration = null)`): the frame less the margins, cut to every crop - a
    /// combined guide's parts intersected; empty when nothing is left.
    pub fn get_safe_rectangle(&self, frame_size: (f32, f32), seconds: Option<f64>, duration: Option<f64>) -> RectangleF {
        if frame_size.0 <= 0.0 || frame_size.1 <= 0.0 {
            return RectangleF::default();
        }
        let mut safe = RectangleF::new(0.0, 0.0, frame_size.0, frame_size.1);
        if !self.parts.is_empty() {
            for part in &self.parts {
                safe = intersect(safe, part.get_safe_rectangle(frame_size, seconds, duration));
            }
            return safe;
        }
        if let Some(rule) = &self.time_rule {
            if !rule.applies_at(seconds, duration) {
                return safe;
            }
        }
        let (width, height) = frame_size;
        let m = self.margins;
        if m.top() + m.bottom() >= 1.0 || m.left() + m.right() >= 1.0 {
            return RectangleF::default();
        }
        let left = m.left() * width;
        let top = m.top() * height;
        safe = RectangleF::new(left, top, (width - (m.right() * width)) - left, (height - (m.bottom() * height)) - top);
        for crop in &self.crops {
            safe = intersect(safe, crop.get_kept_rectangle(frame_size));
        }
        safe
    }

    fn add_zone(&self, zones: &mut Vec<SafeAreaZone>, zone_name: &str, x: f32, y: f32, width: f32, height: f32) {
        // A sliver below a hundredth of a pixel is rounding noise from a crop that removes nothing.
        if width < 0.01 || height < 0.01 {
            return;
        }
        zones.push(SafeAreaZone { guide_name: self.name.clone(), zone_name: zone_name.to_string(), bounds: RectangleF::new(x, y, width, height) });
    }
}
