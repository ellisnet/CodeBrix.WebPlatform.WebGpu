//! `DrawingSvgOptions`: how a document is loaded (the defaults suit a web page).

use std::sync::Arc;

/// How a document is loaded. `Default` = 96 DPI, a 100 x 100 size for a document that states
/// none, no extra style sheet, no fonts (a document's `<text>` is then reported and not drawn).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingSvgOptions {
    /// Dots per inch for physical units (mm, in, pt).
    pub dpi: f32,
    /// The width used when the document states neither a width nor a viewBox.
    pub default_width: f32,
    /// The height used when the document states neither a height nor a viewBox.
    pub default_height: f32,
    /// A style sheet applied to the document before its own (static properties only).
    pub style_sheet: Option<String>,
    /// The fonts the document's `<text>` is laid out with: the BYTES of TrueType / OpenType
    /// files (or collections) the application supplies - from its backend, or packed into the
    /// frontend. The crate never looks for a system font. With none, text is reported
    /// ([`NoFontsRegistered`](crate::svg::DrawingSvgWarningKind::NoFontsRegistered)) and not drawn.
    pub fonts: Vec<Arc<Vec<u8>>>,
    /// The family used for text that names no family, a generic family (serif, sans-serif,
    /// monospace, cursive, fantasy) or a family the supplied fonts do not have (reported as
    /// [`FontSubstituted`](crate::svg::DrawingSvgWarningKind::FontSubstituted)). None: the family of
    /// the first supplied font.
    pub font_family: Option<String>,
}

impl Default for DrawingSvgOptions {
    fn default() -> DrawingSvgOptions {
        DrawingSvgOptions { dpi: 96.0, default_width: 100.0, default_height: 100.0, style_sheet: None, fonts: Vec::new(), font_family: None }
    }
}

impl DrawingSvgOptions {
    /// The defaults with one font for the document's text (its bytes); the font's own family
    /// stands in for every family the document names.
    pub fn with_font(font: Arc<Vec<u8>>) -> DrawingSvgOptions {
        DrawingSvgOptions { fonts: vec![font], ..DrawingSvgOptions::default() }
    }
}
