#![deny(missing_docs)]
//! Node.js bindings that surface Markflow's Rust implementation.

use markflow_core::{MarkdownStream, MarkflowError, RewriteOptions, StreamingRewriter};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value as JsonValue;

/// Configuration options for the HTML rewriter
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RewriteConfig {
    /// Enable lazy loading for images (default: true)
    pub enforce_img_loading_lazy: bool,
}

impl Default for RewriteConfig {
    fn default() -> Self {
        Self {
            enforce_img_loading_lazy: true,
        }
    }
}

impl From<RewriteConfig> for RewriteOptions {
    fn from(config: RewriteConfig) -> Self {
        RewriteOptions {
            enforce_img_loading_lazy: config.enforce_img_loading_lazy,
        }
    }
}

/// Parse result with HTML output and processing statistics
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed HTML output
    pub html: String,
    /// Processing time in milliseconds
    pub processing_time_ms: f64,
}

/// Parsed frontmatter document plus any parser errors.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct FrontmatterResult {
    /// Structured frontmatter data represented as JSON.
    pub frontmatter: JsonValue,
    /// Any syntax or parsing errors surfaced by the extractor.
    pub errors: Vec<String>,
}

/// Parses markdown string to HTML with default options
#[napi]
pub fn parse(input: String) -> napi::Result<String> {
    markflow_core::parse(&input).map_err(convert_error)
}

/// Parses markdown string to HTML with custom rewrite options
#[napi]
pub fn parse_with_options(input: String, config: RewriteConfig) -> napi::Result<String> {
    let events = markflow_core::get_event_iterator(&input).map_err(convert_error)?;
    let options: RewriteOptions = config.into();
    let rewriter = StreamingRewriter::new(Vec::new(), options);

    let rewriter = events.stream_to_writer(rewriter).map_err(convert_error)?;
    let output = rewriter.into_inner().map_err(convert_error)?;
    String::from_utf8(output).map_err(convert_error)
}

/// Parses markdown and returns both HTML output and processing statistics
#[napi]
pub fn parse_with_stats(input: String) -> napi::Result<ParseResult> {
    use std::time::Instant;

    let start = Instant::now();
    let html = parse(input)?;
    let elapsed = start.elapsed();

    Ok(ParseResult {
        html,
        processing_time_ms: elapsed.as_secs_f64() * 1000.0,
    })
}

/// Extracts YAML or TOML frontmatter without compiling the entire Markdown document.
#[napi]
pub fn parse_frontmatter(content: String) -> napi::Result<FrontmatterResult> {
    match extract_frontmatter_block(&content) {
        Ok(Some((kind, block))) => match deserialize_frontmatter(kind, &block) {
            Ok(frontmatter) => Ok(FrontmatterResult {
                frontmatter,
                errors: Vec::new(),
            }),
            Err(err) => Ok(FrontmatterResult {
                frontmatter: empty_frontmatter(),
                errors: vec![err],
            }),
        },
        Ok(None) => Ok(FrontmatterResult {
            frontmatter: empty_frontmatter(),
            errors: Vec::new(),
        }),
        Err(err) => Ok(FrontmatterResult {
            frontmatter: empty_frontmatter(),
            errors: vec![err],
        }),
    }
}

/// Improved error converter that matches on enum variants
fn convert_error<E: Into<MarkflowError>>(err: E) -> Error {
    let err = err.into();
    match err {
        // Map specific errors to specific NAPI statuses
        MarkflowError::EncodingError(e) => {
            Error::new(Status::InvalidArg, format!("Encoding error: {}", e))
        }
        // IO errors and Adapter errors usually imply a runtime failure
        MarkflowError::IoError(e) => Error::from_reason(format!("IO error: {}", e)),
        MarkflowError::MarkdownAdapter(msg) => {
            Error::from_reason(format!("Markdown parser error: {}", msg))
        }
    }
}

#[derive(Copy, Clone)]
enum FrontmatterFormat {
    Yaml,
    Toml,
}

impl FrontmatterFormat {
    fn delimiter(self) -> &'static str {
        match self {
            FrontmatterFormat::Yaml => "---",
            FrontmatterFormat::Toml => "+++",
        }
    }

    fn label(self) -> &'static str {
        match self {
            FrontmatterFormat::Yaml => "YAML",
            FrontmatterFormat::Toml => "TOML",
        }
    }

    fn from_line(line: &str) -> Option<Self> {
        match normalize_line(line) {
            "---" => Some(FrontmatterFormat::Yaml),
            "+++" => Some(FrontmatterFormat::Toml),
            _ => None,
        }
    }
}

fn extract_frontmatter_block(
    input: &str,
) -> std::result::Result<Option<(FrontmatterFormat, String)>, String> {
    let without_bom = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut lines = without_bom.lines();

    let first_line = loop {
        match lines.next() {
            Some(line) if !line.trim().is_empty() => break line,
            Some(_) => continue,
            None => return Ok(None),
        }
    };

    let kind = match FrontmatterFormat::from_line(first_line) {
        Some(kind) => kind,
        None => return Ok(None),
    };

    let mut block: Vec<&str> = Vec::new();
    for line in lines {
        if normalize_line(line) == kind.delimiter() {
            return Ok(Some((kind, block.join("\n"))));
        }
        block.push(line);
    }

    Err(format!(
        "Unterminated {} frontmatter block. Expected closing '{}' line.",
        kind.label(),
        kind.delimiter()
    ))
}

fn deserialize_frontmatter(
    kind: FrontmatterFormat,
    block: &str,
) -> std::result::Result<JsonValue, String> {
    match kind {
        FrontmatterFormat::Yaml => serde_yaml::from_str::<JsonValue>(block)
            .map_err(|err| format!("YAML parse error: {err}")),
        FrontmatterFormat::Toml => {
            let value: toml::Value =
                toml::from_str(block).map_err(|err| format!("TOML parse error: {err}"))?;
            serde_json::to_value(value).map_err(|err| format!("TOML conversion error: {err}"))
        }
    }
}

fn empty_frontmatter() -> JsonValue {
    JsonValue::Object(Default::default())
}

fn normalize_line(line: &str) -> &str {
    line.trim_end_matches('\r')
}

#[cfg(test)]
mod tests {
    use super::{empty_frontmatter, parse_frontmatter};
    use serde_json::Value as JsonValue;

    #[test]
    fn parses_yaml_frontmatter_block() {
        let input = "---\ntitle: Test\n---\nBody".to_string();
        let result = parse_frontmatter(input).unwrap();
        assert!(result.errors.is_empty());
        let title = result
            .frontmatter
            .get("title")
            .and_then(JsonValue::as_str)
            .unwrap();
        assert_eq!(title, "Test");
    }

    #[test]
    fn returns_empty_object_when_no_frontmatter() {
        let result = parse_frontmatter("# Heading".to_string()).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.frontmatter, empty_frontmatter());
    }
}
