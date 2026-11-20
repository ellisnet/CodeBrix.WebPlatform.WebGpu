//! [`GraphicsError`]: one error WebGPU reported outside an error scope (C#: `GraphicsError`), and
//! its [`ErrorType`].

use std::fmt;

use codebrix_webgpu::{GpuError, GpuErrorKind};

/// The kind of a [`GraphicsError`] (C#: the native `ErrorType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorType {
    /// A validation error: the application asked for something WebGPU does not allow (a bug in
    /// the calling code or its data).
    Validation,
    /// The device ran out of memory.
    OutOfMemory,
    /// An internal error of the browser's WebGPU implementation (often a system limit).
    Internal,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrorType::Validation => "Validation",
            ErrorType::OutOfMemory => "OutOfMemory",
            ErrorType::Internal => "Internal",
        })
    }
}

/// One error WebGPU reported for work that was not inside an error scope: what
/// [`GraphicsContext::take_errors`](crate::GraphicsContext::take_errors) hands out (C#:
/// `GraphicsError`, from `TakeErrors()` and the `UncapturedError` event).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsError {
    error_type: ErrorType,
    message: String,
}

impl GraphicsError {
    /// An error of `error_type` with WebGPU's message.
    pub fn new(error_type: ErrorType, message: impl Into<String>) -> GraphicsError {
        GraphicsError { error_type, message: message.into() }
    }

    /// The kind of error (C#: `Type`).
    pub fn error_type(&self) -> ErrorType {
        self.error_type
    }

    /// WebGPU's message (C#: `Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The device's error, as `codebrix-webgpu` collected it, as a graphics error.
    pub(crate) fn from_gpu_error(error: &GpuError) -> GraphicsError {
        let error_type = match error.kind() {
            GpuErrorKind::Validation => ErrorType::Validation,
            GpuErrorKind::OutOfMemory => ErrorType::OutOfMemory,
            GpuErrorKind::Internal => ErrorType::Internal,
        };
        GraphicsError::new(error_type, error.message())
    }
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for GraphicsError {}

#[cfg(test)]
mod tests {
    use codebrix_webgpu::{GpuError, GpuErrorKind};

    use super::{ErrorType, GraphicsError};

    #[test]
    fn a_device_error_keeps_its_kind_and_message() {
        let error = GraphicsError::from_gpu_error(&GpuError::new(GpuErrorKind::OutOfMemory, "no room"));
        assert_eq!(error.error_type(), ErrorType::OutOfMemory);
        assert_eq!(error.message(), "no room");
    }

    #[test]
    fn display_is_the_type_then_the_message() {
        let error = GraphicsError::new(ErrorType::Validation, "the buffer is too small");
        assert_eq!(error.to_string(), "Validation: the buffer is too small");
        assert_eq!(error.error_type(), ErrorType::Validation);
        assert_eq!(error.message(), "the buffer is too small");
    }
}
