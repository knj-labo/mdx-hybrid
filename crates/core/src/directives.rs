//! Directive rewriting utilities.

use std::fmt::Write as _;

use crate::code_fence::{FenceState, advance_fence_state};

/// Ensures Aside import is present when directives were rewritten.
/// If `count > 0` and no existing import from `@astrojs/starlight/components` is present,
/// it pushes the import into the `hoisted` list.
pub fn ensure_aside_import(hoisted: &mut Vec<String>, directive_count: usize) {
    if directive_count == 0 {
        return;
    }

    let already_imported = hoisted
        .iter()
        .any(|line| line.contains("@astrojs/starlight/components"));

    if !already_imported {
        hoisted.insert(
            0,
            "import { Aside } from '@astrojs/starlight/components';".to_string(),
        );
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DirectiveOpening {
    name: String,
    bracket_title: Option<String>,
    raw_attrs: String,
}

impl DirectiveOpening {
    pub(crate) fn to_aside_start(&self) -> String {
        let mut tag = String::from("<Aside");

        // type attribute is always injected/overwritten.
        write!(tag, " type=\"{}\"", self.name).ok();

        // Attributes from source line: keep as-is after stripping conflicting keys.
        if !self.raw_attrs.is_empty() {
            tag.push(' ');
            tag.push_str(&self.raw_attrs);
        }

        // Title resolution: bracket > attribute (attributes already stripped of title when bracket present).
        if let Some(title) = self.bracket_title.as_ref() {
            write!(tag, " title=\"{}\"", title).ok();
        }

        tag.push('>');
        tag
    }

    pub(crate) fn to_aside_end(&self) -> String {
        "</Aside>".to_string()
    }
}

pub(crate) fn parse_opening_directive(line: &str) -> Option<DirectiveOpening> {
    let trimmed = line.trim();
    if !trimmed.starts_with(":::") {
        return None;
    }

    // Strip leading :::
    let after_colons = &trimmed[3..];
    let mut chars = after_colons.chars().peekable();

    // Read directive name (alphabetic)
    let mut name = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_alphabetic() {
            name.push(ch.to_ascii_lowercase());
            chars.next();
        } else {
            break;
        }
    }

    if name.is_empty() || !is_supported_name(&name) {
        return None;
    }

    // Optional bracket title
    let mut bracket_title = None;
    if let Some(&'[') = chars.peek() {
        chars.next(); // consume [
        let mut title = String::new();
        while let Some(&ch) = chars.peek() {
            chars.next();
            if ch == ']' {
                bracket_title = Some(title);
                break;
            } else {
                title.push(ch);
            }
        }
    }

    // Remaining slice treated as attributes (trim leading whitespace)
    let remaining: String = chars.collect();
    let raw_attrs = normalize_attrs(remaining.trim(), bracket_title.is_some());

    Some(DirectiveOpening {
        name,
        bracket_title,
        raw_attrs,
    })
}

fn normalize_attrs(attrs: &str, has_bracket_title: bool) -> String {
    if attrs.is_empty() {
        return String::new();
    }

    let mut cleaned = String::new();

    for tok in attrs.split_whitespace() {
        let key = tok
            .split('=')
            .next()
            .unwrap_or("")
            .trim_matches(|c: char| c == ' ');

        // Remove any type=... attribute; we always override with directive name.
        if key.eq_ignore_ascii_case("type") {
            continue;
        }

        // Remove title when bracket title exists.
        if has_bracket_title && key.eq_ignore_ascii_case("title") {
            continue;
        }

        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(tok);
    }

    cleaned
}

pub(crate) fn is_directive_closer(line: &str) -> bool {
    line.trim() == ":::"
}

fn is_supported_name(name: &str) -> bool {
    matches!(
        name,
        "note" | "tip" | "info" | "caution" | "warning" | "danger"
    )
}

/// Legacy string-based directive rewrite used internally by the streaming adapter for block-local transforms.
pub(crate) fn rewrite_directives_to_asides(input: &str) -> (String, usize) {
    let mut fence_state = FenceState::default();
    let mut output = String::new();
    let mut count = 0usize;

    // Stack of active directive names (to support nesting if encountered).
    let mut directive_stack: Vec<DirectiveOpening> = Vec::new();

    for line in input.lines() {
        let fence_outcome = advance_fence_state(line, fence_state);
        fence_state = fence_outcome.next_state;

        if fence_outcome.skip_imports {
            // Inside code fence; passthrough without touching directive syntax.
            writeln!(output, "{}", line).ok();
            continue;
        }

        if let Some(opening) = parse_opening_directive(line) {
            count += 1;
            directive_stack.push(opening.clone());
            let start_tag = opening.to_aside_start();
            writeln!(output, "{}", start_tag).ok();
            continue;
        }

        if is_directive_closer(line)
            && let Some(opened) = directive_stack.pop()
        {
            let end_tag = opened.to_aside_end();
            writeln!(output, "{}", end_tag).ok();
            continue;
        }

        writeln!(output, "{}", line).ok();
    }

    // For any unclosed directives, close them at the end to avoid broken output.
    while directive_stack.pop().is_some() {
        writeln!(output, "</Aside>").ok();
    }

    (output, count)
}

#[cfg(test)]
mod tests {
    use crate::{
        DirectiveAdapter, MarkdownStream, RewriteOptions, StreamingRewriter, get_event_iterator,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn rewrites_simple_note() {
        let input = ":::note\nhello\n:::";
        let (out, count) = render_with_adapter(input);
        assert_eq!(count, 1);
        assert!(out.contains("<Aside type=\"note\">"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn preserves_code_fence() {
        let input = "```\n:::note\n```";
        let (out, count) = render_with_adapter(input);
        assert_eq!(count, 0);
        assert!(out.contains(":::note"));
    }

    #[test]
    fn bracket_title_overrides_attr() {
        let input = ":::note[Hi] title=\"Ignore\"\nBody\n:::";
        let (out, _) = render_with_adapter(input);
        assert!(out.contains("<Aside type=\"note\" title=\"Hi\">"));
        assert!(!out.contains("Ignore"));
    }

    #[test]
    fn type_attr_is_overwritten() {
        let input = ":::warning type=\"old\"\nBody\n:::";
        let (out, _) = render_with_adapter(input);
        assert!(out.contains("<Aside type=\"warning\">"));
        assert!(!out.contains("type=\"old\""));
    }

    #[test]
    fn nested_directives_close_in_order() {
        let input = ":::note\nOuter\n:::tip\nInner\n:::\n:::";
        let (out, count) = render_with_adapter(input);
        assert_eq!(count, 2);
        assert!(out.contains("<Aside type=\"note\">"));
        assert!(out.contains("<Aside type=\"tip\">"));
        assert!(out.contains("</Aside>"));
    }

    #[test]
    fn attribute_title_retained_when_no_bracket() {
        let input = ":::info title=\"Keep me\"\nBody\n:::";
        let (out, _) = render_with_adapter(input);
        assert!(out.contains("<Aside type=\"info\" title=\"Keep me\">"));
    }

    #[test]
    fn arbitrary_attributes_are_preserved() {
        let input = ":::danger data-test=\"yes\" class=\"foo\"\nBody\n:::";
        let (out, _) = render_with_adapter(input);
        assert!(out.contains("<Aside type=\"danger\""));
        assert!(out.contains("data-test=\"yes\""));
        assert!(out.contains("class=\"foo\""));
    }

    #[test]
    fn ignores_directive_like_text_in_inline_code() {
        let input = "Here is `:::note` inside code";
        let (out, count) = render_with_adapter(input);
        assert_eq!(count, 0);
        assert!(out.contains("<code>:::note</code>"));
    }

    #[test]
    fn ignores_directive_like_text_in_html_attribute() {
        let input = "<div title=\":note\">content</div>";
        let (out, count) = render_with_adapter(input);
        assert_eq!(count, 0);
        assert!(out.contains("<div title=\":note\">content</div>"));
    }

    fn render_with_adapter(input: &str) -> (String, usize) {
        let events = get_event_iterator(input).expect("parser");
        let directive_count = Rc::new(RefCell::new(0usize));
        let pipeline = DirectiveAdapter::new(events, Rc::clone(&directive_count));
        let writer = StreamingRewriter::new(Vec::new(), RewriteOptions::default());
        let writer = pipeline.stream_to_writer(writer).expect("render stream");
        let html = String::from_utf8(writer.into_inner().expect("flush")).expect("utf8");
        (html, *directive_count.borrow())
    }
}
