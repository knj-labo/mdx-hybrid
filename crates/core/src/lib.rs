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
pub mod streaming_rewriter;

mod html_renderer;

pub use adapter::MarkdownStream;
pub use frontmatter::{FrontmatterError, FrontmatterExtraction, extract_frontmatter};
pub use parse_config::{ParseConfig, ParseConstructs};
pub use streaming_rewriter::{RewriteOptions, StreamingRewriter};

use thiserror::Error;

mod markdown_adapter;
mod parse_config;

use crate::code_fence::collect_root_imports;

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

/// parses Markdown and rewrites the resulting HTML stream with the default rewrite options.
pub fn parse(input: &str) -> Result<String, MarkflowError> {
    let (_, body_lines) = collect_root_imports(input);
    let body = body_lines.join("\n");
    let events = get_event_iterator(&body)?;
    let rewriter = StreamingRewriter::new(Vec::new(), RewriteOptions::default());

    let rewriter = events.stream_to_writer(rewriter)?;

    let output = rewriter.into_inner()?;
    let string = String::from_utf8(output)?;
    Ok(string)
}

/// Iterator alias so callers don't need to depend on the adapter module path.
pub type MarkdownEventStream = markdown_adapter::MarkdownParser;

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
        let output = parse(input).unwrap();
        assert!(output.contains("<h1 id=\"hello-world\">Hello, World!</h1>"));
    }

    #[test]
    fn test_parse_list() {
        let input = "* Item 1\n* Item 2";
        let output = parse(input).unwrap();
        assert!(output.contains("<ul>"));
        assert!(output.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_parse_applies_lazy_loading() {
        let input = "![alt](img.png)";
        let output = parse(input).unwrap();
        assert!(output.contains("loading=\"lazy\""));
    }

    #[test]
    fn test_parse_table_alignment_and_math() {
        let input = "| A | B |\n|:-|:-:|\n| $x$ | $$y$$ |";
        let output = parse(input).unwrap();
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
        let output = parse(input).unwrap();
        assert!(output.contains("frontmatter"));
        assert!(output.contains("title: test"));
    }

    #[test]
    fn test_reference_link_resolves_definition() {
        let input = "[Example][ref]\n\n[ref]: https://example.com \"Example Site\"";
        let output = parse(input).unwrap();
        assert!(output.contains("<a href=\"https://example.com\""));
        assert!(output.contains("title=\"Example Site\""));
    }

    #[test]
    fn test_reference_image_resolves_definition() {
        let input = "![Alt][logo]\n\n[logo]: https://cdn.example.com/logo.png \"Logo\"";
        let output = parse(input).unwrap();
        assert!(output.contains("<img src=\"https://cdn.example.com/logo.png\""));
        assert!(output.contains("title=\"Logo\""));
    }

    #[test]
    fn test_mdx_embedded_jsx_preserved() {
        let input = read_fixture("mdx/embedded-jsx/component.mdx");
        let output = parse(&input).unwrap();
        assert!(output.contains("<Aside title=\"Heads up\">"));
        assert!(output.contains("</Aside>"));
    }

    #[test]
    fn test_mdx_inline_expression_preserved() {
        let input = read_fixture("mdx/expressions/inline.mdx");
        let output = parse(&input).unwrap();
        assert!(
            output.contains("{props.name ?? 'friend'}"),
            "output: {output}"
        );
    }

    #[test]
    fn test_mdx_flow_expression_preserved() {
        let input = read_fixture("mdx/expressions/flow.mdx");
        let output = parse(&input).unwrap();
        assert!(output.contains("steps.join(' → ');"), "output: {output}");
    }

    #[test]
    fn test_mdx_esm_import_preserved() {
        let input = read_fixture("mdx/esm/imports.mdx");
        let output = parse(&input).unwrap();
        assert!(
            !output.contains("import Tabs from"),
            "root-level imports should be hoisted away from HTML output"
        );
    }
}
