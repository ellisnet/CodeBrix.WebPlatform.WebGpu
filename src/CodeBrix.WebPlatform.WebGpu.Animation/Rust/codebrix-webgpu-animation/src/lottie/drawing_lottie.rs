//! `DrawingLottie`: a Lottie document read from its JSON, with its size, timing, markers,
//! image assets and the report of what is not drawn.

use kurbo::Affine;
use serde_json::Value;

use crate::lottie::document_check;
use crate::lottie::drawing_lottie_options::DrawingLottieOptions;
use crate::lottie::drawing_lottie_warning::{DrawingLottieWarning, WarningList};
use crate::lottie::group_folding;
use crate::lottie::lottie_blend_mode::LottieBlendMode;
use crate::lottie::lottie_draw_command::LottieDrawList;
use crate::lottie::lottie_image_asset::LottieImageAsset;
use crate::lottie::lottie_marker::LottieMarker;
use crate::lottie::lottie_play_options::LottiePlayOptions;
use crate::lottie::lottie_range::LottieRange;
use crate::lottie::recording_sink::RecordingSink;

/// A Lottie document: read once from its JSON (velato's importer, after the crate's own
/// check of the document), then evaluated for any frame into a [`LottieDrawList`]. Share it
/// between shapes with an `Arc`.
#[derive(Debug)]
pub struct DrawingLottie {
    composition: velato::Composition,
    markers: Vec<LottieMarker>,
    images: Vec<LottieImageAsset>,
    layer_blends: Vec<Option<LottieBlendMode>>,
    warnings: Vec<DrawingLottieWarning>,
}

impl DrawingLottie {
    /// Reads a document from its JSON text. The error says what is wrong.
    pub fn from_json(json: &str, options: &DrawingLottieOptions) -> Result<DrawingLottie, String> {
        let value: Value = serde_json::from_str(json).map_err(|e| format!("The Lottie document is not valid JSON ({e}): pass the animation's JSON text as it was exported."))?;
        DrawingLottie::from_value(value, options)
    }

    /// Reads a document from the bytes of its JSON (UTF-8, an optional byte-order mark).
    /// dotLottie files (`.lottie`, a zip archive) are refused with an error that says what to
    /// do: the backend unpacks the archive and passes the animation JSON inside it.
    pub fn load(bytes: &[u8], options: &DrawingLottieOptions) -> Result<DrawingLottie, String> {
        if bytes.starts_with(b"PK\x03\x04") {
            return Err("These bytes are a zip archive - a dotLottie (.lottie) file - which this crate does not read: unpack it on the backend and pass the animation JSON inside it (the file under animations/ named in its manifest.json), with any images it uses as decoded pixels.".to_string());
        }
        let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
        let text = std::str::from_utf8(bytes).map_err(|e| format!("The Lottie document is not UTF-8 text ({e}): pass the animation's JSON as it was exported."))?;
        DrawingLottie::from_json(text, options)
    }

    fn from_value(mut value: Value, options: &DrawingLottieOptions) -> Result<DrawingLottie, String> {
        let mut warnings = WarningList::default();
        let checked = document_check::check(&mut value, options, &mut warnings)?;
        let composition = velato::Composition::from_json(value).map_err(|e| format!("The Lottie document could not be read ({e}): a property it needs is missing or has the wrong type."))?;
        let mut layer_blends = checked.layer_blends;
        layer_blends.resize(composition.layers.len(), None);
        Ok(DrawingLottie { composition, markers: checked.markers, images: checked.images, layer_blends, warnings: warnings.items })
    }

    /// The width in document units (Lottie `w`).
    pub fn width(&self) -> f64 {
        self.composition.width as f64
    }

    /// The height in document units (Lottie `h`).
    pub fn height(&self) -> f64 {
        self.composition.height as f64
    }

    /// Frames per second (Lottie `fr`).
    pub fn frame_rate(&self) -> f64 {
        self.composition.frame_rate
    }

    /// The first frame (Lottie `ip`).
    pub fn in_point(&self) -> f64 {
        self.composition.frames.start
    }

    /// The frame playback stops before (Lottie `op`, exclusive).
    pub fn out_point(&self) -> f64 {
        self.composition.frames.end
    }

    /// The number of frames, `op - ip`.
    pub fn frame_count(&self) -> f64 {
        self.out_point() - self.in_point()
    }

    /// The length in seconds, `(op - ip) / fr`.
    pub fn duration(&self) -> f64 {
        self.frame_count() / self.frame_rate()
    }

    /// The markers, in document order.
    pub fn markers(&self) -> &[LottieMarker] {
        &self.markers
    }

    /// The marker of that name, if there is one.
    pub fn marker(&self, name: &str) -> Option<&LottieMarker> {
        self.markers.iter().find(|m| m.name == name)
    }

    /// The image assets (what [`Image`](crate::lottie::LottieDrawCommand::Image) commands name).
    pub fn images(&self) -> &[LottieImageAsset] {
        &self.images
    }

    /// What is not drawn, or not drawn as written, deduplicated.
    pub fn warnings(&self) -> &[DrawingLottieWarning] {
        &self.warnings
    }

    /// The report of what this document uses that is not drawn or drawn in an approximated
    /// way: the same list as [`warnings`](DrawingLottie::warnings), one line each,
    /// "Kind: message". Empty when everything is drawn as written.
    pub fn report(&self) -> Vec<String> {
        self.warnings.iter().map(|w| format!("{:?}: {}", w.kind, w.message)).collect()
    }

    /// The frames a range plays, `(start, end)` with `end` exclusive, clamped to the document's
    /// in and out points. An unknown marker name is an error.
    pub fn resolve_range(&self, range: &LottieRange) -> Result<(f64, f64), String> {
        let (ip, op) = (self.in_point(), self.out_point());
        let (start, end) = match range {
            LottieRange::Whole => (ip, op),
            LottieRange::Frames { start, end } => (*start, *end),
            LottieRange::Marker(name) => {
                let Some(m) = self.marker(name) else {
                    let names: Vec<&str> = self.markers.iter().map(|m| m.name.as_str()).collect();
                    return Err(format!("The Lottie document has no marker named '{name}' (its markers: {}).", if names.is_empty() { "none".to_string() } else { names.join(", ") }));
                };
                (m.frame, if m.duration > 0.0 { m.end_frame() } else { op })
            }
        };
        let (start, end) = (start.clamp(ip, op), end.clamp(ip, op));
        if end.is_nan() || start.is_nan() || end <= start {
            return Err(format!("The range {range:?} holds no frames of this document (frames {ip} to {op}): give an end after its start."));
        }
        Ok((start, end))
    }

    /// The Lottie frame shown at session `time` under these play options.
    pub fn frame_at(&self, options: &LottiePlayOptions, time: f64) -> Result<f64, String> {
        let (start, end) = self.resolve_range(&options.range)?;
        Ok(options.frame_in(start, end, self.frame_rate(), time))
    }

    /// The document evaluated at Lottie `frame` (any number: velato interpolates between
    /// keyframes) under `transform` (document units to output units) at opacity `alpha`; with
    /// `fold_groups`, one-draw groups are folded into their draw.
    pub fn draw_list(&self, frame: f64, transform: Affine, alpha: f64, fold_groups: bool) -> LottieDrawList {
        let mut renderer = velato::Renderer::new();
        self.draw_list_with(&mut renderer, frame, transform, alpha, fold_groups)
    }

    /// As [`draw_list`](DrawingLottie::draw_list), reusing a velato renderer's buffers.
    pub(crate) fn draw_list_with(&self, renderer: &mut velato::Renderer, frame: f64, transform: Affine, alpha: f64, fold_groups: bool) -> LottieDrawList {
        let mut sink = RecordingSink::new(&self.layer_blends, self.width(), self.height(), transform);
        renderer.append(&self.composition, frame, transform, alpha.clamp(0.0, 1.0), &mut sink);
        let unsupported = sink.unsupported;
        let (commands, folded_groups) = if fold_groups { group_folding::fold(sink.commands) } else { (sink.commands, 0) };
        LottieDrawList { commands, frame, folded_groups, unsupported }
    }
}
