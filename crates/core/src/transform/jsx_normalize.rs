//! JSX indentation normalization utilities.

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
        let (line_body, line_ending) = if let Some(stripped) = line.strip_suffix('\n') {
            let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
            (stripped, &line[stripped.len()..])
        } else {
            (line, "")
        };

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
            if let Some(tag) = jsx_open_tag(trimmed) {
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

struct JsxTag {
    name: String,
    #[allow(dead_code)]
    self_closing: bool,
}

fn jsx_open_tag(trimmed: &str) -> Option<JsxTag> {
    // Basic JSX tag detection: <Name ...
    if !trimmed.starts_with('<') || trimmed.starts_with("</") || trimmed.starts_with("<!") {
        return None;
    }

    let mut chars = trimmed.chars().peekable();
    chars.next(); // skip '<'

    let mut name = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == '>' || ch == '/' {
            break;
        }
        name.push(ch);
        chars.next();
    }

    if name.is_empty() {
        return None;
    }

    let self_closing = trimmed.trim_end().ends_with("/>");
    Some(JsxTag { name, self_closing })
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

}
