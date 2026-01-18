//! JSX indentation normalization utilities.

/// Normalizes JSX indentation to prevent content from being treated as code blocks.
///
/// With code_indented: false in MDX mode, deep indentation won't trigger code blocks.
/// This function strips leading indentation from JSX tags (opening and closing tags)
/// while preserving all other content as-is.
///
/// Special handling for `<Steps>`: Non-indented content after a blank line inside
/// a numbered list gets 3-space indentation added to make it list continuation.
pub fn normalize_mdx_jsx_indentation(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_fence = false;
    let mut fence_marker: Option<char> = None;
    let mut jsx_stack: Vec<String> = Vec::new();

    // Steps-specific state tracking
    let mut in_steps = false;
    let mut in_numbered_list = false;
    let mut after_blank_in_list = false;
    let mut in_steps_directive = false;
    let mut in_filetree = false;

    // Blank line tracking for component tag insertion
    let mut last_line_was_blank = true; // Treat start of file as having a blank line

    for line in input.split_inclusive('\n') {
        let (line_body, line_ending) = if let Some(stripped) = line.strip_suffix('\n') {
            let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
            (stripped, &line[stripped.len()..])
        } else {
            (line, "")
        };

        let trimmed = line_body.trim_start();

        // Track code fences to avoid processing JSX-like content inside them
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
            output.push_str(line_body);
            output.push_str(line_ending);
            last_line_was_blank = trimmed.is_empty();
            continue;
        }

        if !in_fence {
            // Track JSX tags but preserve indentation when inside JSX blocks
            if let Some(tag) = jsx_open_tag(trimmed) {
                // Insert blank line before top-level component tags if needed.
                // Uppercase first letter = user-defined component (Box, Steps, Aside, etc.)
                // Lowercase first letter = standard HTML tag (div, span, etc.) - no insertion
                let is_component = tag.name.chars().next().is_some_and(|c| c.is_uppercase());
                if jsx_stack.is_empty() && is_component && !last_line_was_blank {
                    output.push('\n');
                }

                // Track entering <Steps>
                if tag.name.eq_ignore_ascii_case("steps") {
                    in_steps = true;
                    in_numbered_list = false;
                    after_blank_in_list = false;
                }
                // Track entering <FileTree>
                if tag.name.eq_ignore_ascii_case("filetree") {
                    in_filetree = true;
                }
                if !tag.self_closing {
                    jsx_stack.push(tag.name);
                }
                // Preserve indentation for nested JSX tags inside JSX blocks
                if jsx_stack.len() > 1 {
                    output.push_str(line_body);
                } else {
                    output.push_str(trimmed);
                }
                output.push_str(line_ending);
                last_line_was_blank = false; // JSX tag line is not blank
                continue;
            }

            if let Some(tag_name) = jsx_close_tag(trimmed) {
                // Track exiting <Steps>
                if tag_name.eq_ignore_ascii_case("steps") {
                    in_steps = false;
                    in_numbered_list = false;
                    after_blank_in_list = false;
                    in_steps_directive = false;
                }
                // Track exiting <FileTree>
                if tag_name.eq_ignore_ascii_case("filetree") {
                    in_filetree = false;
                }
                // Check if we should preserve indentation before popping
                let should_preserve_indent = jsx_stack.len() > 1;
                if let Some(last) = jsx_stack.pop()
                    && last != tag_name
                {
                    jsx_stack.clear();
                }
                // Preserve indentation for nested JSX closing tags
                if should_preserve_indent {
                    output.push_str(line_body);
                } else {
                    output.push_str(trimmed);
                }
                output.push_str(line_ending);
                last_line_was_blank = false; // JSX close tag line is not blank
                continue;
            }

            // Inside JSX blocks: preserve original indentation
            if !jsx_stack.is_empty() {
                let leading_spaces = line_body.len() - trimmed.len();

                // Steps-specific: track numbered list context and blank lines
                // Skip this logic when inside FileTree to preserve its list structure
                if in_steps && !in_filetree {
                    if is_numbered_list_item(trimmed) {
                        in_numbered_list = true;
                        after_blank_in_list = false;
                        in_steps_directive = false;
                    } else if trimmed.is_empty() && in_numbered_list {
                        after_blank_in_list = true;
                    } else if in_numbered_list && leading_spaces == 0 {
                        // Check for directive syntax (:::word or just :::)
                        let is_directive_opener = trimmed.starts_with(":::")
                            && trimmed.len() > 3
                            && trimmed.chars().nth(3).is_some_and(|c| c.is_alphabetic());
                        let is_directive_closer = trimmed == ":::";

                        if is_directive_opener && !in_steps_directive {
                            // Start of directive - add indent and track state
                            in_steps_directive = true;
                            after_blank_in_list = false;
                            output.push_str("   ");
                            output.push_str(trimmed);
                            output.push_str(line_ending);
                            last_line_was_blank = false; // Directive opener is not blank
                            continue;
                        } else if is_directive_closer && in_steps_directive {
                            // End of directive - add indent and reset state
                            in_steps_directive = false;
                            output.push_str("   ");
                            output.push_str(trimmed);
                            output.push_str(line_ending);
                            last_line_was_blank = false; // Directive closer is not blank
                            continue;
                        } else if in_steps_directive {
                            // Inside directive - add indent to all content
                            output.push_str("   ");
                            output.push_str(trimmed);
                            output.push_str(line_ending);
                            last_line_was_blank = trimmed.is_empty();
                            continue;
                        } else if after_blank_in_list
                            && !trimmed.is_empty()
                            && !trimmed.starts_with("</")
                        {
                            // Non-indented content after blank line in numbered list
                            // Add 3-space indent to make it list continuation
                            output.push_str("   ");
                            output.push_str(trimmed);
                            output.push_str(line_ending);
                            after_blank_in_list = false;
                            last_line_was_blank = false; // Content is not blank
                            continue;
                        }
                    }
                }

                // Preserve original indentation for content inside JSX blocks
                output.push_str(line_body);
                output.push_str(line_ending);
                last_line_was_blank = trimmed.is_empty();
                continue;
            }
        }

        output.push_str(line_body);
        output.push_str(line_ending);
        last_line_was_blank = trimmed.is_empty();
    }

    output
}

struct JsxTag {
    name: String,
    self_closing: bool,
}

fn jsx_open_tag(trimmed: &str) -> Option<JsxTag> {
    if !trimmed.starts_with('<') || trimmed.starts_with("</") || trimmed.starts_with("<!") {
        return None;
    }
    let mut chars = trimmed.chars().peekable();
    chars.next();
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

fn jsx_close_tag(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("</") {
        return None;
    }
    let mut chars = trimmed.chars().peekable();
    chars.next();
    chars.next();
    let mut name = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == '>' {
            break;
        }
        name.push(ch);
        chars.next();
    }
    if name.is_empty() { None } else { Some(name) }
}

/// Check if a line starts with a numbered list item pattern (e.g., "1. ", "2. ", "10. ")
fn is_numbered_list_item(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip digits
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Must have at least one digit, followed by ". "
    i > 0 && bytes.get(i) == Some(&b'.') && bytes.get(i + 1) == Some(&b' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steps_content_normalization() {
        let input = "<Steps>\n1. Step\n\nContent\n</Steps>\n";
        let output = normalize_mdx_jsx_indentation(input);
        assert!(output.contains("   Content"));
    }

    #[test]
    fn test_nested_jsx_preserves_indent() {
        let input = "<Steps>\n1. Step\n\n   <Tabs>\n   content\n   </Tabs>\n</Steps>\n";
        let output = normalize_mdx_jsx_indentation(input);
        assert!(output.contains("   <Tabs>"));
    }

    #[test]
    fn test_inserts_blank_line_before_component() {
        // Uppercase first letter = component → insert blank line
        let input = "Some text\n<MyComponent>\ncontent\n</MyComponent>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(result.contains("Some text\n\n<MyComponent>"));
    }

    #[test]
    fn test_no_blank_line_before_html_tag() {
        // Lowercase first letter = HTML tag → no blank line insertion
        let input = "Some text\n<div>\ncontent\n</div>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(result.contains("Some text\n<div>"));
    }

    #[test]
    fn test_preserves_existing_blank_line() {
        let input = "Some text\n\n<Box>\ncontent\n</Box>\n";
        let result = normalize_mdx_jsx_indentation(input);
        // Blank line should not be duplicated
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn test_no_blank_line_inside_fence() {
        let input = "```\n<Box>\n```\n";
        let result = normalize_mdx_jsx_indentation(input);
        // Inside code fence, no changes should be made
        assert_eq!(result, input);
    }

    #[test]
    fn test_no_blank_line_for_nested_component() {
        // Nested components should not get blank line inserted
        let input = "<Parent>\n<Child />\n</Parent>\n";
        let result = normalize_mdx_jsx_indentation(input);
        assert!(!result.contains("\n\n<Child"));
    }
}
