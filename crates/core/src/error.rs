use thiserror::Error;

/// Errors that can occur during Markdown processing.
#[derive(Debug, Error)]
pub enum MarkflowError {
    /// IO error during streaming.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// UTF-8 encoding error.
    #[error("Encoding error: {0}")]
    EncodingError(#[from] std::string::FromUtf8Error),
    /// markdown-rs parser error surfaced through the adapter.
    #[error("markdown-rs error: {0}")]
    MarkdownAdapter(String),
    /// Rendering error while emitting HTML/JSX.
    #[error("Render error: {0}")]
    RenderError(String),
    /// Unknown component or directive encountered.
    #[error("Unknown component: {0}")]
    UnknownComponent(String),
    /// Internal logic error (unexpected state).
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Non-fatal warnings that don't prevent rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseWarning {
    /// Code fence opened but never closed
    UnclosedCodeFence {
        /// Line number where the fence started
        line: usize,
        /// Fence marker character (backtick or tilde)
        marker: char,
        /// Surrounding context for error message
        context: String,
    },
    /// Other potential warnings for future use
    SuspiciousMarkup {
        /// Line number where the suspicious markup was found
        line: usize,
        /// Warning message
        message: String,
    },
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseWarning::UnclosedCodeFence {
                line,
                marker,
                context,
            } => {
                write!(
                    f,
                    "Unclosed code fence ({}): line {}, near '{}'",
                    marker, line, context
                )
            }
            ParseWarning::SuspiciousMarkup { line, message } => {
                write!(f, "Line {}: {}", line, message)
            }
        }
    }
}

/// Collection of parse diagnostics (warnings, not errors)
#[derive(Debug, Clone, Default)]
pub struct ParseDiagnostics {
    /// List of non-fatal warnings
    pub warnings: Vec<ParseWarning>,
}

impl ParseDiagnostics {
    /// Create a new empty diagnostics collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a warning to the diagnostics collection
    pub fn add_warning(&mut self, warning: ParseWarning) {
        self.warnings.push(warning);
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}
