//! The stateful compiler and its configuration.

use crate::types::*;
use markflow_core::{MarkflowError, MdastOptions, RenderBlock, to_blocks};
use napi_derive::napi;
use std::path::Path;

const ASTRO_DEFAULT_RUNTIME: &str = "astro/runtime/server/index.js";

/// Counts unbalanced braces in a line, ignoring braces inside strings and comments.
/// Returns positive for excess `{`, negative for excess `}`.
fn count_braces(line: &str) -> i32 {
    let mut count = 0i32;
    let mut chars = line.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_template = false;
    let mut in_line_comment = false;

    while let Some(c) = chars.next() {
        // Skip line comments entirely
        if in_line_comment {
            continue;
        }

        // Check for line comment start
        if c == '/' && chars.peek() == Some(&'/') && !in_single_quote && !in_double_quote && !in_template {
            in_line_comment = true;
            chars.next(); // consume second /
            continue;
        }

        // Track string state
        match c {
            '\'' if !in_double_quote && !in_template => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote && !in_template => {
                in_double_quote = !in_double_quote;
            }
            '`' if !in_single_quote && !in_double_quote => {
                in_template = !in_template;
            }
            '\\' if in_single_quote || in_double_quote || in_template => {
                // Skip escaped character
                chars.next();
            }
            '{' if !in_single_quote && !in_double_quote && !in_template => {
                count += 1;
            }
            '}' if !in_single_quote && !in_double_quote && !in_template => {
                count -= 1;
            }
            _ => {}
        }
    }

    count
}

/// Extracts import/export statements from anywhere in the document.
/// MDX allows imports to be placed anywhere for readability, so we need
/// to hoist them all to the top of the generated module.
///
/// This function is careful to NOT extract imports from inside code fences,
/// which contain example code snippets.
///
/// Multi-line exports are tracked via brace counting to ensure complete
/// statements are hoisted (e.g., `export const fn = () => { ... }`).
fn extract_all_imports(input: &str) -> (Vec<String>, String) {
    let mut hoisted = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_code_fence = false;
    let mut pending_statement: Option<String> = None;
    let mut brace_depth = 0i32;

    for line in input.lines() {
        let trimmed = line.trim_start();

        // Track code fence state (``` or ~~~)
        // Opening fence: starts with ``` or ~~~ followed by optional info string
        // Closing fence: starts with ``` or ~~~ followed by ONLY whitespace
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fence_marker = if trimmed.starts_with("```") { "```" } else { "~~~" };
            let after_marker = &trimmed[fence_marker.len()..];

            if in_code_fence {
                // Inside a fence - only close if there's no info string (just whitespace)
                if after_marker.trim().is_empty() {
                    in_code_fence = false;
                }
                // Otherwise it's content inside the fence
            } else {
                // Outside a fence - this opens a new fence
                in_code_fence = true;
            }
            body_lines.push(line);
            continue;
        }

        // Inside code fence - always add to body
        if in_code_fence {
            body_lines.push(line);
            continue;
        }

        // Continue collecting multi-line statement
        if let Some(ref mut stmt) = pending_statement {
            stmt.push('\n');
            stmt.push_str(line);
            brace_depth += count_braces(line);

            if brace_depth <= 0 {
                // Statement complete
                hoisted.push(pending_statement.take().unwrap());
            }
            continue;
        }

        // Check for new import/export statement
        if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
            // Skip re-exporting default since we handle that separately
            if !trimmed.contains("export default") {
                brace_depth = count_braces(line);

                if brace_depth > 0 {
                    // Multi-line statement - start collecting
                    pending_statement = Some(line.to_string());
                } else {
                    // Single-line statement
                    hoisted.push(line.to_string());
                }
                continue;
            }
        }

        body_lines.push(line);
    }

    // Handle unclosed statement (edge case - treat as body)
    let unclosed_lines: Vec<&str>;
    if let Some(ref stmt) = pending_statement {
        unclosed_lines = stmt.lines().collect();
        for stmt_line in &unclosed_lines {
            body_lines.push(stmt_line);
        }
    }

    let body = body_lines.join("\n");
    (hoisted, body)
}

#[derive(Debug, Clone)]
pub(crate) struct InternalCompilerConfig {
    pub(crate) jsx_import_source: String,
}

impl InternalCompilerConfig {
    pub(crate) fn new(config: Option<CompilerConfig>) -> Self {
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
    pub(crate) config: InternalCompilerConfig,
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
    ///
    /// When parsing fails, this returns a fallback error page instead of an error,
    /// allowing the build to continue. The error is logged to stderr.
    #[napi(js_name = "compile")]
    pub fn compile_mdx(
        &self,
        source: String,
        filepath: String,
        options: Option<FileOptions>,
    ) -> napi::Result<CompileResult> {
        // Parse to IR first (framework-agnostic data).
        let ir_result = compile_ir(
            source.clone(),
            filepath.clone(),
            options.clone(),
            Some(CompilerConfig {
                jsx_import_source: Some(self.config.jsx_import_source.clone()),
                ..CompilerConfig::default()
            }),
        );

        match ir_result {
            Ok(ir) => compile_document_from_ir(ir),
            Err(err) => {
                // Log error but return fallback page
                eprintln!("[markflow] Parse error in {}: {}", filepath, err);
                Ok(generate_error_fallback(&filepath, &err.to_string()))
            }
        }
    }
}

/// Generates a fallback CompileResult that displays the parse error.
/// This allows the build to continue even when parsing fails.
fn generate_error_fallback(filepath: &str, error_message: &str) -> CompileResult {
    let escaped_error = error_message
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "");

    let escaped_filepath = filepath
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let code = format!(
        r##"import {{ Fragment as _Fragment, jsx as _jsx, jsxs as _jsxs }} from "astro/jsx-runtime";

export const frontmatter = {{}};
export function getHeadings() {{
  return [];
}}

export default function MarkflowError() {{
  return _jsxs("div", {{
    style: {{ border: "2px solid #dc2626", padding: "1.5rem", margin: "1rem", backgroundColor: "#fef2f2", borderRadius: "0.5rem" }},
    children: [
      _jsx("h3", {{
        style: {{ color: "#dc2626", marginTop: 0, marginBottom: "0.5rem" }},
        children: "Markflow Parse Error"
      }}),
      _jsx("p", {{
        style: {{ color: "#991b1b", fontSize: "0.875rem", marginBottom: "0.5rem" }},
        children: "{escaped_filepath}"
      }}),
      _jsx("pre", {{
        style: {{ whiteSpace: "pre-wrap", backgroundColor: "#fee2e2", padding: "1rem", borderRadius: "0.25rem", overflow: "auto", fontSize: "0.875rem", color: "#7f1d1d" }},
        children: "{escaped_error}"
      }})
    ]
  }});
}}
"##
    );

    CompileResult {
        code,
        map: None,
        frontmatter_json: "{}".to_string(),
        headings: vec![],
        imports: vec![],
        diagnostics: Diagnostics { warnings: vec![] },
    }
}

#[napi]
/// Helper factory exposed to JavaScript for ergonomic reuse.
pub fn create_compiler(config: Option<CompilerConfig>) -> MarkflowCompiler {
    MarkflowCompiler::new(config)
}

/// Converts mdast RenderBlocks to a JSX string format.
///
/// Html blocks are output directly. Component blocks are rendered as
/// JSX component syntax: <Name {...props}>{slotHtml}</Name>
fn blocks_to_jsx_string(blocks: &[RenderBlock]) -> String {
    let mut result = String::new();
    for block in blocks {
        match block {
            RenderBlock::Html { content } => {
                result.push_str(content);
            }
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                result.push('<');
                result.push_str(name);

                // Render props as {...{key: "value", ...}}
                if !props.is_empty() {
                    result.push_str(" {...{");
                    let mut first = true;
                    for (key, value) in props {
                        if !first {
                            result.push_str(", ");
                        }
                        first = false;
                        // Escape the key and value for JavaScript
                        result.push('"');
                        result.push_str(&key.replace('"', "\\\""));
                        result.push_str("\": \"");
                        result.push_str(&value.replace('"', "\\\"").replace('\n', "\\n"));
                        result.push('"');
                    }
                    result.push_str("}}");
                }

                result.push('>');
                result.push_str(slot_html);
                result.push_str("</");
                result.push_str(name);
                result.push('>');
            }
        }
    }
    result
}

/// Compiles Markdown/MDX and returns a neutral IR.
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

    let frontmatter_extraction = markflow_core::extract_frontmatter(&source)
        .map_err(|err| super::convert_error(MarkflowError::MarkdownAdapter(err.to_string())))?;
    let frontmatter = frontmatter_extraction.value;
    let raw_body = source[frontmatter_extraction.body_start..].to_string();

    // Extract import/export statements from anywhere in the document
    // MDX allows imports to be placed near where they're used for readability
    let (hoisted_imports, body_without_imports) = extract_all_imports(&raw_body);

    // Use mdast pipeline to generate blocks
    let mdast_options = MdastOptions {
        inject_starlight_css: false,
        enable_directives: true,
    };
    let blocks_result = to_blocks(&body_without_imports, &mdast_options).map_err(|err| {
        super::convert_error(with_path(
            MarkflowError::MarkdownAdapter(err),
            &effective_path,
        ))
    })?;

    // Convert blocks to JSX module string
    let jsx_body = blocks_to_jsx_string(&blocks_result.blocks);

    // Merge leading imports with any imports found in the JSX body
    let hoisted = hoisted_imports;
    let jsx = jsx_body;

    // mdast doesn't produce diagnostics yet - return empty warnings
    let diagnostics = Diagnostics { warnings: vec![] };

    // Use headings from mdast blocks_result
    let headings: Vec<_> = blocks_result
        .headings
        .into_iter()
        .map(|h| super::HeadingEntry {
            depth: h.depth,
            slug: h.slug,
            text: h.text,
        })
        .collect();
    let layout_import: Option<String> = frontmatter
        .get("layout")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let frontmatter_json = serde_json::to_string(&frontmatter).unwrap_or_else(|_| "{}".to_string());

    Ok(CompileIrResult {
        html: jsx,
        hoisted_imports: hoisted
            .into_iter()
            .map(|source| ImportSpec {
                source,
                kind: ImportKind::Hoisted,
            })
            .collect(),
        frontmatter_json,
        headings,
        file_path: effective_path,
        url: options.url.clone(),
        layout_import,
        runtime_import: internal.jsx_import_source,
        diagnostics,
    })
}

fn with_path(err: MarkflowError, path: &str) -> MarkflowError {
    match err {
        MarkflowError::MarkdownAdapter(msg) => {
            MarkflowError::MarkdownAdapter(format!("{msg} ({path})"))
        }
        other => other,
    }
}

pub(crate) fn compile_document_from_ir(ir: CompileIrResult) -> napi::Result<CompileResult> {
    let hoisted_imports = super::dedupe_imports(
        ir.hoisted_imports
            .iter()
            .map(|spec| spec.source.clone())
            .collect(),
    );
    let headings_json = serde_json::to_string(&ir.headings).unwrap_or_else(|_| "[]".to_string());
    let code = super::codegen::generate_module_code_from_ir(&ir, &hoisted_imports, &headings_json)?;
    let imports = super::build_import_list(ir.layout_import.as_deref(), Path::new(&ir.file_path));

    Ok(CompileResult {
        code,
        map: None,
        frontmatter_json: ir.frontmatter_json,
        headings: ir.headings,
        imports,
        diagnostics: ir.diagnostics,
    })
}

#[cfg(test)]
pub(crate) fn compile_document(
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
