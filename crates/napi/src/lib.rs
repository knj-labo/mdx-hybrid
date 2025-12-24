#![deny(missing_docs)]
//! Node.js bindings that surface Markflow's Rust implementation.

use markflow_core::event::{
    Event as CoreEvent, HeadingLevel, Tag as CoreTag, TagEnd as CoreTagEnd,
};
use markflow_core::{MarkflowError, ParseConfig, RewriteOptions, extract_frontmatter};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
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
    /// Enable directive rewriting (:::note, etc.). Defaults to true.
    pub enable_directives: Option<bool>,
    /// Enable hoisting of root-level import/export statements. Defaults to true.
    pub enable_hoist: Option<bool>,
    /// Enable smart punctuation (quotes, dashes, ellipsis). Defaults to true.
    pub enable_smartypants: Option<bool>,
    /// Enable Astro docs component rewrites (Aside/Steps/Tabs/FileTree). Defaults to true.
    pub enable_components: Option<bool>,
}

impl Default for RewriteConfig {
    fn default() -> Self {
        Self {
            enforce_img_loading_lazy: true,
            enable_directives: None,
            enable_hoist: None,
            enable_smartypants: None,
            enable_components: None,
        }
    }
}

impl From<RewriteConfig> for RewriteOptions {
    fn from(config: RewriteConfig) -> Self {
        RewriteOptions {
            enforce_img_loading_lazy: config.enforce_img_loading_lazy,
            enable_directives: config.enable_directives.unwrap_or(true),
            enable_hoist: config.enable_hoist.unwrap_or(true),
            enable_smartypants: config.enable_smartypants.unwrap_or(true),
            enable_components: config.enable_components.unwrap_or(true),
            ..RewriteOptions::default()
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
    let result = markflow_core::parse(&input).map_err(convert_error)?;
    Ok(result.html)
}

/// Compiles Markdown/MDX and returns a neutral IR; JS adapters produce final framework code.
#[napi(js_name = "compileIr")]
pub fn compile_ir(
    source: String,
    filepath: String,
    options: Option<FileOptions>,
    config: Option<CompilerConfig>,
) -> napi::Result<CompileIrResult> {
    let internal = InternalCompilerConfig::new(config);
    let options = options.unwrap_or_default();
    let effective_path = options.file.clone().unwrap_or_else(|| filepath.clone());
    let file_type = options
        .file_type
        .map(FileType::from)
        .unwrap_or_else(|| FileType::from_path(Path::new(&effective_path)));

    let frontmatter_extraction = extract_frontmatter(&source)
        .map_err(|err| convert_error(MarkflowError::MarkdownAdapter(err.to_string())))?;
    let frontmatter = frontmatter_extraction.value;
    let raw_body = source[frontmatter_extraction.body_start..].to_string();

    let parse_result = markflow_core::parse_with_options(&raw_body, RewriteOptions::default())
        .map_err(convert_error)?;

    let headings = collect_headings(&raw_body, file_type)?;
    let layout_import: Option<String> = frontmatter
        .get("layout")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let frontmatter_json = serde_json::to_string(&frontmatter).unwrap_or_else(|_| "{}".to_string());

    Ok(CompileIrResult {
        html: parse_result.html,
        hoisted_imports: parse_result
            .imports
            .into_iter()
            .map(|spec| ImportSpec {
                source: spec.source,
                kind: match spec.kind {
                    markflow_core::ImportKind::Hoisted => ImportKind::Hoisted,
                    markflow_core::ImportKind::Transform => ImportKind::Transform,
                },
            })
            .collect(),
        frontmatter_json,
        headings,
        file_path: effective_path,
        url: options.url.clone(),
        layout_import,
        runtime_import: internal.jsx_import_source,
    })
}

/// Renders markdown/MDX to a JSX string while preserving raw JSX nodes.
#[napi(js_name = "renderToJsx")]
pub fn render_to_jsx_napi(input: String) -> napi::Result<String> {
    markflow_core::render_to_jsx(&input).map_err(convert_error)
}

/// Parses markdown string to HTML with custom rewrite options
#[napi]
pub fn parse_with_options(input: String, config: RewriteConfig) -> napi::Result<String> {
    let options: RewriteOptions = config.into();
    let result = markflow_core::parse_with_options(&input, options).map_err(convert_error)?;
    Ok(result.html)
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
    match extract_frontmatter(&content) {
        Ok(result) => Ok(FrontmatterResult {
            frontmatter: result.value,
            errors: Vec::new(),
        }),
        Err(err) => Ok(FrontmatterResult {
            frontmatter: empty_frontmatter(),
            errors: vec![err.to_string()],
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
    /// Overrides the module used for JSX runtime helpers.
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
    /// Explicitly sets the file type so callers can override extension-based detection.
    pub file_type: Option<FileInputType>,
}

/// File categories supported by the compiler.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInputType {
    /// Standard Markdown (.md) without MDX extensions.
    Markdown,
    /// Full MDX documents (.mdx) with JSX/ESM hoisting.
    Mdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileType {
    Markdown,
    Mdx,
}

impl FileType {
    fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("mdx"))
            .map(|is_mdx| {
                if is_mdx {
                    FileType::Mdx
                } else {
                    FileType::Markdown
                }
            })
            .unwrap_or(FileType::Markdown)
    }
}

impl From<FileInputType> for FileType {
    fn from(value: FileInputType) -> Self {
        match value {
            FileInputType::Markdown => FileType::Markdown,
            FileInputType::Mdx => FileType::Mdx,
        }
    }
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

/// Neutral IR returned when Astro-compat codegen is disabled.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileIrResult {
    /// Rendered HTML output.
    pub html: String,
    /// Hoisted imports/exports captured during parsing (structured).
    pub hoisted_imports: Vec<ImportSpec>,
    /// Serialized frontmatter JSON string.
    pub frontmatter_json: String,
    /// Heading metadata collected during parsing.
    pub headings: Vec<HeadingEntry>,
    /// Absolute or workspace-relative file path of the source.
    pub file_path: String,
    /// Route URL (if provided) associated with the file.
    pub url: Option<String>,
    /// Layout import path extracted from frontmatter (if any).
    pub layout_import: Option<String>,
    /// JSX runtime import source to be used by JS adapters.
    pub runtime_import: String,
}

/// Structured import returned by the compiler IR.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ImportSpec {
    /// Raw import/export statement text.
    pub source: String,
    /// Logical kind (hoisted or transform-required).
    pub kind: ImportKind,
}

/// Import category surfaced to JS callers.
#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    /// Import/export lifted from document root.
    Hoisted,
    /// Import required by transforms (e.g., directive mapper).
    Transform,
}

#[derive(Debug, Clone)]
struct InternalCompilerConfig {
    jsx_import_source: String,
}

impl InternalCompilerConfig {
    fn new(config: Option<CompilerConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        let jsx_import_source = cfg
            .jsx_import_source
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
    ///
    /// Internally this delegates to `compile_ir` for parsing/rewriting, then
    /// formats the legacy Astro module code. A future adapter hook can replace
    /// the codegen step without changing the JS-facing signature.
    #[napi(js_name = "compile")]
    pub fn compile_mdx(
        &self,
        source: String,
        filepath: String,
        options: Option<FileOptions>,
    ) -> napi::Result<CompileResult> {
        // Parse to IR first (framework-agnostic data).
        let ir = compile_ir(
            source.clone(),
            filepath.clone(),
            options.clone(),
            Some(CompilerConfig {
                jsx_import_source: Some(self.config.jsx_import_source.clone()),
                ..CompilerConfig::default()
            }),
        )?;

        compile_document_from_ir(ir)
    }
}

/// Helper factory to share a compiler instance across the Vite plugin lifecycle.
#[napi]
/// Helper factory exposed to JavaScript for ergonomic reuse.
pub fn create_compiler(config: Option<CompilerConfig>) -> MarkflowCompiler {
    MarkflowCompiler::new(config)
}

#[cfg(test)]
fn compile_document(
    config: &InternalCompilerConfig,
    source: String,
    filepath: String,
    options: Option<FileOptions>,
    hoisted_imports: Vec<String>,
) -> napi::Result<CompileResult> {
    let mut ir = compile_ir(
        source,
        filepath,
        options,
        Some(CompilerConfig {
            jsx_import_source: Some(config.jsx_import_source.clone()),
            ..CompilerConfig::default()
        }),
    )?;

    if !hoisted_imports.is_empty() {
        ir.hoisted_imports
            .extend(hoisted_imports.into_iter().map(|source| ImportSpec {
                source,
                kind: ImportKind::Hoisted,
            }));
    }

    compile_document_from_ir(ir)
}

fn compile_document_from_ir(ir: CompileIrResult) -> napi::Result<CompileResult> {
    let hoisted_imports = dedupe_imports(
        ir.hoisted_imports
            .iter()
            .map(|spec| spec.source.clone())
            .collect(),
    );
    let headings_json =
        serde_json::to_string(&ir.headings).unwrap_or_else(|_| "[]".to_string());
    let code = generate_module_code_from_ir(&ir, &hoisted_imports, &headings_json)?;
    let imports = build_import_list(ir.layout_import.as_deref(), Path::new(&ir.file_path));

    Ok(CompileResult {
        code,
        map: None,
        frontmatter_json: ir.frontmatter_json,
        headings: ir.headings,
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

fn collect_headings(body: &str, file_type: FileType) -> napi::Result<Vec<HeadingEntry>> {
    let parse_config = match file_type {
        FileType::Markdown => ParseConfig::markdown(),
        FileType::Mdx => ParseConfig::mdx(),
    };
    let events =
        markflow_core::get_event_iterator_with_config(body, parse_config).map_err(convert_error)?;
    let mut collector = HeadingCollector::new();
    for event in events {
        collector.observe(&event);
    }
    Ok(collector.into_entries())
}

struct HeadingCollector {
    headings: Vec<HeadingEntry>,
    slugger: markflow_core::Slugger,
    current_heading: Option<HeadingCapture>,
}

impl HeadingCollector {
    fn new() -> Self {
        Self {
            headings: Vec::new(),
            slugger: markflow_core::Slugger::new(),
            current_heading: None,
        }
    }

    fn record_heading(&mut self, text: &str, level: HeadingLevel, id: Option<String>) {
        let slug = id.unwrap_or_else(|| self.slugger.next_slug(text));
        self.headings.push(HeadingEntry {
            depth: level as u8,
            slug,
            text: text.to_string(),
        });
    }

    fn begin_heading(&mut self, level: HeadingLevel, id: Option<String>) {
        self.current_heading = Some(HeadingCapture {
            level,
            buffer: String::new(),
            id,
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
                self.record_heading(text, level, capture.id.clone());
            }
        }
    }

    fn observe<'a>(&mut self, event: &CoreEvent<'a>) {
        match event {
            CoreEvent::Start(CoreTag::Heading { level, id, .. }) => {
                let slug_from_tag = id.as_ref().map(|cow| cow.to_string());
                self.begin_heading(*level, slug_from_tag)
            }
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
    id: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn generate_module_code_from_ir(
    ir: &CompileIrResult,
    hoisted_imports: &[String],
    headings_json: &str,
) -> napi::Result<String> {
    let mut code = String::new();
    writeln!(
        code,
        "import {{ createComponent, markHTMLString }} from '{}';",
        ir.runtime_import
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    if ir.layout_import.is_some() {
        writeln!(
            code,
            "import {{ renderComponentToString }} from '{}';",
            ASTRO_RENDER_HELPERS
        )
        .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    if let Some(layout) = ir.layout_import.as_deref() {
        writeln!(code, "import Layout from {};", js_string_literal(layout))
            .map_err(|err| Error::from_reason(err.to_string()))?;
    }

    for import in hoisted_imports {
        writeln!(code, "{}", import).map_err(|err| Error::from_reason(err.to_string()))?;
    }

    writeln!(
        code,
        "const _html = `{}`;",
        escape_template_literal(&ir.html)
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "const _markflowHtml = markHTMLString(_html);")
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(code, "export const frontmatter = {};", ir.frontmatter_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(
        code,
        "export const file = {};",
        js_string_literal(&ir.file_path)
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    let url_literal = ir
        .url
        .as_deref()
        .map(js_string_literal)
        .unwrap_or_else(|| "undefined".to_string());
    writeln!(code, "export const url = {};", url_literal)
        .map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(code, "export const headings = {};", headings_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "export function getHeadings() {{")
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "  return {};", headings_json)
        .map_err(|err| Error::from_reason(err.to_string()))?;
    writeln!(code, "}}").map_err(|err| Error::from_reason(err.to_string()))?;

    writeln!(
        code,
        "const _MarkflowContent = createComponent(async () => _markflowHtml);"
    )
    .map_err(|err| Error::from_reason(err.to_string()))?;

    if ir.layout_import.is_some() {
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

fn normalize_import_key(source: &str) -> String {
    let mut key = String::with_capacity(source.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut escape = false;

    for ch in source.chars() {
        if escape {
            key.push(ch);
            escape = false;
            continue;
        }

        if ch == '\\' && (in_single || in_double || in_backtick) {
            key.push(ch);
            escape = true;
            continue;
        }

        match ch {
            '\'' if !in_double && !in_backtick => {
                in_single = !in_single;
                key.push(ch);
            }
            '"' if !in_single && !in_backtick => {
                in_double = !in_double;
                key.push(ch);
            }
            '`' if !in_single && !in_double => {
                in_backtick = !in_backtick;
                key.push(ch);
            }
            ch if ch.is_whitespace() && !(in_single || in_double || in_backtick) => {}
            _ => key.push(ch),
        }
    }

    key
}

fn dedupe_imports(mut imports: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(imports.len());
    for import in imports.drain(..) {
        let key = normalize_import_key(&import);
        if seen.insert(key) {
            deduped.push(import);
        }
    }
    deduped
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

fn empty_frontmatter() -> JsonValue {
    JsonValue::Object(Default::default())
}

#[cfg(test)]
mod tests {
    use super::{InternalCompilerConfig, compile_document, empty_frontmatter, parse_frontmatter};
    use crate::render_to_jsx_napi;
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

    #[test]
    fn render_to_jsx_preserves_raw_jsx() {
        let input = "import X from './x'\n\n<MyComponent />".to_string();
        let output = render_to_jsx_napi(input).expect("render_to_jsx succeeds");
        assert!(output.starts_with("import X from './x'"));
        assert!(output.contains("<MyComponent />"));
    }

    #[test]
    fn compile_document_emits_frontmatter_json() {
        let config = InternalCompilerConfig::new(None);
        let source = "---\ntitle: Test\n---\n# Hello".to_string();
        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");
        assert_eq!(result.frontmatter_json, "{\"title\":\"Test\"}");
        assert!(
            result
                .code
                .contains("export const frontmatter = {\"title\":\"Test\"};"),
            "code: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_handles_missing_frontmatter() {
        let config = InternalCompilerConfig::new(None);
        let source = "# Hello".to_string();
        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");
        assert_eq!(result.frontmatter_json, "{}");
        assert!(
            result.code.contains("export const frontmatter = {};"),
            "code: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_root_imports() {
        let config = InternalCompilerConfig::new(None);
        let source = "import X from './x';\n\n# Title".to_string();
        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");
        assert!(
            result.code.contains("import X from './x';"),
            "code missing hoisted import: {}",
            result.code
        );
        assert!(
            !result.code.contains("const _html = `import X from './x';`"),
            "import leaked into HTML template: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_ignores_imports_inside_fences() {
        let config = InternalCompilerConfig::new(None);
        let source = "```\nimport Y from './y'\n```\n\n# Title".to_string();
        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");
        assert!(
            !result.code.contains("import Y from './y'"),
            "fenced import should not hoist: {}",
            result.code
        );
        assert!(
            result.code.contains("import Y from &#39;./y&#39;"),
            "fenced import should remain in rendered HTML: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_exports_variants() {
        let config = InternalCompilerConfig::new(None);
        let source = "\
export const foo = () => {\n  return 1\n}\n\
export default function bar()\n{\n  return foo();\n}\n\
export { foo };\n\
\n# Title"
            .to_string();

        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");

        // hoisted exports appear before _html declaration
        let hoist_pos = result.code.find("export const foo").unwrap();
        let html_pos = result.code.find("const _html = `").unwrap();
        assert!(
            hoist_pos < html_pos,
            "exports should be hoisted before HTML: {}",
            result.code
        );

        // exports should not leak into HTML template
        assert!(
            !result.code.contains("const _html = `export const foo"),
            "exports leaked into HTML: {}",
            result.code
        );

        // default export body hoisted, not in HTML
        assert!(
            result.code.contains("export default function bar()"),
            "default export missing: {}",
            result.code
        );
        assert!(
            !result.code.contains("bar()\\n{\\n  return foo();\\n}\\n"),
            "default export text appeared in HTML: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_does_not_hoist_exports_inside_fence() {
        let config = InternalCompilerConfig::new(None);
        let source = "```\nexport const no = true\n```\n\nexport const yes = true;".to_string();
        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");

        assert!(
            result.code.contains("export const yes = true;"),
            "export outside fence should hoist: {}",
            result.code
        );
        assert!(
            result
                .code
                .contains("const _html = `<pre><code>export const no = true</code></pre>"),
            "fenced export should stay in HTML: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_export_edge_cases() {
        let config = InternalCompilerConfig::new(None);
        let source = "\
export default async () => {\n  return 1\n}\n\
export * from './mod';\n\
export const foo = 1 // inline\n\
\n# Title"
            .to_string();

        let result = compile_document(&config, source, "test.mdx".into(), None, Vec::new())
            .expect("compile success");

        let html_pos = result.code.find("const _html = `").unwrap();
        assert!(
            result.code.find("export default async () => {").unwrap() < html_pos,
            "default async export should be hoisted"
        );
        assert!(
            result.code.find("export * from './mod';").unwrap() < html_pos,
            "export * should be hoisted"
        );
        assert!(
            result.code.find("export const foo = 1 // inline").unwrap() < html_pos,
            "inline comment export should be hoisted"
        );
        assert!(
            !result
                .code
                .contains("export const foo = 1 // inline\\n# Title"),
            "export should not leak into HTML: {}",
            result.code
        );
    }
}
