use std::cell::RefCell;
use std::rc::Rc;

use markflow_wasm::{render_html, stream_html};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

fn collect_stream(
    input: &str,
    enforce_lazy: Option<bool>,
    enable_directives: Option<bool>,
    enable_hoist: Option<bool>,
    enable_smartypants: Option<bool>,
    enable_components: Option<bool>,
) -> Vec<String> {
    let chunks = Rc::new(RefCell::new(Vec::<String>::new()));
    let target = chunks.clone();
    let callback = Closure::wrap(Box::new(move |chunk: JsValue| {
        target
            .borrow_mut()
            .push(chunk.as_string().expect("chunk string"));
    }) as Box<dyn FnMut(JsValue)>);

    stream_html(
        input,
        callback.as_ref().unchecked_ref(),
        enforce_lazy,
        enable_directives,
        enable_hoist,
        enable_smartypants,
        enable_components,
    )
    .expect("stream html");
    drop(callback);

    Rc::try_unwrap(chunks).unwrap().into_inner()
}

#[wasm_bindgen_test]
fn stream_matches_render_html() {
    let input = "# Hello\\n\\n**bold** _italic_";
    let expected = render_html(input, None, None, None, None, None).expect("render html");

    let chunks = collect_stream(input, None, None, None, None, None);

    assert!(!chunks.is_empty(), "stream should emit at least one chunk");
    assert_eq!(chunks.join(""), expected);
}

#[wasm_bindgen_test]
fn stream_respects_lazy_option() {
    let input = "<img src=\"/hero.png\">";
    let default_output = collect_stream(input, None, None, None, None, None).join("");
    assert!(default_output.contains("loading=\"lazy\""));

    let disabled_output = collect_stream(input, Some(false), None, None, None, None).join("");
    assert!(!disabled_output.contains("loading=\"lazy\""));
}
