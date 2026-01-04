//! The stateful compiler and its configuration.

use crate::types::*;
use markflow_core::{MarkflowError, render_to_html_mdast, render_to_jsx};
use napi_derive::napi;
use std::path::Path;

const ASTRO_DEFAULT_RUNTIME: &str = "astro/runtime/server/index.js";

fn split_leading_imports(input: &str) -> (Vec<String>, String) {
    let mut hoisted = Vec::new();
    let mut lines = Vec::new();
    let mut collecting = true;

    for line in input.lines() {
        if collecting {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                lines.push(line);
                continue;
            }
            if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                hoisted.push(line.to_string());
                continue;
            }
            collecting = false;
        }
        lines.push(line);
    }

    let body = lines.join("\n");
    (hoisted, body)
}

#[derive(Debug, Clone)]
pub(crate) struct InternalCompilerConfig {
    pub(crate) jsx_import_source: String,
    pub(crate) pipeline: Pipeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pipeline {
    Multipass,
    Mdast,
}

impl InternalCompilerConfig {
    pub(crate) fn new(config: Option<CompilerConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        let jsx_import_source = cfg
            .jsx_import_source
            .unwrap_or_else(|| ASTRO_DEFAULT_RUNTIME.to_string());
        let pipeline = match std::env::var("MARKFLOW_PIPELINE").ok().as_deref() {
            Some("mdast") => Pipeline::Mdast,
            Some("multipass") => Pipeline::Multipass,
            _ => match cfg.pipeline.as_deref() {
                Some("mdast") => Pipeline::Mdast,
                _ => Pipeline::Multipass,
            },
        };

        Self {
            jsx_import_source,
            pipeline,
        }
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

    let jsx_full = match internal.pipeline {
        Pipeline::Multipass => render_to_jsx(&raw_body)
            .map_err(|err| super::convert_error(with_path(err, &effective_path)))?,
        Pipeline::Mdast => match render_to_html_mdast(&raw_body) {
            Ok(value) => value,
            Err(err) => {
                let _ = err;
                let fallback = render_to_jsx(&raw_body)
                    .map_err(|err| super::convert_error(with_path(err, &effective_path)))?;
                fallback
            }
        },
    };
    let (hoisted, jsx) = split_leading_imports(&jsx_full);

    let headings = match super::collect_headings(&raw_body, file_type, &effective_path) {
        Ok(value) => value,
        Err(_) => Vec::new(),
    };
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
