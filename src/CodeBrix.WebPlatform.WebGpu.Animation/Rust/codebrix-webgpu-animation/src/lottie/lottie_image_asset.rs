//! `LottieImageAsset`: an image a document refers to (its pixels are the application's).

/// An image asset of a document: what an image layer draws. The crate decodes and fetches
/// nothing: the application looks at `location` (a file name relative to `directory`, or a
/// `data:` URL with the encoded bytes), gets the DECODED pixels from its backend and hands them
/// to the drawing half.
#[derive(Clone, Debug, PartialEq)]
pub struct LottieImageAsset {
    /// The asset's `id` (what [`Image`](crate::lottie::LottieDrawCommand::Image) commands name).
    pub id: String,
    /// The authored width and height, when the asset gives them.
    pub size: Option<(f64, f64)>,
    /// The asset's location: `u` + `p`, or the `data:` URL as written.
    pub location: String,
    /// True when the location is a `data:` URL (the encoded bytes are inside the document).
    pub is_data_url: bool,
}
