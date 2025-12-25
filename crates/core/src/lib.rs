#![deny(missing_docs)]
//! Streaming Markdown core utilities: parser, MarkdownStream, and HTML rewriter glue.

/// Markdown event to `io::Write` bridge utilities.
pub mod adapter;
/// Core event types that decouple Markflow from pulldown-cmark specifics.
#[allow(missing_docs)]
pub mod event;
/// YAML frontmatter extraction helpers.
pub mod frontmatter;
/// Parsing layer (markdown-rs adapter + config).
pub mod parser;
/// Rendering layer (HTML/JSX writers + streaming rewriter).
pub mod renderer;
/// Streaming transformers (hoist, directives, code fences, etc.).
pub mod transform;
/// Slug generation utilities.
pub mod slug;
/// Core parsing and rewriting engine.
pub mod engine;
/// Error types and utilities.
pub mod error;

pub use adapter::MarkdownStream;
pub use engine::{
    get_event_iterator, get_event_iterator_with_config, parse, parse_with_options, ImportKind,
    ImportSpec, ParseResult,
};
pub use error::MarkflowError;
pub use frontmatter::{FrontmatterError, FrontmatterExtraction, extract_frontmatter};
pub use parser::{ParseConfig, ParseConstructs};
pub use renderer::{RewriteOptions, StreamingRewriter, render_to_jsx};
pub use slug::{Slugger, slugify};
pub use transform::{
    code_fence,
    directive_adapter::DirectiveAdapter,
    directives::{AsideDirectiveMapper, DirectiveMapper, DirectiveTransform},
    hoist_adapter::HoistAdapter,
};

use crate::parser::markdown_adapter;



/// Iterator alias so callers don't need to depend on the adapter module path.
pub type MarkdownEventStream = markdown_adapter::MarkdownParser;


