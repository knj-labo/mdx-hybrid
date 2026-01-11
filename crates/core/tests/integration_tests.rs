use markflow_core::{
    ComponentRegistry, JsxComponentPlugin, JsxElement, JsxOptions, MarkflowError, RenderContext,
    RenderOutcome, RewriteOptions, parse, parse_with_options, render_to_jsx,
    render_to_jsx_with_options,
};
use std::fs;
use std::path::PathBuf;

// This helper was in the original lib.rs test module.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/core")
}

// This helper was also in the original lib.rs test module.
fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixtures_dir().join(path)).unwrap()
}

struct BadgePlugin;

impl JsxComponentPlugin for BadgePlugin {
    fn matches(&self, name: &str) -> bool {
        name == "Badge"
    }

    fn render_children(
        &self,
        element: &JsxElement<'_>,
        ctx: &RenderContext<'_>,
        output: &mut String,
    ) -> Result<RenderOutcome, MarkflowError> {
        output.push_str("<span class=\"badge\">");
        ctx.render_children(element.children, output)?;
        output.push_str("</span>");
        Ok(RenderOutcome::Handled)
    }
}

#[test]
fn jsx_renderer_preserves_raw_jsx() {
    let input = "import X from './x'\n\n<MyComponent />\n";
    let result = render_to_jsx(input).expect("render_to_jsx succeeds");
    assert!(result.starts_with("import X from './x'"));
    assert!(result.contains("<MyComponent />"));
}

#[test]
fn jsx_registry_allows_imports_and_plugins() {
    let input = "<Badge>Hi</Badge>";
    let mut components = ComponentRegistry::new();
    components.register_import("Badge", "import Badge from './Badge.astro';");
    components.register_plugin(BadgePlugin);
    let options = JsxOptions {
        components,
        ..JsxOptions::default()
    };
    let output =
        render_to_jsx_with_options(input, options).expect("render_to_jsx_with_options succeeds");

    assert!(
        output.starts_with("import Badge from './Badge.astro';\n"),
        "import should be hoisted before the body. output: {output}"
    );
    assert!(
        output.contains("<span class=\"badge\"><p>Hi</p>"),
        "plugin output should wrap children. output: {output}"
    );
    assert!(
        output.contains("</span></Badge>"),
        "plugin output should remain inside the component. output: {output}"
    );
}

#[test]
fn jsx_steps_nested_tabs_stays_in_list_item() {
    let input =
        "<Steps>\n1. First\n   <Tabs>\n     <Tab title=\"A\">A</Tab>\n   </Tabs>\n</Steps>\n";
    let output = render_to_jsx(input).expect("render_to_jsx succeeds");
    let li_start = output.find("<li").expect("li should exist");
    let li_close = output[li_start..]
        .find("</li>")
        .map(|idx| li_start + idx)
        .expect("li should close");
    let tabs_pos = output.find("<Tabs>").expect("tabs should exist");
    assert!(
        tabs_pos > li_start && tabs_pos < li_close,
        "Tabs should be nested inside the list item. output: {output}"
    );
}

#[test]
fn jsx_inline_code_preserves_paragraph() {
    let input = "Click `button` now";
    let output = render_to_jsx(input).expect("render_to_jsx succeeds");
    assert!(
        output.contains("<p>Click <code>button</code> now</p>"),
        "inline code should stay inside a single paragraph. output: {output}"
    );
    assert!(
        !output.contains("</p><code>button</code><p>"),
        "inline code should not split paragraphs. output: {output}"
    );
}

#[test]
fn test_parse() {
    let input = "# Hello, World!";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<h1 id=\"hello-world\">Hello, World!</h1>"));
}

#[test]
fn test_parse_list() {
    let input = "* Item 1\n* Item 2";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<ul>"));
    assert!(output.contains("<li>Item 1</li>"));
}

#[test]
fn test_parse_applies_lazy_loading() {
    let input = "![alt](img.png)";
    let output = parse(input).unwrap().html;
    assert!(output.contains("loading=\"lazy\""));
}

#[test]
fn test_parse_without_hoist_keeps_imports_in_html() {
    let input = "import X from './x'\n\n# Title";
    let opts = RewriteOptions {
        enable_hoist: false,
        enable_smartypants: false,
        ..RewriteOptions::default()
    };
    let result = parse_with_options(input, opts).expect("parse succeeds");

    assert!(
        result.html.contains("import X from './x'"),
        "import should remain in HTML when hoist is disabled"
    );
    assert!(
        result.imports.is_empty(),
        "hoisted import list should be empty when hoist is disabled"
    );
}

#[test]
fn test_parse_table_alignment_and_math() {
    let input = "| A | B |\n|:-|:-:|\n| $x$ | $$y$$ |";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<table>"));
    assert!(
        output.contains("<td style=\"text-align:left\"><span class=\"math-inline\">x</span></td>")
    );
    assert!(output.contains("<span class=\"math-inline\">y</span>"));
}

#[test]
fn test_parse_frontmatter_passthrough() {
    let input = "---\ntitle: test\n---\n\ncontent";
    let output = parse(input).unwrap().html;
    assert!(output.contains("frontmatter"));
    assert!(output.contains("title: test"));
}

#[test]
fn test_reference_link_resolves_definition() {
    let input = "[Example][ref]\n\n[ref]: https://example.com \"Example Site\"";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<a href=\"https://example.com\""));
    assert!(output.contains("title=\"Example Site\""));
}

/*
// This test calls a private function `apply_smartypants`, which is not public from an integration test.
// This should be a unit test in the `transform/smartypants.rs` module.
#[test]
fn test_smartypants_enabled() {
    let input = "Hello -- \"world\" ...";
    let out = apply_smartypants(input);
    assert_eq!(out, "Hello – “world” …");
}
*/

#[test]
fn test_smartypants_disabled() {
    let input = "Hello -- \"world\" ...";
    let opts = RewriteOptions {
        enable_smartypants: false,
        ..RewriteOptions::default()
    };
    let output = parse_with_options(input, opts).unwrap().html;
    assert!(
        output.contains("Hello -- &quot;world&quot; ...")
            || output.contains("Hello -- \"world\" ...")
    );
}

#[test]
fn test_directive_rewrites_to_aside() {
    let input = ":::note[Heads up]\ncontent\n:::";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<aside class=\"aside aside--note\">"));
    assert!(output.contains("Heads up"));
    assert!(output.contains("</aside>"));
}

#[test]
fn test_reference_image_resolves_definition() {
    let input = "![Alt][logo]\n\n[logo]: https://cdn.example.com/logo.png \"Logo\"";
    let output = parse(input).unwrap().html;
    assert!(output.contains("<img src=\"https://cdn.example.com/logo.png\""));
    assert!(output.contains("title=\"Logo\""));
}

#[test]
fn test_mdx_embedded_jsx_preserved() {
    let input = read_fixture("mdx/embedded-jsx/component.mdx");
    let output = parse(&input).unwrap().html;
    assert!(output.contains("<aside class=\"aside\">"));
    assert!(output.contains("Heads up"));
    assert!(output.contains("</aside>"));
}

#[test]
fn test_mdx_inline_expression_preserved() {
    let input = read_fixture("mdx/expressions/inline.mdx");
    let output = parse(&input).unwrap().html;
    assert!(
        output.contains("{props.name ?? 'friend'}"),
        "output: {output}"
    );
}

#[test]
fn test_mdx_flow_expression_preserved() {
    let input = read_fixture("mdx/expressions/flow.mdx");
    let output = parse(&input).unwrap().html;
    assert!(output.contains("steps.join(' → ');"), "output: {output}");
}

#[test]
fn test_mdx_esm_import_preserved() {
    let input = read_fixture("mdx/esm/imports.mdx");
    let output = parse(&input).unwrap();
    assert!(
        !output.html.contains("import Tabs from"),
        "root-level imports should be hoisted away from HTML output"
    );
    assert!(
        output
            .imports
            .iter()
            .any(|spec| spec.source.contains("import Tabs from")),
        "imports should be hoisted into the imports list"
    );
}
