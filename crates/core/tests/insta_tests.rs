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
fn markdown_basic_snapshot() {
    snapshot_from("markdown_basic", "# Hello, World!");
}

#[test]
fn directive_note_snapshot() {
    snapshot_from("directive_note", ":::note\nBody\n:::");
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
