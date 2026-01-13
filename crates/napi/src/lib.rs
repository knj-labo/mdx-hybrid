#![deny(missing_docs)]
//! Node.js bindings that surface Markflow's Rust implementation.

use markflow_core::{MarkflowError, RewriteOptions, extract_frontmatter};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::Path;

/// Module code generation helpers.
mod codegen;
/// The stateful compiler and its configuration.
pub mod compiler;
/// Heading extraction helpers.
mod headings;
/// NAPI-exposed data structures.
pub mod types;
/// Utility helpers.
mod utils;
#[allow(deprecated)]
pub use types::*;
use utils::empty_frontmatter;
pub(crate) use utils::{build_import_list, dedupe_imports};

/// Parses markdown string to HTML with default options
#[napi]
pub fn parse(input: String) -> napi::Result<String> {
    let result = markflow_core::parse(&input).map_err(convert_error)?;
    Ok(result.html)
}

#[cfg(test)]
fn wrap_jsx_fragment_as_module(input: &str) -> String {
    let mut imports = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_import_block = true;

    for line in input.lines() {
        if in_import_block && line.trim_start().starts_with("import ") {
            imports.push(line);
            continue;
        }

        if in_import_block && line.trim().is_empty() {
            continue;
        }

        in_import_block = false;
        body_lines.push(line);
    }

    let imports = if imports.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", imports.join("\n"))
    };
    let body = body_lines.join("\n");

    format!(
        "{imports}export default function _Tmp() {{\n  return (\n    <>\n{body}\n    </>\n  );\n}}\n"
    )
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

/// Parses markdown into structured RenderBlock objects using the mdast v2 renderer.
///
/// This function uses the Block Architecture to return a structured representation
/// of the markdown content, allowing JavaScript to dynamically map component names
/// to actual Astro components without hardcoding in Rust.
///
/// # Arguments
///
/// * `input` - The markdown text to parse
/// * `opts` - Optional configuration object with:
///   - `inject_starlight_css`: boolean (default: false)
///   - `enable_directives`: boolean (default: true)
///
/// # Returns
///
/// Returns an array of RenderBlock objects. Each block is either:
/// - `{type: "html", content: "<p>...</p>"}` - Plain HTML content
/// - `{type: "component", name: "note", props: {title: "..."}, slotHtml: "..."}` - Component block
///
/// # Example (JavaScript)
///
/// ```javascript
/// const { parseBlocks } = require('markflow-napi');
///
/// const input = `:::note[Important]
/// This is **bold** text.
/// :::`;
///
/// const blocks = parseBlocks(input, { enable_directives: true });
/// // blocks = [
/// //   {
/// //     type: "component",
/// //     name: "note",
/// //     props: { title: "Important" },
/// //     slotHtml: "<p>This is <strong>bold</strong> text.</p>"
/// //   }
/// // ]
/// ```
#[napi(js_name = "parseBlocks")]
pub fn parse_blocks(input: String, opts: Option<BlockOptions>) -> napi::Result<ParseBlocksResult> {
    use markflow_core::renderer::mdast;

    // Parse options from JavaScript
    let options = if let Some(o) = opts {
        mdast::Options {
            inject_starlight_css: o.inject_starlight_css.unwrap_or(false),
            enable_directives: o.enable_directives.unwrap_or(true),
        }
    } else {
        mdast::Options {
            inject_starlight_css: false,
            enable_directives: true,
        }
    };

    // Parse markdown to blocks and extract headings
    let result = mdast::to_blocks(&input, &options)
        .map_err(|e| Error::from_reason(format!("Failed to parse blocks: {}", e)))?;

    // Convert core RenderBlock to NAPI RenderBlock
    let blocks: Vec<RenderBlock> = result
        .blocks
        .into_iter()
        .map(|block| match block {
            mdast::RenderBlock::Html { content } => RenderBlock {
                r#type: "html".to_string(),
                content: Some(content),
                name: None,
                props: None,
                slot_html: None,
            },
            mdast::RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                // Convert HashMap<String, String> to JsonValue
                let props_json = serde_json::to_value(&props)
                    .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

                RenderBlock {
                    r#type: "component".to_string(),
                    content: None,
                    name: Some(name),
                    props: Some(props_json),
                    slot_html: Some(slot_html),
                }
            }
        })
        .collect();

    // Convert headings
    let headings: Vec<HeadingEntry> = result
        .headings
        .into_iter()
        .map(|h| HeadingEntry {
            depth: h.depth,
            slug: h.slug,
            text: h.text,
        })
        .collect();

    Ok(ParseBlocksResult { blocks, headings })
}

/// Represents the type of the input file, either Markdown or MDX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Standard Markdown file.
    Markdown,
    /// MDX (Markdown with JSX) file.
    Mdx,
}

impl FileType {
    #[allow(dead_code)]
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
        MarkflowError::RenderError(msg) => {
            Error::new(Status::InvalidArg, format!("Render error: {}", msg))
        }
        MarkflowError::UnknownComponent(name) => {
            Error::new(Status::InvalidArg, format!("Unknown component: {}", name))
        }
        MarkflowError::InternalError(msg) => Error::from_reason(format!("Internal error: {}", msg)),
    }
}

#[cfg(test)]
mod tests {
    use super::{empty_frontmatter, parse_frontmatter};
    use crate::compiler::InternalCompilerConfig;
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
    fn wrap_jsx_fragment_as_module_preserves_imports_and_body() {
        let input = "import X from './x'\n\n<div />".to_string();
        let output = super::wrap_jsx_fragment_as_module(&input);
        assert!(output.starts_with("import X from './x'"));
        assert!(output.contains("export default function _Tmp"));
        assert!(output.contains("<div />"));
    }

    #[test]
    fn compile_document_emits_frontmatter_json() {
        let config = InternalCompilerConfig::new(None);
        let source = "---\ntitle: Test\n---\n# Hello".to_string();
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();
        let hoist_pos = result.code.find("import X from './x';").unwrap();
        assert!(
            hoist_pos < content_pos,
            "import should be hoisted before JSX content: {}",
            result.code
        );
        assert_eq!(
            result.code.matches("import X from './x';").count(),
            1,
            "hoisted import should not appear in JSX body: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_ignores_imports_inside_fences() {
        let config = InternalCompilerConfig::new(None);
        let source = "```\nimport Y from './y'\n```\n\n# Title".to_string();
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();
        let fenced_pos = result.code.find("import Y from './y'").unwrap();
        assert!(
            fenced_pos > content_pos,
            "fenced import should remain in JSX body: {}",
            result.code
        );
        assert_eq!(
            result.code.matches("import Y from './y'").count(),
            1,
            "fenced import should not be hoisted: {}",
            result.code
        );
        assert!(
            result.code.contains("<pre><code") && result.code.contains("import Y from './y'"),
            "fenced import should stay in rendered JSX: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_multiline_leading_exports() {
        let config = InternalCompilerConfig::new(None);
        // Test multi-line arrow function export at document start
        let source =
            "export const foo = () => {\n  return 1\n}\n\nexport { foo };\n\n# Title".to_string();

        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();

        // Multi-line export should be hoisted completely
        let hoist_pos = result.code.find("export const foo = () => {").unwrap();
        assert!(
            hoist_pos < content_pos,
            "multi-line export should be hoisted before JSX content: {}",
            result.code
        );

        // The closing brace should also be hoisted (part of the same statement)
        assert!(
            result.code[..content_pos].contains("return 1"),
            "multi-line export body should be hoisted: {}",
            result.code
        );

        // Should not appear in JSX body
        assert!(
            !result.code[content_pos..].contains("return 1"),
            "multi-line export should not appear in JSX body: {}",
            result.code
        );

        // Re-export should also work
        let reexport_pos = result.code.find("export { foo };").unwrap();
        assert!(
            reexport_pos < content_pos,
            "re-export should be hoisted: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_exports_variants() {
        let config = InternalCompilerConfig::new(None);
        let source = "\nexport const foo = () => {\n  return 1\n}\n\nexport default function bar()\n{\n  return foo();\n}\n\nexport { foo };\n\n\n# Title"
            .to_string();

        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();

        // hoisted exports appear before JSX content
        let hoist_pos = result.code.find("export const foo").unwrap();
        assert!(
            hoist_pos < content_pos,
            "exports should be hoisted before JSX content: {}",
            result.code
        );
        assert_eq!(
            result.code.matches("export const foo").count(),
            1,
            "hoisted exports should not appear in JSX body: {}",
            result.code
        );

        // default export body hoisted, not in JSX
        let default_pos = result.code.find("export default function bar()").unwrap();
        assert!(
            default_pos < content_pos,
            "default export should be hoisted before JSX content: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_does_not_hoist_exports_inside_fence() {
        let config = InternalCompilerConfig::new(None);
        let source = "```\nexport const no = true\n```\n\nexport const yes = true;".to_string();
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();
        let fenced_pos = result.code.find("export const no = true").unwrap();

        // mdast currently only hoists LEADING imports/exports
        // exports that appear after content are not hoisted
        // TODO: Add full hoisting support similar to old multipass pipeline

        assert!(
            fenced_pos > content_pos,
            "fenced export should stay in JSX body: {}",
            result.code
        );
        assert!(
            result.code.contains("<pre><code") && result.code.contains("export const no = true"),
            "fenced export should stay in rendered JSX: {}",
            result.code
        );
    }

    #[test]
    fn compile_document_hoists_export_edge_cases() {
        let config = InternalCompilerConfig::new(None);
        let source = "\nexport default async () => {\n  return 1\n}\n\nexport * from './mod';\n\nexport const foo = 1 // inline\n\n\n# Title"
            .to_string();

        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
                .expect("compile success");
        let content_pos = result.code.find("function MarkflowContent").unwrap();
        assert!(
            result.code.find("export default async () => {").unwrap() < content_pos,
            "default async export should be hoisted"
        );
        assert!(
            result.code.find("export * from './mod';").unwrap() < content_pos,
            "export * should be hoisted"
        );
        assert!(
            result.code.find("export const foo = 1 // inline").unwrap() < content_pos,
            "inline comment export should be hoisted"
        );
        assert_eq!(
            result
                .code
                .matches("export const foo = 1 // inline")
                .count(),
            1,
            "inline export should not appear in JSX body: {}",
            result.code
        );
    }
}
