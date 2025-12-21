use once_cell::sync::Lazy;

static INIT: Lazy<()> = Lazy::new(|| {
    // placeholder for future shared setup
});

fn render(input: &str, enable_components: bool) -> String {
    Lazy::force(&INIT);
    let options = markflow_core::RewriteOptions {
        enable_components,
        ..markflow_core::RewriteOptions::default()
    };
    markflow_core::parse_with_options(input, options)
        .expect("parse succeeds")
        .html
}

fn snapshot(name: &str, input: &str, enable_components: bool) {
    let output = render(input, enable_components);
    insta::assert_snapshot!(name, output);
}

#[test]
fn tabs_snapshot_enabled() {
    snapshot(
        "tabs_enabled",
        include_str!("../../../fixtures/core/components/tabs.md"),
        true,
    );
}

#[test]
fn tabs_snapshot_disabled() {
    snapshot(
        "tabs_disabled",
        include_str!("../../../fixtures/core/components/tabs.md"),
        false,
    );
}

#[test]
fn steps_snapshot_enabled() {
    snapshot(
        "steps_enabled",
        include_str!("../../../fixtures/core/components/steps.md"),
        true,
    );
}

#[test]
fn steps_snapshot_disabled() {
    snapshot(
        "steps_disabled",
        include_str!("../../../fixtures/core/components/steps.md"),
        false,
    );
}

#[test]
fn filetree_snapshot_enabled() {
    snapshot(
        "filetree_enabled",
        include_str!("../../../fixtures/core/components/filetree.md"),
        true,
    );
}

#[test]
fn filetree_snapshot_disabled() {
    snapshot(
        "filetree_disabled",
        include_str!("../../../fixtures/core/components/filetree.md"),
        false,
    );
}

#[test]
fn aside_directive_snapshot_enabled() {
    snapshot(
        "aside_directive_enabled",
        include_str!("../../../fixtures/core/components/aside_directive.md"),
        true,
    );
}

#[test]
fn aside_directive_snapshot_disabled() {
    snapshot(
        "aside_directive_disabled",
        include_str!("../../../fixtures/core/components/aside_directive.md"),
        false,
    );
}

#[test]
fn aside_mdx_snapshot_enabled() {
    snapshot(
        "aside_mdx_enabled",
        include_str!("../../../fixtures/core/components/aside_mdx.md"),
        true,
    );
}

#[test]
fn aside_mdx_snapshot_disabled() {
    snapshot(
        "aside_mdx_disabled",
        include_str!("../../../fixtures/core/components/aside_mdx.md"),
        false,
    );
}
