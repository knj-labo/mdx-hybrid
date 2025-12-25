//! The stateful compiler and its configuration.

use crate::types::*;
use markflow_core::{MarkflowError, RewriteOptions, render_to_jsx};
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
    let file_type = options
        .file_type
        .map(super::FileType::from)
        .unwrap_or_else(|| super::FileType::from_path(Path::new(&effective_path)));

    let frontmatter_extraction = markflow_core::extract_frontmatter(&source)
        .map_err(|err| super::convert_error(MarkflowError::MarkdownAdapter(err.to_string())))?;
    let frontmatter = frontmatter_extraction.value;
    let raw_body = source[frontmatter_extraction.body_start..].to_string();

    let parse_result = markflow_core::parse_with_options(&raw_body, RewriteOptions::default())
        .map_err(super::convert_error)?;
    let mut jsx = render_to_jsx(&raw_body).map_err(super::convert_error)?;
    let import_prefix_len: usize = parse_result
        .imports
        .iter()
        .map(|spec| spec.source.len() + 1)
        .sum();
    if import_prefix_len > 0 && jsx.len() >= import_prefix_len {
        jsx = jsx[import_prefix_len..].to_string();
    }

    let headings = super::collect_headings(&raw_body, file_type)?;
    let layout_import: Option<String> = frontmatter
        .get("layout")
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let frontmatter_json = serde_json::to_string(&frontmatter).unwrap_or_else(|_| "{}".to_string());

    Ok(CompileIrResult {
        html: jsx,
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
