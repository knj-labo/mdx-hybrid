//! Directive preprocessing for the mdast renderer.

use crate::transform::code_fence::{advance_fence_state, FenceState};
use crate::transform::directives::{is_directive_closer, parse_opening_directive};
use std::fmt::Write;

/// Preprocesses input markdown to convert directive syntax into internal JSX tags.
///
/// This allows markdown-rs to preserve directive structure even though it doesn't
/// natively support `::: note` syntax. Using JSX tags ensures markdown between
/// the markers is still parsed correctly and unifies directive handling with JSX.
///
/// # Examples
///
/// Input:
/// ```text
/// :::note[Title]
/// Content
/// :::
/// ```
///
/// Output:
/// ```text
/// <mf-directive name="note" title="Title">
/// Content
/// </mf-directive>
/// ```
pub fn preprocess_directives(input: &str) -> String {
    let mut fence_state = FenceState::default();
    let mut output = String::with_capacity(input.len());
    // Track directive names and their leading whitespace for proper closing
    let mut directive_stack: Vec<(String, String)> = Vec::new();

    for line in input.lines() {
        let fence_outcome = advance_fence_state(line, fence_state);
        fence_state = fence_outcome.next_state;

        // Inside code fence - passthrough without processing
        if fence_outcome.skip_imports {
            writeln!(output, "{}", line).ok();
            continue;
        }

        // Check for directive opening
        if let Some(opening) = parse_opening_directive(line) {
            // Preserve leading whitespace from original line
            let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();

            directive_stack.push((opening.name.clone(), leading_ws.clone()));

            // Convert to JSX container tag (NOT self-closing)
            write!(
                output,
                "{}<mf-directive name=\"{}\"",
                leading_ws, opening.name
            )
            .ok();

            if let Some(title) = &opening.bracket_title {
                let escaped_title = title.replace('"', "&quot;");
                write!(output, " title=\"{}\"", escaped_title).ok();
            }

            if !opening.raw_attrs.is_empty() {
                write!(
                    output,
                    " attrs=\"{}\"",
                    opening.raw_attrs.replace('"', "&quot;")
                )
                .ok();
            }

            // Opening tag, not self-closing
            writeln!(output, ">").ok();
            continue;
        }

        // Check for directive closer
        if is_directive_closer(line) && !directive_stack.is_empty() {
            let (_, leading_ws) = directive_stack.pop().unwrap();
            // Close the container tag with same indentation as opener
            writeln!(output, "{}</mf-directive>", leading_ws).ok();
            continue;
        }

        // Regular line - passthrough
        writeln!(output, "{}", line).ok();
    }

    // Close any unclosed directives
    while let Some((_, leading_ws)) = directive_stack.pop() {
        writeln!(output, "{}</mf-directive>", leading_ws).ok();
    }

    output
}
