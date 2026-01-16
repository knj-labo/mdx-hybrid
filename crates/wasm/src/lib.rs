use js_sys::Function;
use markflow_core::MdastOptions;
use markflow_core::code_fence::collect_root_imports;
use markflow_core::codegen::{AstroModuleOptions, DirectiveMappingResult, blocks_to_jsx_string};
use markflow_core::renderer::mdast::to_blocks;
use markflow_core::{MarkdownStream, RewriteOptions, StreamingRewriter, get_event_iterator};
use serde::Serialize;
use std::io::{self, Write};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// ============================================================================
// Compile API Types
// ============================================================================

/// Heading metadata extracted from the document.
#[derive(Debug, Clone, Serialize)]
pub struct HeadingEntry {
    /// Heading depth (1-6).
    pub depth: u8,
    /// Slugified identifier.
    pub slug: String,
    /// Visible heading text.
    pub text: String,
}

/// Result of compiling MDX to an Astro-compatible module.
#[derive(Debug, Clone, Serialize)]
pub struct CompileResult {
    /// Generated JavaScript/JSX module code.
    pub code: String,
    /// Serialized frontmatter as JSON string.
    pub frontmatter_json: String,
    /// Extracted heading metadata.
    pub headings: Vec<HeadingEntry>,
    /// Whether the user provided their own export default.
    pub has_user_default_export: bool,
}

// ============================================================================
// Compile API
// ============================================================================

/// Compiles MDX source into an Astro-compatible JavaScript module.
///
/// This function extracts frontmatter, hoists imports/exports, parses markdown
/// to JSX, and generates a complete module with createComponent wrapper.
///
/// # Arguments
///
/// * `source` - The MDX source code
/// * `filepath` - File path for error messages and `export const file`
///
/// # Returns
///
/// Returns a JavaScript object with:
/// - `code`: The generated module code
/// - `frontmatter_json`: Serialized frontmatter
/// - `headings`: Array of heading metadata
/// - `has_user_default_export`: Whether user provided export default
#[wasm_bindgen]
pub fn compile(source: &str, filepath: &str) -> Result<JsValue, JsError> {
    // 1. Extract frontmatter
    let frontmatter_extraction = markflow_core::extract_frontmatter(source)
        .map_err(|e| JsError::new(&format!("Frontmatter error: {}", e)))?;
    let frontmatter = frontmatter_extraction.value;
    let raw_body = &source[frontmatter_extraction.body_start..];

    // 2. Extract imports/exports from entire document
    let (hoisted_imports, body_lines) = collect_root_imports(raw_body);
    let body_without_imports = body_lines.join("\n");
    let has_user_default_export = hoisted_imports
        .iter()
        .any(|s| s.trim_start().starts_with("export default"));

    // 3. Parse to blocks and render JSX
    let mdast_options = MdastOptions {
        enable_directives: true,
    };
    let blocks_result = to_blocks(&body_without_imports, &mdast_options)
        .map_err(|e| JsError::new(&format!("Parse error: {}", e)))?;

    let jsx_body = blocks_to_jsx_string(
        &blocks_result.blocks,
        None::<fn(&str) -> Option<DirectiveMappingResult>>,
    );

    // 4. Convert headings
    let headings: Vec<HeadingEntry> = blocks_result
        .headings
        .into_iter()
        .map(|h| HeadingEntry {
            depth: h.depth,
            slug: h.slug,
            text: h.text,
        })
        .collect();

    // 5. Serialize data
    let frontmatter_json = serde_json::to_string(&frontmatter).unwrap_or_else(|_| "{}".to_string());
    let headings_json = serde_json::to_string(&headings).unwrap_or_else(|_| "[]".to_string());

    // 6. Generate module code
    let code = markflow_core::codegen::generate_astro_module(&AstroModuleOptions {
        jsx: &jsx_body,
        hoisted_imports: &hoisted_imports,
        frontmatter_json: &frontmatter_json,
        headings_json: &headings_json,
        filepath,
        url: None,
        layout_import: None,
        has_user_default_export,
    });

    // 7. Build result
    let result = CompileResult {
        code,
        frontmatter_json,
        headings,
        has_user_default_export,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Serialization error: {}", e)))
}

// ============================================================================
// Existing Render API
// ============================================================================

/// Renders markdown into an HTML `String`.
#[wasm_bindgen(js_name = render_html)]
pub fn render_html(
    input: &str,
    enforce_img_loading_lazy: Option<bool>,
    enable_directives: Option<bool>,
    enable_hoist: Option<bool>,
    enable_smartypants: Option<bool>,
    enable_components: Option<bool>,
) -> Result<String, JsError> {
    let enable_directives = enable_directives.unwrap_or(true);
    let enable_hoist = enable_hoist.unwrap_or(true);
    let enable_smartypants = enable_smartypants.unwrap_or(true);
    let enable_components = enable_components.unwrap_or(true);

    let body = if enable_hoist {
        let (_, body_lines) = collect_root_imports(input);
        body_lines.join("\n")
    } else {
        input.to_string()
    };

    let events = get_event_iterator(&body).map_err(to_js_error)?;
    let options = RewriteOptions {
        enforce_img_loading_lazy: enforce_img_loading_lazy.unwrap_or(true),
        enable_directives,
        enable_hoist,
        enable_smartypants,
        enable_components,
        ..RewriteOptions::default()
    };
    let rewriter = StreamingRewriter::new(Vec::new(), options);
    let rewriter = events
        .stream_to_writer(rewriter)
        .map_err(|err| JsError::new(&err.to_string()))?;
    let output = rewriter
        .into_inner()
        .map_err(|err| JsError::new(&err.to_string()))?;
    String::from_utf8(output).map_err(to_js_error)
}

/// Streams rendered HTML chunks into the provided JavaScript callback.
///
/// The callback is invoked with each UTF-8 chunk produced by the streaming
/// renderer, so callers can forward output to a `WritableStream`, append to the
/// DOM incrementally, or buffer it manually.
#[wasm_bindgen(js_name = stream_html)]
pub fn stream_html(
    input: &str,
    chunk_callback: &Function,
    enforce_img_loading_lazy: Option<bool>,
    enable_directives: Option<bool>,
    enable_hoist: Option<bool>,
    enable_smartypants: Option<bool>,
    enable_components: Option<bool>,
) -> Result<(), JsError> {
    let enable_directives = enable_directives.unwrap_or(true);
    let enable_hoist = enable_hoist.unwrap_or(true);
    let enable_smartypants = enable_smartypants.unwrap_or(true);
    let enable_components = enable_components.unwrap_or(true);

    let options = RewriteOptions {
        enforce_img_loading_lazy: enforce_img_loading_lazy.unwrap_or(true),
        enable_directives,
        enable_hoist,
        enable_smartypants,
        enable_components,
        ..RewriteOptions::default()
    };

    let body = if enable_hoist {
        let (_, body_lines) = collect_root_imports(input);
        body_lines.join("\n")
    } else {
        input.to_string()
    };
    let events = get_event_iterator(&body).map_err(to_js_error)?;
    let writer = JsChunkWriter::new(chunk_callback.clone());
    let rewriter = StreamingRewriter::new(writer, options);

    let rewriter = events
        .stream_to_writer(rewriter)
        .map_err(|err| JsError::new(&err.to_string()))?;

    rewriter
        .into_inner()
        .map_err(|err| JsError::new(&err.to_string()))?;
    Ok(())
}

fn to_js_error<E: ToString>(err: E) -> JsError {
    JsError::new(&err.to_string())
}

struct JsChunkWriter {
    callback: Function,
}

impl JsChunkWriter {
    fn new(callback: Function) -> Self {
        Self { callback }
    }
}

impl Write for JsChunkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let chunk = std::str::from_utf8(buf)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        self.callback
            .call1(&JsValue::UNDEFINED, &JsValue::from_str(chunk))
            .map_err(js_callback_error)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn js_callback_error(err: JsValue) -> io::Error {
    let message = err
        .as_string()
        .or_else(|| {
            js_sys::JSON::stringify(&err)
                .ok()
                .and_then(|s| s.as_string())
        })
        .unwrap_or_else(|| "callback threw".to_string());
    io::Error::other(message)
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
/// * `opts` - Optional JavaScript object with options:
///   - `enable_directives`: boolean (default: true)
///
/// # Returns
///
/// Returns a JavaScript array of RenderBlock objects. Each block is either:
/// - `{type: "html", content: "<p>...</p>"}` - Plain HTML content
/// - `{type: "component", name: "note", props: {title: "..."}, slot_html: "..."}` - Component block
///
/// # Example (JavaScript)
///
/// ```javascript
/// import { parse_blocks } from './markflow_wasm';
///
/// const input = `:::note[Important]
/// This is **bold** text.
/// :::`;
///
/// const blocks = parse_blocks(input, { enable_directives: true });
/// // blocks = [
/// //   {
/// //     type: "component",
/// //     name: "note",
/// //     props: { title: "Important" },
/// //     slot_html: "<p>This is <strong>bold</strong> text.</p>"
/// //   }
/// // ]
/// ```
#[wasm_bindgen(js_name = parse_blocks)]
pub fn parse_blocks(input: &str, opts: JsValue) -> Result<JsValue, JsError> {
    use markflow_core::renderer::mdast::{Options, to_blocks};

    // Parse options from JavaScript
    let options: Options = if opts.is_undefined() || opts.is_null() {
        Options {
            enable_directives: true,
        }
    } else {
        serde_wasm_bindgen::from_value(opts)
            .map_err(|e| JsError::new(&format!("Invalid options: {}", e)))?
    };

    // Parse markdown to blocks
    let blocks = to_blocks(input, &options).map_err(|e| JsError::new(&e))?;

    // Convert to JavaScript value using zero-copy serialization
    serde_wasm_bindgen::to_value(&blocks)
        .map_err(|e| JsError::new(&format!("Serialization error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_html_hoists_imports() {
        let input = "import X from './x';\n\n# Hi";
        let html = render_html(input, None, None, None, None, None).expect("render_html success");
        assert!(
            !html.contains("import X"),
            "import should not appear in rendered HTML"
        );
        assert!(html.contains("<h1 id=\"hi\">Hi</h1>"));
    }

    #[test]
    fn render_html_without_hoist_keeps_imports() {
        let input = "import X from './x';\n\n# Hi";
        let html =
            render_html(input, None, None, Some(false), None, None).expect("render_html success");
        assert!(
            html.contains("import X"),
            "import should remain when hoist is disabled"
        );
    }

    // Note: WASM-specific tests are in tests/ directory using wasm-bindgen-test
    // Regular cargo test cannot run wasm-bindgen functions
}
