use js_sys::Function;
use markflow_core::{MarkdownStream, RewriteOptions, StreamingRewriter, get_event_iterator};
use std::io::{self, Write};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Renders markdown into an HTML `String`.
#[wasm_bindgen(js_name = render_html)]
pub fn render_html(input: &str) -> Result<String, JsError> {
    markflow_core::parse(input).map_err(to_js_error)
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
) -> Result<(), JsError> {
    let options = RewriteOptions {
        enforce_img_loading_lazy: enforce_img_loading_lazy.unwrap_or(true),
    };

    let events = get_event_iterator(input).map_err(to_js_error)?;
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
