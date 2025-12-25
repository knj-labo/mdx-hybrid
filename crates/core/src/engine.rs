//! Core parsing and rewriting engine.

use crate::MarkflowError;
use crate::adapter::MarkdownStream;
use crate::event::Event;
use crate::parser::{ParseConfig, markdown_adapter};
use crate::renderer::{RewriteOptions, StreamingRewriter};
use crate::transform::smartypants::apply_smartypants;
use crate::transform::{DirectiveAdapter, HoistAdapter};
use std::cell::RefCell;
use std::rc::Rc;

/// Returns an iterator over Markdown events backed by `markdown-rs`.
pub fn get_event_iterator(input: &str) -> Result<markdown_adapter::MarkdownParser, MarkflowError> {
    get_event_iterator_with_config(input, ParseConfig::mdx())
}

/// Returns an iterator using the specified parse configuration.
pub fn get_event_iterator_with_config(
    input: &str,
    config: ParseConfig,
) -> Result<markdown_adapter::MarkdownParser, MarkflowError> {
    markdown_adapter::MarkdownParser::new_with_config(input, config)
        .map_err(|err| MarkflowError::MarkdownAdapter(err.to_string()))
}

/// Structured result of parsing Markdown.
#[derive(Debug)]
pub struct ParseResult {
    /// Rendered HTML output.
    pub html: String,
    /// Hoisted top-level ESM import/export statements.
    pub imports: Vec<ImportSpec>,
}

/// Structured import captured during parsing/rewriting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpec {
    /// Raw import statement text.
    pub source: String,
    /// Logical kind (hoisted import/export, mapper-required, etc.).
    pub kind: ImportKind,
}

/// Category of import for downstream adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    /// Imported or exported from the document root.
    Hoisted,
    /// Required by directive mapper or other transforms.
    Transform,
}

/// Parses Markdown and rewrites the resulting HTML stream with the default rewrite options.
///
/// This helper hoists top-level `import`/`export` statements from MDX ESM blocks while
/// streaming the remaining Markdown to HTML. The returned [`ParseResult`] exposes both
/// the rendered HTML and the hoisted imports so callers do not need to rescan the input.
pub fn parse(input: &str) -> Result<ParseResult, MarkflowError> {
    parse_with_options(input, RewriteOptions::default())
}

/// Parses Markdown with custom rewrite options, applying directive rewriting before streaming.
pub fn parse_with_options(
    input: &str,
    options: RewriteOptions,
) -> Result<ParseResult, MarkflowError> {
    let events = get_event_iterator(input)?;
    let hoisted = Rc::new(RefCell::new(Vec::new()));
    let directive_count = Rc::new(RefCell::new(0usize));
    let required_imports = options.required_imports.clone();
    let mapper = options.directive_mapper.clone();
    let rewrite_options = options.clone();

    // Compose adapters conditionally based on options using a boxed iterator
    // to keep type uniform across branches.
    let mut events: Box<dyn Iterator<Item = Event<'static>>> = Box::new(events);

    if options.enable_hoist {
        events = Box::new(HoistAdapter::new(events, Rc::clone(&hoisted)));
    }

    if options.enable_directives {
        events = Box::new(DirectiveAdapter::new(
            events,
            Rc::clone(&directive_count),
            mapper,
            Rc::clone(&required_imports),
        ));
    }

    let rewriter = StreamingRewriter::new(Vec::new(), rewrite_options);
    let rewriter = events.stream_to_writer(rewriter)?;

    let mut imports: Vec<ImportSpec> = hoisted
        .borrow()
        .iter()
        .cloned()
        .map(|source| ImportSpec {
            source,
            kind: ImportKind::Hoisted,
        })
        .collect();

    imports.extend(
        required_imports
            .borrow()
            .iter()
            .cloned()
            .map(|source| ImportSpec {
                source,
                kind: ImportKind::Transform,
            }),
    );

    let output = rewriter.into_inner()?;
    let mut html = String::from_utf8(output)?;

    if options.enable_smartypants {
        html = apply_smartypants(&html);
    }

    Ok(ParseResult { html, imports })
}
