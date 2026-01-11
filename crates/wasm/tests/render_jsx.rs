use markflow_wasm::render_jsx_with_options_wasm;
use serde::Serialize;
use wasm_bindgen_test::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestJsxComponentImport {
    name: &'static str,
    import: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestJsxRenderOptions {
    component_imports: Vec<TestJsxComponentImport>,
}

#[wasm_bindgen_test]
fn render_jsx_with_options_hoists_imports() {
    let input = "<Badge>Hi</Badge>";
    let options = TestJsxRenderOptions {
        component_imports: vec![TestJsxComponentImport {
            name: "Badge",
            import: "import Badge from './Badge.astro';",
        }],
    };
    let opts = serde_wasm_bindgen::to_value(&options).expect("serialize options");
    let output =
        render_jsx_with_options_wasm(input, opts).expect("render_jsx_with_options");
    assert!(output.starts_with("import Badge from './Badge.astro';"));
    assert!(output.contains("<Badge>"));
}
