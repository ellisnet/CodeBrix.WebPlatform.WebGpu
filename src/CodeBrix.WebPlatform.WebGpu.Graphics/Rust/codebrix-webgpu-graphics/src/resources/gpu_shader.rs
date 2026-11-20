//! [`GpuShader`]: a compiled WGSL shader module and its source, and [`ShaderDiagnostics`], the
//! browser compiler's messages about it (C#: `GpuShader`, the `diagnostics` of `TryCreateShader`).

use std::fmt;

/// A WGSL shader module (C#: `GpuShader`). A browser takes WGSL only (C#'s SPIR-V and GLSL inputs
/// have no counterpart).
pub struct GpuShader {
    handle: wgpu::ShaderModule,
    source: String,
    label: Option<String>,
}

impl GpuShader {
    pub(crate) fn new(handle: wgpu::ShaderModule, source: &str, label: Option<&str>) -> GpuShader {
        GpuShader { handle, source: source.to_string(), label: label.map(str::to_string) }
    }

    /// The `wgpu` shader module (C#: `Handle`).
    pub fn handle(&self) -> &wgpu::ShaderModule {
        &self.handle
    }

    /// The WGSL source (C#: `Source`).
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The label (C#: `Label`).
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The browser compiler's messages about this module - errors, warnings and notes - with
    /// their places in the source (async: the browser answers with a promise).
    pub async fn diagnostics(&self) -> ShaderDiagnostics {
        ShaderDiagnostics::from_wgpu(&self.handle.get_compilation_info().await)
    }
}

/// How serious a [`ShaderMessage`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderMessageKind {
    /// The shader cannot be used.
    Error,
    /// The shader works, but the compiler points at something.
    Warning,
    /// A note.
    Info,
}

/// One message of the browser's shader compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderMessage {
    /// How serious it is.
    pub kind: ShaderMessageKind,
    /// The compiler's text.
    pub text: String,
    /// The 1-based line and column (column in bytes) it points at, if any.
    pub position: Option<(u32, u32)>,
}

impl fmt::Display for ShaderMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            ShaderMessageKind::Error => "error",
            ShaderMessageKind::Warning => "warning",
            ShaderMessageKind::Info => "info",
        };
        match self.position {
            Some((line, column)) => write!(f, "{kind} at {line}:{column}: {}", self.text),
            None => write!(f, "{kind}: {}", self.text),
        }
    }
}

/// Everything the browser's shader compiler said about one module (C#: the `diagnostics` text of
/// `TryCreateShader`). Its `Display` is one message per line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaderDiagnostics {
    /// The messages, in the compiler's order.
    pub messages: Vec<ShaderMessage>,
}

impl ShaderDiagnostics {
    /// The messages of a `wgpu` compilation info.
    pub fn from_wgpu(info: &wgpu::CompilationInfo) -> ShaderDiagnostics {
        let messages = info
            .messages
            .iter()
            .map(|message| ShaderMessage {
                kind: match message.message_type {
                    wgpu::CompilationMessageType::Error => ShaderMessageKind::Error,
                    wgpu::CompilationMessageType::Warning => ShaderMessageKind::Warning,
                    wgpu::CompilationMessageType::Info => ShaderMessageKind::Info,
                },
                text: message.message.clone(),
                position: message.location.map(|location| (location.line_number, location.line_position)),
            })
            .collect();
        ShaderDiagnostics { messages }
    }

    /// True when at least one message is an error.
    pub fn has_errors(&self) -> bool {
        self.count(ShaderMessageKind::Error) > 0
    }

    /// The number of messages of a kind.
    pub fn count(&self, kind: ShaderMessageKind) -> usize {
        self.messages.iter().filter(|message| message.kind == kind).count()
    }

    /// True when the compiler said nothing at all.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl fmt::Display for ShaderDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, message) in self.messages.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{message}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ShaderDiagnostics, ShaderMessage, ShaderMessageKind};

    #[test]
    fn diagnostics_count_and_print_one_message_per_line() {
        let diagnostics = ShaderDiagnostics {
            messages: vec![
                ShaderMessage { kind: ShaderMessageKind::Warning, text: "unused".to_string(), position: Some((3, 7)) },
                ShaderMessage { kind: ShaderMessageKind::Error, text: "unknown type".to_string(), position: None },
            ],
        };
        assert!(diagnostics.has_errors());
        assert_eq!(diagnostics.count(ShaderMessageKind::Warning), 1);
        assert_eq!(diagnostics.to_string(), "warning at 3:7: unused\nerror: unknown type");
        assert!(ShaderDiagnostics::default().is_empty());
    }
}
