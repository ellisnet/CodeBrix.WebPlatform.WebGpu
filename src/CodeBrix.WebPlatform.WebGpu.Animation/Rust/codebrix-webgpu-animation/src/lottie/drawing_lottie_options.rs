//! `DrawingLottieOptions`: how a document is read.

/// How a document is read ([`DrawingLottie::from_json`](crate::lottie::DrawingLottie::from_json)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawingLottieOptions {
    /// The most copies one repeater may make (default 1000). velato expands repeaters without
    /// a limit; a document asking for more is limited to this and reported
    /// ([`RepeaterCopiesLimited`](crate::lottie::DrawingLottieWarningKind::RepeaterCopiesLimited)).
    pub max_repeater_copies: u32,
    /// Draws solid-colour layers (`ty` 1) as a filled rectangle of their size and colour
    /// (default true). velato reads them but draws nothing for them; the crate turns each into a
    /// shape layer holding that rectangle before velato reads the document.
    pub draw_solid_layers: bool,
}

impl Default for DrawingLottieOptions {
    fn default() -> DrawingLottieOptions {
        DrawingLottieOptions { max_repeater_copies: 1000, draw_solid_layers: true }
    }
}
