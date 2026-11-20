//! `FontRegistry`: the fonts text is drawn with, registered from their BYTES (C# `FontRegistry`;
//! no path or stream overloads - the crate never reads files or fetches anything).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::text::font_face::FontFace;
use crate::text::missing_character::MissingCharacter;
use crate::text::text_shaping;

/// The options of [`FontRegistry::measure_text`] (C#'s default arguments: tracking 0, the first
/// registered font, kerning on).
#[derive(Clone, Debug, PartialEq)]
pub struct MeasureTextOptions {
    /// Extra space after every character but the last, as a fraction of the size (default 0).
    pub tracking: f32,
    /// The family; None (the default) for the registry's first font.
    pub family_name: Option<String>,
    /// True (the default) to apply the font's kerning.
    pub kerning: bool,
}

impl Default for MeasureTextOptions {
    fn default() -> MeasureTextOptions {
        MeasureTextOptions { tracking: 0.0, family_name: None, kerning: true }
    }
}

struct Entry {
    name: String,
    face: Rc<FontFace>,
}

/// The fonts a drawing's text uses. SYSTEM FONTS ARE NEVER USED: every font is registered from
/// the bytes of a TrueType or OpenType file, wherever those bytes came from - the application's
/// backend through the platform's generated `<name>_bytes` call, or bytes packed into the
/// frontend (`include_bytes!`, a Dioxus asset). One file is one face.
///
/// A REGISTRY IS A SHARED HANDLE (as a C# reference is): `clone()` gives the same registry, so a
/// font registered through one handle is there for every session, shape and overlay that holds
/// another - register when the bytes arrive, and everything that draws text afterwards finds
/// it. It belongs to one thread (the browser's), unlike C#'s locked registry.
///
/// FAMILY NAMES: a font is known by its own family name (from its name table) or by the
/// override given when it was registered; names match without regard to case; `None` means the
/// FIRST registered font.
#[derive(Clone, Default)]
pub struct FontRegistry {
    entries: Rc<RefCell<Vec<Entry>>>,
}

impl fmt::Debug for FontRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontRegistry").field("families", &self.get_registered_family_names()).finish()
    }
}

impl PartialEq for FontRegistry {
    /// Two handles are equal when they are the same registry.
    fn eq(&self, other: &FontRegistry) -> bool {
        Rc::ptr_eq(&self.entries, &other.entries)
    }
}

impl FontRegistry {
    /// An empty registry.
    pub fn new() -> FontRegistry {
        FontRegistry::default()
    }

    /// The number of registered fonts.
    pub fn count(&self) -> usize {
        self.entries.borrow().len()
    }

    /// Registers a font from the bytes of a font file and returns the family name it is known by
    /// (C# `RegisterFont(byte[] data, string familyNameOverride = null)`): the override when one
    /// is given (trimmed), else the font's own family name. The error (C# ArgumentException)
    /// says what to do.
    pub fn register_font(&self, data: Vec<u8>, family_name_override: Option<&str>) -> Result<String, String> {
        let face = FontFace::from_bytes(data)?;
        let name = match family_name_override.map(str::trim).filter(|s| !s.is_empty()) {
            Some(name) => name.to_string(),
            None => face.family_name.clone(),
        };
        self.entries.borrow_mut().push(Entry { name: name.clone(), face: Rc::new(face) });
        Ok(name)
    }

    /// The family names the fonts are known by, in the order they were registered.
    pub fn get_registered_family_names(&self) -> Vec<String> {
        self.entries.borrow().iter().map(|e| e.name.clone()).collect()
    }

    /// True when a font answers to the family name (None: when any font is registered).
    pub fn contains(&self, family_name: Option<&str>) -> bool {
        self.resolve(family_name).is_ok()
    }

    /// The advance width of a line of text, in the units of `size` (C# `MeasureText`): the same
    /// width a [`TextShape`](crate::shapes::TextShape) of that text has, tracking included.
    pub fn measure_text(&self, text: &str, size: f32, options: &MeasureTextOptions) -> Result<f32, String> {
        check_size_and_tracking(size, options.tracking)?;
        let (_, face) = self.resolve(options.family_name.as_deref())?;
        let line = text_shaping::shape(&face, text, options.kerning);
        let scale = size / face.units_per_em;
        let count = line.clusters.len();
        Ok(if count == 0 { 0.0 } else { line.advance * scale + (count as f32 - 1.0) * options.tracking * size })
    }

    /// The characters of `text` that a family lacks, in text order (no C# counterpart; the
    /// owner's rule): what a [`TextShape`](crate::shapes::TextShape) of that text would report,
    /// without making one - so a string can be checked before it is shown.
    pub fn get_missing_characters(&self, text: &str, family_name: Option<&str>) -> Result<Vec<MissingCharacter>, String> {
        let (_, face) = self.resolve(family_name)?;
        Ok(text_shaping::shape(&face, text, true).missing)
    }

    /// The name and face a family name resolves to, by the documented rule. The errors are C#'s.
    pub(crate) fn resolve(&self, family_name: Option<&str>) -> Result<(String, Rc<FontFace>), String> {
        let entries = self.entries.borrow();
        if entries.is_empty() {
            return Err("No font is registered: register a font file before drawing text (system fonts are never used).".to_string());
        }
        let Some(wanted) = family_name else {
            return Ok((entries[0].name.clone(), entries[0].face.clone()));
        };
        let wanted = wanted.trim();
        let lower = wanted.to_lowercase();
        entries
            .iter()
            .find(|e| e.name.to_lowercase() == lower || e.face.family_name.to_lowercase() == lower)
            .map(|e| (e.name.clone(), e.face.clone()))
            .ok_or_else(|| format!("No font is registered under the family name '{wanted}': register it, or pass one of {:?}.", entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>()))
    }
}

/// C#'s checks of a text size and tracking (ArgumentOutOfRangeException).
pub(crate) fn check_size_and_tracking(size: f32, tracking: f32) -> Result<(), String> {
    if !(size > 0.0) || size.is_infinite() {
        return Err(format!("A text size must be positive and finite (it was {size}): pass the em size in calibrated units."));
    }
    if !tracking.is_finite() {
        return Err("A text's tracking must be a finite number: pass the extra space as a fraction of the size (0 for none).".to_string());
    }
    Ok(())
}
