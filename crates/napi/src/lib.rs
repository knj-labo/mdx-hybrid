#![deny(missing_docs)]
//! Node.js bindings that surface Markflow's Rust implementation.

use markflow_core::event::{
    Event as CoreEvent, HeadingLevel, Tag as CoreTag, TagEnd as CoreTagEnd,
};
use markflow_core::{MarkdownStream, MarkflowError, RewriteOptions, StreamingRewriter};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

const ASTRO_DEFAULT_RUNTIME: &str = "astro/runtime/server/index.js";
const ASTRO_RENDER_HELPERS: &str = "astro/runtime/server/render/index.js";

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
        Ok(Some((kind, block, _body_start))) => match deserialize_frontmatter(kind, &block) {
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

/// Options passed to the compiler constructor.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct CompilerConfig {
    /// Enables GFM extensions (currently always on; placeholder for parity).
    pub gfm: Option<bool>,
    /// Enables smart punctuation substitutions (placeholder flag).
    pub smartypants: Option<bool>,
    /// Enables syntax highlighting (placeholder flag).
    pub syntax_highlighting: Option<bool>,
    /// Overrides the module used for Astro runtime helpers.
    pub jsx_import_source: Option<String>,
}

/// File-specific overrides that accompany each compilation.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct FileOptions {
    /// Route URL that Astro associates with the file.
    pub url: Option<String>,
    /// Absolute file path (overrides the `filepath` argument when provided).
    pub file: Option<String>,
}

/// Heading metadata returned from the compiler.
#[napi(object)]
#[derive(Debug, Clone, Serialize)]
pub struct HeadingEntry {
    /// Heading depth (1-6).
    pub depth: u8,
    /// Slugified identifier.
    pub slug: String,
    /// Visible heading text.
    pub text: String,
}

/// Imported module referenced by the compiled output.
#[napi(object)]
#[derive(Debug, Clone, Serialize)]
pub struct ImportedModule {
    /// Resolved file path of the import.
    pub path: String,
    /// Logical category (`layout`, `component`, etc.).
    pub kind: String,
}

/// Result returned by the streaming compiler.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Compiled JavaScript/JSX module text.
    pub code: String,
    /// Source map in v3 format (null when unavailable).
    pub map: Option<String>,
    /// JSON string containing serialized frontmatter.
    pub frontmatter_json: String,
    /// Heading metadata collected during compilation.
    pub headings: Vec<HeadingEntry>,
    /// Dependencies referenced while compiling (layouts/imports).
    pub imports: Vec<ImportedModule>,
}

#[derive(Debug, Clone)]
struct InternalCompilerConfig {
    jsx_import_source: String,
}

impl InternalCompilerConfig {
    fn new(config: Option<CompilerConfig>) -> Self {
        let jsx_import_source = config
            .and_then(|cfg| cfg.jsx_import_source)
            .unwrap_or_else(|| ASTRO_DEFAULT_RUNTIME.to_string());

        Self { jsx_import_source }
    }
}

/// Stateful compiler exposed to Node callers.
#[napi]
pub struct MarkflowCompiler {
    config: InternalCompilerConfig,
}

#[napi]
impl MarkflowCompiler {
    #[napi(constructor)]
    /// Creates a compiler that can be reused across Vite transform hooks.
    pub fn new(config: Option<CompilerConfig>) -> Self {
        Self {
            config: InternalCompilerConfig::new(config),
        }
    }

    /// Compiles Markdown/MDX into an Astro-compatible module string.
    #[napi(js_name = "compile")]
    pub fn compile_mdx(
        &self,
        source: String,
        filepath: String,
        options: Option<FileOptions>,
    ) -> napi::Result<CompileResult> {
        compile_document(&self.config, source, filepath, options)
    }
}

/// Helper factory to share a compiler instance across the Vite plugin lifecycle.
#[napi]
/// Helper factory exposed to JavaScript for ergonomic reuse.
pub fn create_compiler(config: Option<CompilerConfig>) -> MarkflowCompiler {
    MarkflowCompiler::new(config)
}

fn compile_document(
    config: &InternalCompilerConfig,
    source: String,
    filepath: String,
    options: Option<FileOptions>,
) -> napi::Result<CompileResult> {
    let (frontmatter, body_start) = match extract_frontmatter_block(&source) {
        Ok(Some((kind, block, body_index))) => match deserialize_frontmatter(kind, &block) {
            Ok(frontmatter) => (frontmatter, body_index),
            Err(err) => {
                return Err(convert_error(MarkflowError::MarkdownAdapter(format!(
                    "Frontmatter parse error: {err}"
                ))));
            }
        },
        Ok(None) => (empty_frontmatter(), 0),
        Err(err) => {
            return Err(convert_error(MarkflowError::MarkdownAdapter(format!(
                "Frontmatter parse error: {err}"
            ))));
        }
    };

    let body = source[body_start..].to_string();
    let mut heading_collector = HeadingCollector::new();
    let layout_import: Option<String> = frontmatter
        .get("layout")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let runtime_import = config.jsx_import_source.clone();
    let file_tag = options
        .as_ref()
        .and_then(|opts| opts.file.clone())
        .unwrap_or_else(|| filepath.clone());
    let url = options.and_then(|opts| opts.url);

    let html = render_document_to_html(&body, &mut heading_collector)?;
    let headings = heading_collector.into_entries();

    let code = generate_module_code(
        &runtime_import,
        layout_import.as_deref(),
        &html,
        &frontmatter,
        &file_tag,
        url.as_deref(),
        &headings,
    )?;

    let imports = build_import_list(layout_import.as_deref(), Path::new(&filepath));
    let frontmatter_json = serde_json::to_string(&frontmatter).unwrap_or_else(|_| "{}".to_string());

    Ok(CompileResult {
        code,
        map: None,
        frontmatter_json,
        headings,
        imports,
    })
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
) -> std::result::Result<Option<(FrontmatterFormat, String, usize)>, String> {
    let (without_bom, bom_len) = if let Some(stripped) = input.strip_prefix('\u{feff}') {
        (stripped, '\u{feff}'.len_utf8())
    } else {
        (input, 0)
    };

    let mut cursor = 0usize;

    // Skip leading blank lines while tracking byte offset.
    loop {
        match next_line(without_bom, cursor) {
            Some((line, next_cursor)) => {
                if line.trim().is_empty() {
                    cursor = next_cursor;
                    continue;
                }
                let kind = match FrontmatterFormat::from_line(line) {
                    Some(kind) => kind,
                    None => return Ok(None),
                };
                let block_start = next_cursor;
                let mut scan_cursor = next_cursor;

                loop {
                    match next_line(without_bom, scan_cursor) {
                        Some((block_line, next_line_cursor)) => {
                            if normalize_line(block_line) == kind.delimiter() {
                                let block_slice = &without_bom[block_start..scan_cursor];
                                let body_index = bom_len + next_line_cursor;
                                return Ok(Some((
                                    kind,
                                    block_slice.trim_end_matches(['\r', '\n']).to_string(),
                                    body_index,
                                )));
                            }
                            scan_cursor = next_line_cursor;
                        }
                        None => {
                            return Err(format!(
                                "Unterminated {} frontmatter block. Expected closing '{}' line.",
                                kind.label(),
                                kind.delimiter()
                            ));
                        }
                    }
                }
            }
            None => return Ok(None),
        }
    }
}

fn next_line(input: &str, start: usize) -> Option<(&str, usize)> {
    if start >= input.len() {
        return None;
    }

    let bytes = &input.as_bytes()[start..];
    if let Some(pos) = bytes.iter().position(|b| *b == b'\n') {
        let line_end = start + pos;
        let line = &input[start..line_end];
        Some((line, line_end + 1))
    } else {
        Some((&input[start..], input.len()))
    }
}

fn render_document_to_html(
    body: &str,
    heading_collector: &mut HeadingCollector,
) -> napi::Result<String> {
    let events = markflow_core::get_event_iterator(body).map_err(convert_error)?;
    let tracked_events = HeadingTrackingStream::new(events, heading_collector);
    let writer = StreamingRewriter::new(Vec::new(), RewriteOptions::default());
    let writer = tracked_events
        .stream_to_writer(writer)
        .map_err(convert_error)?;
    let buffer = writer.into_inner().map_err(convert_error)?;
    String::from_utf8(buffer).map_err(convert_error)
}

struct HeadingTrackingStream<'collector, 'event, I>
where
    I: Iterator<Item = CoreEvent<'event>>,
{
    inner: I,
    collector: &'collector mut HeadingCollector,
}

impl<'collector, 'event, I> HeadingTrackingStream<'collector, 'event, I>
where
    I: Iterator<Item = CoreEvent<'event>>,
{
    fn new(inner: I, collector: &'collector mut HeadingCollector) -> Self {
        Self { inner, collector }
    }
}

impl<'collector, 'event, I> Iterator for HeadingTrackingStream<'collector, 'event, I>
where
    I: Iterator<Item = CoreEvent<'event>>,
{
    type Item = CoreEvent<'event>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.inner.next()?;
        self.collector.observe(&event);
        Some(event)
    }
}

struct HeadingCollector {
    headings: Vec<HeadingEntry>,
    heading_counts: HashMap<String, usize>,
    current_heading: Option<HeadingCapture>,
}

impl HeadingCollector {
    fn new() -> Self {
        Self {
            headings: Vec::new(),
            heading_counts: HashMap::new(),
            current_heading: None,
        }
    }

    fn record_heading(&mut self, text: &str, level: HeadingLevel) {
        let base = slugify(text);
        let next_count = match self.heading_counts.get_mut(&base) {
            Some(count) => {
                *count += 1;
                *count
            }
            None => {
                self.heading_counts.insert(base.clone(), 0);
                0
            }
        };

        let slug = if next_count == 0 {
            base
        } else {
            format!("{}-{}", base, next_count)
        };

        self.headings.push(HeadingEntry {
            depth: level as u8,
            slug,
            text: text.to_string(),
        });
    }

    fn begin_heading(&mut self, level: HeadingLevel) {
        self.current_heading = Some(HeadingCapture {
            level,
            buffer: String::new(),
        });
    }

    fn push_heading_text(&mut self, text: &str) {
        if let Some(capture) = self.current_heading.as_mut() {
            capture.buffer.push_str(text);
        }
    }

    fn end_heading(&mut self, level: HeadingLevel) {
        if let Some(capture) = self.current_heading.take()
            && level == capture.level
        {
            let text = capture.buffer.trim();
            if !text.is_empty() {
                self.record_heading(text, level);
            }
        }
    }

    fn observe<'a>(&mut self, event: &CoreEvent<'a>) {
        match event {
            CoreEvent::Start(CoreTag::Heading { level, .. }) => self.begin_heading(*level),
            CoreEvent::End(CoreTagEnd::Heading(level)) => self.end_heading(*level),
            CoreEvent::Text(text)
            | CoreEvent::Code(text)
            | CoreEvent::Html(text)
            | CoreEvent::InlineHtml(text)
            | CoreEvent::InlineMath(text)
            | CoreEvent::DisplayMath(text) => self.push_heading_text(text.as_ref()),
            CoreEvent::SoftBreak | CoreEvent::HardBreak => self.push_heading_text(" "),
            _ => {}
        }
    }

    fn into_entries(self) -> Vec<HeadingEntry> {
        self.headings
    }
}

struct HeadingCapture {
    level: HeadingLevel,
    buffer: String,
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if (ch.is_whitespace() || matches!(ch, '-' | '_' | ':' | '.'))
            && !last_dash
            && !slug.is_empty()
        {
            slug.push('-');
            last_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "heading".to_string()
    } else {
        slug
    }
}

fn generate_module_code(
    runtime_module: &str,
    layout_import: Option<&str>,
    html: &str,
    frontmatter: &JsonValue,
    file_path: &str,
    url: Option<&str>,
    headings: &[HeadingEntry],
) -> napi::Result<String> {
    let mut code = String::new();
    writeln!(
        code,
        "import {{ createComponent, markHTMLString }} from '{}';",
        runtime_module
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    if layout_import.is_some() {
        writeln!(
            code,
            "import {{ renderComponentToString }} from '{}';",
            ASTRO_RENDER_HELPERS
        )
        .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    if let Some(layout) = layout_import {
        writeln!(code, "import Layout from {};", js_string_literal(layout))
            .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    writeln!(code, "const _html = `{}`;", escape_template_literal(html))
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "const _markflowHtml = markHTMLString(_html);")
        .map_err(|err| Error::from_reason(err.to_string()))?;

    let frontmatter_literal =
        serde_json::to_string(frontmatter).unwrap_or_else(|_| "{}".to_string());
    writeln!(code, "export const frontmatter = {};", frontmatter_literal)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(
        code,
        "export const file = {};",
        js_string_literal(file_path)
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    let url_literal = url
        .map(js_string_literal)
        .unwrap_or_else(|| "undefined".to_string());
    writeln!(code, "export const url = {};", url_literal)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    let headings_literal = serde_json::to_string(headings).unwrap_or_else(|_| "[]".to_string());
    writeln!(code, "export function getHeadings() {{")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "  return {};", headings_literal)
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "}}").map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(
        code,
        "const _MarkflowContent = createComponent(async () => _markflowHtml);"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    if layout_import.is_some() {
        writeln!(
            code,
            "const _MarkflowPage = createComponent(async (result, props, slots) => {{"
        )
        .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(
            code,
            "  const html = await renderComponentToString(result, 'Layout', Layout, {{ ...props, frontmatter }}, {{"
        )
        .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(
            code,
            "    'default': () => _MarkflowContent(result, props, slots)"
        )
        .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "  }});").map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "  return markHTMLString(html);")
            .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "}});").map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "export const Content = _MarkflowContent;")
            .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "export default _MarkflowPage;")
            .map_err(|err| Error::from_reason(err.to_string()))?;
    } else {
        writeln!(code, "export const Content = _MarkflowContent;")
            .map_err(|err| Error::from_reason(err.to_string()))?;
        writeln!(code, "export default _MarkflowContent;")
            .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    Ok(code)
}

fn build_import_list(layout: Option<&str>, filepath: &Path) -> Vec<ImportedModule> {
    let mut imports = Vec::new();
    if let Some(layout_path) = layout {
        let resolved = filepath
            .parent()
            .map(|dir| dir.join(layout_path))
            .unwrap_or_else(|| PathBuf::from(layout_path));
        imports.push(ImportedModule {
            path: resolved.to_string_lossy().to_string(),
            kind: "layout".to_string(),
        });
    }
    imports
}

fn escape_template_literal(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

fn js_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
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
