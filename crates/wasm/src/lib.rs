use js_sys::Function;
use markflow_core::code_fence::collect_root_imports;
use markflow_core::{
    MarkdownStream, RewriteOptions, StreamingRewriter, get_event_iterator, render_to_jsx,
};
use std::io::{self, Write};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

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

/// Renders markdown/MDX to JSX while preserving raw JSX nodes.
#[wasm_bindgen(js_name = render_jsx)]
pub fn render_jsx(
    input: &str,
    enable_directives: Option<bool>,
    enable_hoist: Option<bool>,
    enable_smartypants: Option<bool>,
    enable_components: Option<bool>,
) -> Result<String, JsError> {
    let enable_directives = enable_directives.unwrap_or(true);
    let enable_hoist = enable_hoist.unwrap_or(true);
    let enable_smartypants = enable_smartypants.unwrap_or(true);
    let enable_components = enable_components.unwrap_or(true);

    if enable_directives && enable_hoist {
        // Preserve original behavior (imports + JSX preserved).
        return render_to_jsx(input).map_err(to_js_error);
    }

    let options = RewriteOptions {
        enable_directives,
        enable_hoist,
        enable_smartypants,
        enable_components,
        ..RewriteOptions::default()
    };

    let parse_result = markflow_core::parse_with_options(input, options).map_err(to_js_error)?;
    Ok(parse_result.html)
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
///   - `inject_starlight_css`: boolean (default: false)
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
    use markflow_core::renderer::mdast::{to_blocks, Options};

    // Parse options from JavaScript
    let options: Options = if opts.is_undefined() || opts.is_null() {
        Options {
            inject_starlight_css: false,
            enable_directives: true,
        }
    } else {
        serde_wasm_bindgen::from_value(opts)
            .map_err(|e| JsError::new(&format!("Invalid options: {}", e)))?
    };

    // Parse markdown to blocks
    let blocks = to_blocks(input, &options)
        .map_err(|e| JsError::new(&e))?;

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
    fn render_jsx_preserves_raw_jsx() {
        let input = "import X from './x'\n\n<Component />\n";
        let jsx = render_jsx(input, None, None, None, None).expect("render_jsx success");
        assert!(jsx.starts_with("import X from './x'"));
        assert!(jsx.contains("<Component />"));
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
