use once_cell::sync::Lazy;

static INIT: Lazy<()> = Lazy::new(|| {
    // any one-time setup can go here later
});

fn snapshot_from(source: &str, input: &str) {
    Lazy::force(&INIT);
    let result = markflow_core::parse(input).expect("parse").html;
    insta::assert_snapshot!(source, result);
}

#[test]
fn directive_inline_code_snapshot() {
    snapshot_from("directive_inline_code", "Here is `:::note` inside code");
}

#[test]
fn directive_html_attr_snapshot() {
    snapshot_from("directive_html_attr", "<div title=\":note\">content</div>");
}

#[test]
fn mdx_import_hoist_snapshot() {
    let input = "import X from './x'\n\n# Title";
    snapshot_from("mdx_import_hoist", input);
}

#[test]
fn mdx_struct_simple_snapshot() {
    let input = include_str!("../../../fixtures/core/mdx/mdx_struct_simple.mdx");
    snapshot_from("mdx_struct_simple", input);
}

#[test]
fn mdx_struct_nested_snapshot() {
    let input = include_str!("../../../fixtures/core/mdx/mdx_struct_nested.mdx");
    snapshot_from("mdx_struct_nested", input);
}

#[test]
fn mdx_struct_expr_snapshot() {
    let input = include_str!("../../../fixtures/core/mdx/mdx_struct_expr.mdx");
    snapshot_from("mdx_struct_expr", input);
}

#[test]
fn slug_duplicates_snapshot() {
    let input = include_str!("../../../fixtures/core/markdown/slug_duplicates.md");
    snapshot_from("slug_duplicates", input);
}
