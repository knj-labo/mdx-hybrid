//! The stateful compiler and its configuration.

use crate::types::*;
use markflow_core::{MarkflowError, MdastOptions, RenderBlock, code_fence, to_blocks};
use napi_derive::napi;
use std::path::Path;

const ASTRO_DEFAULT_RUNTIME: &str = "astro/runtime/server/index.js";

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

    // Extract all imports/exports from the document (not just leading ones)
    // Uses code fence tracking to avoid extracting imports inside code blocks
    let (hoisted_statements, body_lines) = code_fence::collect_root_imports(&raw_body);
    let body_without_imports = body_lines.join("\n");
    let has_user_default_export = hoisted_statements
        .iter()
        .any(|s| s.trim_start().starts_with("export default"));
    let leading_imports = hoisted_statements;

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
    let hoisted = leading_imports;
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
        has_user_default_export,
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
