#![deny(missing_docs)]
//! Node.js bindings that surface Markflow's Rust implementation.

use markflow_core::event::{
    Event as CoreEvent, HeadingLevel, Tag as CoreTag, TagEnd as CoreTagEnd,
};
use markflow_core::{MarkflowError, RewriteOptions, extract_frontmatter};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

/// The stateful compiler and its configuration.
pub mod compiler;
/// NAPI-exposed data structures.
pub mod types;
pub use types::*;

const ASTRO_RENDER_HELPERS: &str = "astro/runtime/server/render/index.js";

/// Parses markdown string to HTML with default options
#[napi]
pub fn parse(input: String) -> napi::Result<String> {
    let result = markflow_core::parse(&input).map_err(convert_error)?;
    Ok(result.html)
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

/// Represents the type of the input file, either Markdown or MDX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Standard Markdown file.
    Markdown,
    /// MDX (Markdown with JSX) file.
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
            "  const html = await renderComponentToString(result, 'Layout', Layout, {{ ...props, frontmatter }}, {{")
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

/// Collects heading information from the raw markdown body.
///
/// This function parses the input `raw_body` and extracts all headings,
/// returning them as a vector of `HeadingEntry`. The `file_type`
/// parameter can be used to configure the parser for Markdown or MDX.
pub fn collect_headings(raw_body: &str, file_type: FileType) -> napi::Result<Vec<HeadingEntry>> {
    let mut collector = HeadingCollector::new();
    let config = match file_type {
        FileType::Markdown => markflow_core::ParseConfig::markdown(),
        FileType::Mdx => markflow_core::ParseConfig::mdx(),
    };
    let event_iterator =
        markflow_core::get_event_iterator_with_config(raw_body, config).map_err(convert_error)?;

    for event in event_iterator {
        collector.observe(&event);
    }

    Ok(collector.into_entries())
}

#[cfg(test)]
mod tests {
    use super::render_to_jsx_napi;
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
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
        let source = "\nexport const foo = () => {\n  return 1\n}\n\nexport default function bar()\n{\n  return foo();\n}\n\nexport { foo };\n\n\n# Title"
            .to_string();

        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
        let source = "\nexport default async () => {\n  return 1\n}\n\nexport * from './mod';\n\nexport const foo = 1 // inline\n\n\n# Title"
            .to_string();

        let result =
            crate::compiler::compile_document(&config, source, "test.mdx".into(), None, Vec::new())
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
