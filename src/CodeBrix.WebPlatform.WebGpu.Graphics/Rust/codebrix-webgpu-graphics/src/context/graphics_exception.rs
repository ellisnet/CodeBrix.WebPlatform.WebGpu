//! [`GraphicsException`] and [`ShaderCompilationException`]: the errors the library returns
//! (C#: the exceptions of the same names, which the C# library throws).

use std::fmt;

/// An error the library returns instead of the object asked for (C#: `GraphicsException`, which
/// the C# library throws). Its message says what went wrong and what to do.
///
/// It comes from two places: checks the library makes itself before it asks WebGPU (a size of 0,
/// a mip level that does not exist, a builder with no colour target), returned at once; and
/// WebGPU's own validation message, which a browser can only give asynchronously - so it is
/// returned by the `_checked(...).await` variants and by
/// [`GraphicsContext::validate`](crate::GraphicsContext::validate). A plain call leaves a WebGPU
/// validation error to [`GraphicsContext::take_errors`](crate::GraphicsContext::take_errors).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsException {
    message: String,
}

impl GraphicsException {
    /// An error with this message.
    pub fn new(message: impl Into<String>) -> GraphicsException {
        GraphicsException { message: message.into() }
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GraphicsException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GraphicsException {}

impl From<ShaderCompilationException> for GraphicsException {
    fn from(error: ShaderCompilationException) -> GraphicsException {
        GraphicsException::new(error.to_string())
    }
}

/// A WGSL shader the browser's compiler refused (C#: `ShaderCompilationException`): the
/// shader's label and the compiler's messages, one per line, with line and column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderCompilationException {
    label: Option<String>,
    diagnostics: String,
}

impl ShaderCompilationException {
    /// The error for the shader `label` with the compiler's `diagnostics`.
    pub fn new(label: Option<&str>, diagnostics: impl Into<String>) -> ShaderCompilationException {
        ShaderCompilationException { label: label.map(str::to_string), diagnostics: diagnostics.into() }
    }

    /// The shader's label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The compiler's messages (C#: `Diagnostics`).
    pub fn diagnostics(&self) -> &str {
        &self.diagnostics
    }
}

impl fmt::Display for ShaderCompilationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.label {
            Some(label) => write!(f, "The shader '{label}' did not compile; correct the WGSL at the places below and create it again.\n{}", self.diagnostics),
            None => write!(f, "The shader did not compile; correct the WGSL at the places below and create it again.\n{}", self.diagnostics),
        }
    }
}

impl std::error::Error for ShaderCompilationException {}

#[cfg(test)]
mod tests {
    use super::{GraphicsException, ShaderCompilationException};

    #[test]
    fn a_shader_error_names_the_shader_and_says_what_to_do() {
        let error = ShaderCompilationException::new(Some("sprite"), "error at 3:7: unknown type");
        let text = error.to_string();
        assert!(text.starts_with("The shader 'sprite' did not compile; correct the WGSL"));
        assert!(text.ends_with("error at 3:7: unknown type"));
        assert_eq!(GraphicsException::from(error).message(), text);
    }
}
