#![deny(missing_docs)]
//! Streaming Markdown core utilities: parser, MarkdownStream, and HTML rewriter glue.

/// Markdown event to `io::Write` bridge utilities.
pub mod adapter;
/// Code fence state tracking utilities for import hoisting safeguards.
pub mod code_fence;
/// Core event types that decouple Markflow from pulldown-cmark specifics.
#[allow(missing_docs)]
pub mod event;
/// YAML frontmatter extraction helpers.
pub mod frontmatter;
/// Minimal JSX renderer that preserves raw JSX nodes.
pub mod jsx_renderer;
pub mod streaming_rewriter;

pub mod directives;
mod html_renderer;

pub use adapter::MarkdownStream;
pub use directives::ensure_aside_import;
pub use frontmatter::{FrontmatterError, FrontmatterExtraction, extract_frontmatter};
pub use jsx_renderer::render_to_jsx;
pub use parse_config::{ParseConfig, ParseConstructs};
pub use slug::{Slugger, slugify};
pub use streaming_rewriter::{RewriteOptions, StreamingRewriter};
pub use transform::{DirectiveAdapter, HoistAdapter};

use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;

mod markdown_adapter;
mod parse_config;
mod slug;
/// Streaming transformers (hoist, directives, etc.).
pub mod transform;

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
}

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
    pub imports: Vec<String>,
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

    let pipeline = DirectiveAdapter::new(
        HoistAdapter::new(events, Rc::clone(&hoisted)),
        Rc::clone(&directive_count),
    );

    let rewriter = StreamingRewriter::new(Vec::new(), options);
    let rewriter = pipeline.stream_to_writer(rewriter)?;

    let mut imports = hoisted.borrow().clone();
    ensure_aside_import(&mut imports, *directive_count.borrow());

    let output = rewriter.into_inner()?;
    let html = String::from_utf8(output)?;

    Ok(ParseResult { html, imports })
}

/// Iterator alias so callers don't need to depend on the adapter module path.
pub type MarkdownEventStream = markdown_adapter::MarkdownParser;

#[cfg(test)]
mod jsx_tests {
    use super::render_to_jsx;

    #[test]
    fn jsx_renderer_preserves_raw_jsx() {
        let input = "import X from './x'\n\n<MyComponent />\n";
        let result = render_to_jsx(input).expect("render_to_jsx succeeds");
        assert!(result.starts_with("import X from './x'"));
        assert!(result.contains("<MyComponent />"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/core")
    }

    fn read_fixture(path: &str) -> String {
        fs::read_to_string(fixtures_dir().join(path)).unwrap()
    }

    #[test]
    fn test_parse() {
        let input = "# Hello, World!";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<h1 id=\"hello-world\">Hello, World!</h1>"));
    }

    #[test]
    fn test_parse_list() {
        let input = "* Item 1\n* Item 2";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<ul>"));
        assert!(output.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_parse_applies_lazy_loading() {
        let input = "![alt](img.png)";
        let output = parse(input).unwrap().html;
        assert!(output.contains("loading=\"lazy\""));
    }

    #[test]
    fn test_parse_table_alignment_and_math() {
        let input = "| A | B |\n|:-|:-:|\n| $x$ | $$y$$ |";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<table>"));
        assert!(
            output.contains(
                "<td style=\"text-align:left\"><span class=\"math-inline\">x</span></td>"
            )
        );
        assert!(output.contains("<span class=\"math-inline\">y</span>"));
    }

    #[test]
    fn test_parse_frontmatter_passthrough() {
        let input = "---\ntitle: test\n---\n\ncontent";
        let output = parse(input).unwrap().html;
        assert!(output.contains("frontmatter"));
        assert!(output.contains("title: test"));
    }

    #[test]
    fn test_reference_link_resolves_definition() {
        let input = "[Example][ref]\n\n[ref]: https://example.com \"Example Site\"";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<a href=\"https://example.com\""));
        assert!(output.contains("title=\"Example Site\""));
    }

    #[test]
    fn test_directive_rewrites_to_aside() {
        let input = ":::note[Heads up]\ncontent\n:::";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<Aside type=\"note\" title=\"Heads up\">"));
        assert!(output.contains("</Aside>"));
    }

    #[test]
    fn test_reference_image_resolves_definition() {
        let input = "![Alt][logo]\n\n[logo]: https://cdn.example.com/logo.png \"Logo\"";
        let output = parse(input).unwrap().html;
        assert!(output.contains("<img src=\"https://cdn.example.com/logo.png\""));
        assert!(output.contains("title=\"Logo\""));
    }

    #[test]
    fn test_mdx_embedded_jsx_preserved() {
        let input = read_fixture("mdx/embedded-jsx/component.mdx");
        let output = parse(&input).unwrap().html;
        assert!(output.contains("<Aside title=\"Heads up\">"));
        assert!(output.contains("</Aside>"));
    }

    #[test]
    fn test_mdx_inline_expression_preserved() {
        let input = read_fixture("mdx/expressions/inline.mdx");
        let output = parse(&input).unwrap().html;
        assert!(
            output.contains("{props.name ?? 'friend'}"),
            "output: {output}"
        );
    }

    #[test]
    fn test_mdx_flow_expression_preserved() {
        let input = read_fixture("mdx/expressions/flow.mdx");
        let output = parse(&input).unwrap().html;
        assert!(output.contains("steps.join(' → ');"), "output: {output}");
    }

    #[test]
    fn test_mdx_esm_import_preserved() {
        let input = read_fixture("mdx/esm/imports.mdx");
        let output = parse(&input).unwrap();
        assert!(
            !output.html.contains("import Tabs from"),
            "root-level imports should be hoisted away from HTML output"
        );
        assert!(
            output
                .imports
                .iter()
                .any(|line| line.contains("import Tabs from")),
            "imports should be hoisted into the imports list"
        );
    }
}
