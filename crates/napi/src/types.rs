//! NAPI-exposed data structures.

#![allow(deprecated)]

use napi_derive::napi;
use serde::Serialize;
use serde_json::Value as JsonValue;

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

/// DEPRECATED: JSX component imports are no longer used with mdast pipeline.
/// This type will be removed in a future version.
#[deprecated(note = "Component imports are handled by mdast pipeline. This type is unused.")]
#[allow(deprecated)]
#[napi(object)]
#[derive(Debug, Clone)]
pub struct JsxComponentImport {
    /// JSX tag name (e.g., "Badge").
    pub name: String,
    /// Import statement to hoist (e.g., "import Badge from './Badge.astro';").
    pub import: String,
}

/// DEPRECATED: JSX rendering options are no longer used with mdast pipeline.
/// This type will be removed in a future version.
#[deprecated(note = "JSX rendering is handled by mdast pipeline. This type is unused.")]
#[allow(deprecated)]
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct JsxRenderOptions {
    /// Optional rewrite options (directives/hoist/smartypants/components).
    pub rewrite: Option<RewriteConfig>,
    /// Component import mappings to hoist into the output.
    pub component_imports: Option<Vec<JsxComponentImport>>,
    /// Component names whose children should NOT be markdown-processed (e.g., Code, Prism).
    pub code_sample_components: Option<Vec<String>>,
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

/// Options passed to the compiler constructor.
#[allow(deprecated)]
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
    /// DEPRECATED: Pipeline selection removed. mdast is the only pipeline.
    /// This field is ignored and will be removed in a future version.
    #[deprecated(note = "Only mdast pipeline is supported. This field is ignored.")]
    pub pipeline: Option<String>,
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

/// Parse warning returned from Rust
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ParseWarningEntry {
    /// Warning type (e.g., "unclosed_code_fence")
    pub warning_type: String,
    /// Line number where warning occurred
    pub line: u32,
    /// Human-readable message
    pub message: String,
}

/// Diagnostics returned with compilation result
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// Non-fatal warnings
    pub warnings: Vec<ParseWarningEntry>,
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
    /// Parse diagnostics (warnings, not errors)
    pub diagnostics: Diagnostics,
}

/// Neutral IR returned when Astro-compat codegen is disabled.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompileIrResult {
    /// Rendered JSX output (string form).
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
    /// Parse diagnostics (warnings, not errors)
    pub diagnostics: Diagnostics,
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

/// Options for the mdast v2 block renderer.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct BlockOptions {
    /// Inject Starlight CSS classes for components (default: false)
    pub inject_starlight_css: Option<bool>,
    /// Enable directive preprocessing (:::note, etc.). Defaults to true.
    pub enable_directives: Option<bool>,
}

/// Represents a rendering block returned by parse_blocks().
///
/// JavaScript receives this as:
/// ```ts
/// type RenderBlock =
///   | { type: "html", content: string }
///   | { type: "component", name: string, props: Record<string, string>, slotHtml: string }
/// ```
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RenderBlock {
    /// Block type: "html" or "component"
    pub r#type: String,
    /// HTML content (for type="html")
    pub content: Option<String>,
    /// Component name (for type="component")
    pub name: Option<String>,
    /// Component props (for type="component")
    pub props: Option<JsonValue>,
    /// Slot HTML content (for type="component")
    pub slot_html: Option<String>,
}

/// Result of parseBlocks() with blocks and extracted headings.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ParseBlocksResult {
    /// Rendering blocks (HTML or Component).
    pub blocks: Vec<RenderBlock>,
    /// Extracted heading metadata.
    pub headings: Vec<HeadingEntry>,
}

use markflow_core::RewriteOptions;

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
