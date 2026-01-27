//! JSX indentation normalization utilities.

/// Information about a parsed JSX tag.
#[derive(Debug, Clone)]
struct JsxTagInfo {
    /// The tag name (e.g., "MyComponent", "div", "Fragment")
    name: String,
    /// Whether this is a self-closing tag (ends with `/>`)
    self_closing: bool,
    /// Whether this tag has a `slot=` attribute
    has_slot_attr: bool,
}

/// Parses a JSX tag from a trimmed line.
/// Returns None for non-JSX content, closing tags, or comments.
fn parse_jsx_tag(trimmed: &str) -> Option<JsxTagInfo> {
    // Must start with '<' but not '</' (closing) or '<!' (comment)
    if !trimmed.starts_with('<') || trimmed.starts_with("</") || trimmed.starts_with("<!") {
        return None;
    }

    let rest = &trimmed[1..];
    let name_end = rest
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(rest.len());
    let name = &rest[..name_end];

    if name.is_empty() {
        return None;
    }

    let self_closing = trimmed.trim_end().ends_with("/>");
    let has_slot_attr = trimmed.contains("slot=");

    Some(JsxTagInfo {
        name: name.to_string(),
        self_closing,
        has_slot_attr,
    })
}

/// Normalizes JSX indentation and spacing.
///
/// 1. Inserts a blank line before any block-level JSX component (Capitalized tag)
///    that follows non-blank content. This prevents "Tag mismatch" errors where
///    components get trapped inside Markdown paragraphs.
///    e.g. "Text\n<Component>" -> "Text\n\n<Component>"
///
/// 2. Preserves indentation inside JSX blocks/fences as much as possible.
pub fn normalize_mdx_jsx_indentation(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_fence = false;
    let mut fence_marker: Option<char> = None;

    // Simple bracket counting to skip logic inside nested structures if needed,
    // but for now strictly generic line-based processing.
    let mut last_line_was_blank = true;

    for line in input.split_inclusive('\n') {
        let (raw_body, line_ending) = if let Some(stripped) = line.strip_suffix('\n') {
            let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
            (stripped, &line[stripped.len()..])
        } else {
            (line, "")
        };

        let line_body = raw_body;
        let trimmed = line_body.trim_start();

        // 1. Code Fence Tracking
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence {
            let marker = trimmed.chars().next();
            if in_fence {
                if marker == fence_marker {
                    in_fence = false;
                    fence_marker = None;
                }
            } else {
                in_fence = true;
                fence_marker = marker;
            }
            // Pass through fencing lines exactly as is
            output.push_str(line_body);
            output.push_str(line_ending);
            last_line_was_blank = trimmed.is_empty();
            continue;
        }

        // 2. Normalization Logic (Only outside fences)
        if !in_fence {
            // Check for JSX opening tags
            if let Some(tag) = parse_jsx_tag(trimmed) {
                // Heuristic: Capitalized tag = Component. Lowercase = HTML.
                // If it's a Component and previous line wasn't blank, insert blank line.
                let is_component = tag.name.chars().next().is_some_and(|c| c.is_uppercase());

                // Only insert if not already blank
                if is_component && !last_line_was_blank {
                    output.push('\n');
                }

                // We are not tracking specific component names anymore.
                // Just pass the line through.
                output.push_str(line_body);
                output.push_str(line_ending);

                // The line we just added is obviously not blank
                last_line_was_blank = false;
                continue;
            }
        }

        // Pass through regular lines
        output.push_str(line_body);
        output.push_str(line_ending);

        last_line_was_blank = trimmed.is_empty();
    }

    output
}

/// Collapses multiline wrapper tags (like `<p>`) that contain only JSX components.
///
/// This fixes a parsing issue in markdown-rs where multiline JSX elements inside
/// list items cause tag mismatch errors. The pattern:
///
/// ```text
///     <p>
///       <Spoiler>content</Spoiler>
///     </p>
/// ```
///
/// Gets transformed to:
///
/// ```text
///     <p><Spoiler>content</Spoiler></p>
/// ```
///
/// This function uses generic rules without hardcoding specific component names:
/// - Lowercase tags like `<p>`, `<div>` are detected as HTML wrappers
/// - Uppercase tags like `<Spoiler>`, `<Option>` are detected as components
pub fn collapse_multiline_wrapper_tags(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut output = String::with_capacity(input.len());
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Detect lowercase HTML wrapper tags: <p>, <div>, <span>, etc.
        // These often wrap JSX components and cause tag mismatch issues
        if let Some(tag_name) = detect_simple_html_wrapper(trimmed) {
            // Look for pattern: <tag>\n  <Component>...</Component>\n  </tag>
            if i + 2 < lines.len() {
                let next_line = lines[i + 1].trim();
                let close_line = lines[i + 2].trim();
                let expected_close = format!("</{}>", tag_name);

                // Check if next line is a component (uppercase) and has matching closer
                let is_component_content = next_line.starts_with('<')
                    && next_line.chars().nth(1).is_some_and(|c| c.is_uppercase());
                let has_closing = close_line == expected_close;

                if is_component_content && has_closing {
                    let indent = &line[..line.len() - trimmed.len()];
                    output.push_str(indent);
                    output.push('<');
                    output.push_str(tag_name);
                    output.push('>');
                    output.push_str(next_line);
                    output.push_str(&expected_close);
                    output.push('\n');
                    i += 3;
                    continue;
                }
            }
        }

        output.push_str(line);
        output.push('\n');
        i += 1;
    }

    // Handle case where input doesn't end with newline
    if !input.ends_with('\n') && !output.is_empty() && output.ends_with('\n') {
        output.pop();
    }

    output
}

/// Normalizes list-embedded JSX components (tab components) to prevent tag mismatch errors.
///
/// Tab components inside lists cause markdown-rs to misinterpret list boundaries.
/// This function inserts blank lines around indented tab components to force proper parsing.
///
/// Target components:
/// - `PackageManagerTabs`, `StaticSsrTabs`, `UIFrameworkTabs`, `TabItem`
/// - `Fragment` (when `slot=` attribute is present)
///
/// IMPORTANT: This function processes entire components as units using depth-tracking.
/// Nested components (like `<Fragment slot="...">` inside `<StaticSsrTabs>`) are passed
/// through as-is without blank line insertion, which would break the parent component.
pub fn normalize_list_jsx_components(input: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let mut output = String::with_capacity(input.len() + 100);
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Check if this is a list JSX component (opening tag)
        if let Some(tag_info) = parse_jsx_tag(trimmed).filter(is_list_jsx_component) {
            // Check if we need a blank line before
            if i > 0 && needs_blank_line_before(&lines, i) {
                output.push('\n');
            }

            output.push_str(line);
            output.push('\n');

            // For self-closing tags, check if we need blank line after
            if tag_info.self_closing {
                if i + 1 < lines.len() && needs_blank_line_after(&lines, i) {
                    output.push('\n');
                }
                i += 1;
                continue;
            }

            // For non-self-closing tags, find matching closing tag with depth tracking
            // and output all inner content as-is (no blank line insertion for nested components)
            let close_tag = format!("</{}>", tag_info.name);
            let open_prefix = format!("<{}", tag_info.name);
            let mut j = i + 1;
            let mut depth = 1;

            while j < lines.len() && depth > 0 {
                let inner_trimmed = lines[j].trim_start();

                // Track nested same-name components (opening tags increase depth)
                if inner_trimmed.starts_with(&open_prefix) {
                    // Check it's actually an opening tag, not a different component
                    // e.g., <Tabs vs <TabsItem - we need the char after open_prefix
                    let after_name = inner_trimmed.get(open_prefix.len()..);
                    let is_same_component = after_name.is_none_or(|s| {
                        s.is_empty()
                            || s.starts_with('>')
                            || s.starts_with(' ')
                            || s.starts_with('/')
                    });
                    if is_same_component && !inner_trimmed.trim_end().ends_with("/>") {
                        depth += 1;
                    }
                }

                // Track closing tags (decrease depth)
                if inner_trimmed.starts_with(&close_tag) {
                    depth -= 1;
                }

                if depth > 0 {
                    // Output inner lines as-is (no blank line insertion)
                    output.push_str(lines[j]);
                    output.push('\n');
                }
                j += 1;
            }

            // Output closing tag
            if j > i + 1 && depth == 0 {
                output.push_str(lines[j - 1]);
                output.push('\n');

                // Blank line after closing if needed
                if j < lines.len() && needs_blank_line_after(&lines, j - 1) {
                    output.push('\n');
                }
                i = j;
                continue;
            }

            // If we couldn't find matching closing tag, just continue normally
            i += 1;
            continue;
        }

        output.push_str(line);
        output.push('\n');
        i += 1;
    }

    // Handle case where input doesn't end with newline
    if !input.ends_with('\n') && !output.is_empty() && output.ends_with('\n') {
        output.pop();
    }

    output
}

/// List of tab component names that need special handling in list context.
const LIST_JSX_COMPONENTS: &[&str] = &[
    // Tab components
    "PackageManagerTabs",
    "StaticSsrTabs",
    "UIFrameworkTabs",
    "Tabs",
    "TabItem",
    // Tutorial/content components
    "Steps",
    "Box",
];

/// Checks if a tag is a list-embedded JSX component that needs special handling.
fn is_list_jsx_component(tag: &JsxTagInfo) -> bool {
    LIST_JSX_COMPONENTS.contains(&tag.name.as_str())
        || (tag.name == "Fragment" && tag.has_slot_attr)
        // Handle custom elements (lowercase with dash, like mf-directive)
        || tag.name.contains('-')
}

/// Checks if a blank line should be inserted before the component at index i.
fn needs_blank_line_before(lines: &[&str], i: usize) -> bool {
    // If the line immediately before is blank, don't insert another blank line
    if i > 0 && lines[i - 1].trim().is_empty() {
        return false;
    }

    // Look backwards for the previous non-blank line
    let mut prev_idx = i.saturating_sub(1);
    while prev_idx > 0 && lines[prev_idx].trim().is_empty() {
        prev_idx -= 1;
    }

    let prev_trimmed = lines[prev_idx].trim();

    // Don't insert if the first line is this component (nothing before it)
    if prev_trimmed.is_empty() {
        return false;
    }

    // If previous line is a closing tag like </Fragment>, </TabItem>, etc., don't add blank
    if prev_trimmed.starts_with("</") {
        return false;
    }

    true
}

/// Checks if a blank line should be inserted after the component at index i.
fn needs_blank_line_after(lines: &[&str], i: usize) -> bool {
    if i + 1 >= lines.len() {
        return false;
    }

    let next_trimmed = lines[i + 1].trim();

    // Don't insert if next line is already blank
    if next_trimmed.is_empty() {
        return false;
    }

    // Don't insert if next line is an opening component tag (they flow together)
    if parse_jsx_tag(next_trimmed)
        .filter(is_list_jsx_component)
        .is_some()
    {
        return false;
    }

    // Don't insert if next line is a closing tag
    if next_trimmed.starts_with("</") {
        return false;
    }

    true
}

/// Detects simple HTML wrapper tags like <p>, <div>, <span>
/// Returns the tag name if it's a simple lowercase tag
fn detect_simple_html_wrapper(trimmed: &str) -> Option<&str> {
    // Simple patterns: "<p>", "<div>", etc.
    let simple_tags = ["p", "div", "span", "li", "td", "th"];
    for tag in &simple_tags {
        let open = format!("<{}>", tag);
        if trimmed == open {
            return Some(tag);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inserts_blank_line_before_component() {
        let input = "Some text\n<MyComponent>\ncontent\n</MyComponent>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(result.contains("Some text\n\n<MyComponent>"));
    }

    #[test]
    fn test_no_blank_line_before_html_tag() {
        let input = "Some text\n<div>\ncontent\n</div>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(result.contains("Some text\n<div>"));
    }

    #[test]
    fn test_preserves_existing_blank_line() {
        let input = "Some text\n\n<Box>\ncontent\n</Box>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn test_no_blank_line_inside_fence() {
        let input = "```\n<Box>\n```\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_collapse_multiline_p_with_component() {
        let input = "    <p>\n      <Spoiler>Content</Spoiler>\n    </p>\n";
        let result = collapse_multiline_wrapper_tags(input);
        assert_eq!(result, "    <p><Spoiler>Content</Spoiler></p>\n");
    }

    #[test]
    fn test_collapse_preserves_non_matching_content() {
        let input = "<p>\nSome text\n</p>\n";
        let result = collapse_multiline_wrapper_tags(input);
        // This should NOT be collapsed because content is plain text, not a component
        assert_eq!(result, input);
    }

    #[test]
    fn test_normalize_list_jsx_inserts_blank_before_tabs() {
        let input =
            "1. List item\n    <PackageManagerTabs>\n    content\n    </PackageManagerTabs>\n";
        let result = normalize_list_jsx_components(input);
        assert!(
            result.contains("1. List item\n\n    <PackageManagerTabs>"),
            "Should insert blank line before indented PackageManagerTabs. Got: {}",
            result
        );
    }

    #[test]
    fn test_normalize_list_jsx_preserves_blank_after_closing() {
        let input = "    </PackageManagerTabs>\n\n3. Next item\n";
        let result = normalize_list_jsx_components(input);
        // No change needed - it's a closing tag, not an opening
        assert_eq!(result, input);
    }

    #[test]
    fn test_normalize_list_jsx_fragment_with_slot() {
        let input = "1. Item\n    <Fragment slot=\"npm\">\n    code\n    </Fragment>\n";
        let result = normalize_list_jsx_components(input);
        assert!(
            result.contains("1. Item\n\n    <Fragment slot="),
            "Should insert blank line before Fragment with slot. Got: {}",
            result
        );
    }

    #[test]
    fn test_normalize_list_jsx_no_change_non_list_context() {
        let input = "<PackageManagerTabs>\ncontent\n</PackageManagerTabs>\n";
        let result = normalize_list_jsx_components(input);
        // No indentation = not in list context, should not change
        assert_eq!(result, input);
    }

    #[test]
    fn test_normalize_list_jsx_nested_components_preserved() {
        // Nested Fragment inside StaticSsrTabs should NOT get blank lines inserted
        // This was the bug that caused "Unexpected closing tag </StaticSsrTabs>, expected </Fragment>"
        let input = r#"1. Item

    <StaticSsrTabs>
    <Fragment slot="static">
    Static content
    </Fragment>
    <Fragment slot="ssr">
    SSR content
    </Fragment>
    </StaticSsrTabs>

2. Next item
"#;
        let result = normalize_list_jsx_components(input);

        // Should NOT have blank lines around nested Fragment tags
        assert!(
            !result.contains("</Fragment>\n\n    <Fragment"),
            "Should NOT insert blank line between nested Fragment tags. Got:\n{}",
            result
        );

        // The overall structure should be preserved - blank line before StaticSsrTabs
        // and blank line after (which already exists)
        assert!(
            result.contains("1. Item\n\n    <StaticSsrTabs>"),
            "Should preserve blank line before StaticSsrTabs. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_normalize_list_jsx_deeply_nested_same_component() {
        // Test depth tracking with same-named nested components
        let input = r#"1. Item

    <Box>
    <Box>
    Inner box
    </Box>
    </Box>

2. Next
"#;
        let result = normalize_list_jsx_components(input);

        // Inner Box should NOT get blank lines
        assert!(
            !result.contains("</Box>\n\n    </Box>"),
            "Should NOT insert blank line between nested closing Box tags. Got:\n{}",
            result
        );
    }
}
