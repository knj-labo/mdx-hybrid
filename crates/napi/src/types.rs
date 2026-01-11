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

/// JSX component import mapping entry.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct JsxComponentImport {
    /// JSX tag name (e.g., "Badge").
    pub name: String,
    /// Import statement to hoist (e.g., "import Badge from './Badge.astro';").
    pub import: String,
}

/// Options for JSX rendering (component imports + rewrite options).
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct JsxRenderOptions {
    /// Optional rewrite options (directives/hoist/smartypants/components).
    pub rewrite: Option<RewriteConfig>,
    /// Component import mappings to hoist into the output.
    pub component_imports: Option<Vec<JsxComponentImport>>,
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
    /// Optional JSX render options (component imports + rewrite options).
    pub jsx: Option<JsxRenderOptions>,
    /// Selects the rendering pipeline ("multipass" or "mdast").
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

use markflow_core::{ComponentRegistry, JsxOptions, RewriteOptions};

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

impl From<JsxRenderOptions> for JsxOptions {
    fn from(options: JsxRenderOptions) -> Self {
        let rewrite_options = options
            .rewrite
            .map(RewriteOptions::from)
            .unwrap_or_default();
        let mut components = ComponentRegistry::new();
        if let Some(imports) = options.component_imports {
            for entry in imports {
                components.register_import(entry.name, entry.import);
            }
        }
        JsxOptions {
            rewrite_options,
            components,
        }
    }
}
