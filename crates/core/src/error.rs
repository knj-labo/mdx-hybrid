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
