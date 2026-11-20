//! The SVG documents of the SVG scenes ("svg-parts", "svg-smil", "svg-css", "svg-features") and the
//! fonts their text is laid out with - RESOURCES OF THE BACKEND (this sample's rule), fetched once
//! with the generated `..._svg_bytes` calls and kept here; the fonts are the bytes the page already
//! fetched for its FontRegistry (Roboto, Open Sans). The crate takes SVG text and font bytes from
//! anywhere; it never reads a file or fetches anything itself.

use std::cell::RefCell;
use std::sync::Arc;

use codebrix_webgpu_animation::svg::{DrawingSvg, DrawingSvgOptions};

use crate::generated::api::svg_documents;

thread_local! {
    static FONTS: RefCell<Vec<Arc<Vec<u8>>>> = const { RefCell::new(Vec::new()) };
    static DOCUMENTS: RefCell<Vec<(&'static str, String)>> = const { RefCell::new(Vec::new()) };
}

/// Keeps a font's bytes for the SVG documents' text (Roboto first: it is the family used for
/// text whose family the fonts do not have).
pub fn set_svg_font(bytes: Vec<u8>) {
    FONTS.with(|f| f.borrow_mut().push(Arc::new(bytes)));
}

fn keep(name: &'static str, bytes: Vec<u8>) -> Result<(), String> {
    let text = String::from_utf8(bytes).map_err(|_| format!("The SVG document '{name}' from the backend is not UTF-8 text."))?;
    DOCUMENTS.with(|d| d.borrow_mut().push((name, text)));
    Ok(())
}

/// Fetches the SVG documents from the backend once (the generated `_bytes` calls).
pub async fn fetch_svg_documents() -> Result<(), String> {
    let failed = |name: &str, message: String| format!("The SVG document '{name}' could not be read from the backend: {message}");
    keep("gear-train", svg_documents::gear_train_svg_bytes().await.map_err(|e| failed("gear-train", e.message().to_string()))?)?;
    keep("clock", svg_documents::clock_svg_bytes().await.map_err(|e| failed("clock", e.message().to_string()))?)?;
    keep("smil-scene", svg_documents::smil_scene_svg_bytes().await.map_err(|e| failed("smil-scene", e.message().to_string()))?)?;
    keep("css-scene", svg_documents::css_scene_svg_bytes().await.map_err(|e| failed("css-scene", e.message().to_string()))?)?;
    keep("features", svg_documents::features_svg_bytes().await.map_err(|e| failed("features", e.message().to_string()))?)?;
    Ok(())
}

/// A fetched document loaded with the fonts (an empty document when it was not fetched - the
/// scene then shows the load error in its report).
pub fn load_svg(name: &str) -> Result<Arc<DrawingSvg>, String> {
    let text = DOCUMENTS.with(|d| d.borrow().iter().find(|(n, _)| *n == name).map(|(_, t)| t.clone())).unwrap_or_default();
    let fonts = FONTS.with(|f| f.borrow().clone());
    DrawingSvg::from_svg(&text, &DrawingSvgOptions { fonts, ..DrawingSvgOptions::default() }).map(Arc::new)
}

/// The notes and the document's report beside a document drawn at the left of the frame: the
/// lines of `notes` (what to look at), then every line of `svg.report()` (what is not drawn or
/// not played), wrapped by hand at about `chars` characters.
pub fn notes_and_report(layer: &mut codebrix_webgpu_animation::models::DrawingLayer, fonts: &codebrix_webgpu_animation::text::FontRegistry, svg: &DrawingSvg, notes: &[&str], x: f32, mut y: f32, chars: usize) {
    use codebrix_webgpu_animation::text::TextAlign;
    use crate::demo::{label, ACCENT, DIM, INK};
    for n in notes {
        label(layer, fonts, n, x, y, 14.0, INK, TextAlign::Left);
        y += 22.0;
    }
    y += 12.0;
    let report = svg.report();
    label(layer, fonts, if report.is_empty() { "report(): empty - everything is drawn and played" } else { "report():" }, x, y, 14.0, ACCENT, TextAlign::Left);
    y += 22.0;
    for line in report {
        let mut rest = line.as_str();
        while !rest.is_empty() {
            let cut = if rest.chars().count() <= chars { rest.len() } else { rest.char_indices().take(chars).filter(|(_, c)| *c == ' ').map(|(i, _)| i).last().unwrap_or(rest.len()) };
            label(layer, fonts, rest[..cut].trim(), x, y, 12.0, DIM, TextAlign::Left);
            y += 17.0;
            rest = rest[cut..].trim_start();
        }
        y += 4.0;
    }
}
