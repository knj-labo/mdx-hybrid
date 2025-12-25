//! NAPI-exposed data structures.

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
