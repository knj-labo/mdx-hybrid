#![allow(missing_docs)]

use crate::{Span, error::ParseDiagnostics};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanEvent<'a> {
    Markdown {
        text: &'a str,
        span: Span,
    },
    Code {
        text: &'a str,
        span: Span,
    },
    JsxOpen {
        name: &'a str,
        attrs: &'a str,
        is_self_closing: bool,
        span: Span,
    },
    JsxClose {
        name: &'a str,
        span: Span,
    },
}

pub struct ScanIter<'a> {
    input: &'a str,
    cursor: usize,
    pending_start: Option<usize>,
    pending_end: usize,
    diagnostics: Rc<RefCell<ParseDiagnostics>>,
}

impl<'a> ScanIter<'a> {
    pub fn new(input: &'a str) -> Self {
        Self::new_with_diagnostics(input, Rc::new(RefCell::new(ParseDiagnostics::new())))
    }

    pub fn new_with_diagnostics(
        input: &'a str,
        diagnostics: Rc<RefCell<ParseDiagnostics>>,
    ) -> Self {
        Self {
            input,
            cursor: 0,
            pending_start: None,
            pending_end: 0,
            diagnostics,
        }
    }

    pub fn diagnostics(&self) -> Rc<RefCell<ParseDiagnostics>> {
        Rc::clone(&self.diagnostics)
    }

    fn flush_markdown(&mut self) -> Option<ScanEvent<'a>> {
        let start = self.pending_start?;
        let end = self.pending_end;
        self.pending_start = None;
        self.pending_end = 0;
        if start >= end {
            return None;
        }
        Some(ScanEvent::Markdown {
            text: &self.input[start..end],
            span: Span::new(start, end),
        })
    }

    fn extend_markdown(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        match self.pending_start {
            Some(_) => {
                self.pending_end = end;
            }
            None => {
                self.pending_start = Some(start);
                self.pending_end = end;
            }
        }
    }
}

impl<'a> Iterator for ScanIter<'a> {
    type Item = ScanEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.input.as_bytes();

        loop {
            if self.cursor >= bytes.len() {
                return self.flush_markdown();
            }

            let prev_cursor = self.cursor;

            let line_start = find_line_start(bytes, self.cursor);
            if is_fence_start(bytes, line_start).is_some() && self.cursor != line_start {
                self.cursor = line_start;
            }

            if let Some((marker, count)) = is_fence_start(bytes, self.cursor) {
                if let Some(fence_end) = find_fence_end(bytes, self.cursor + 1, marker, count) {
                    let end_pos = find_byte(bytes, fence_end, b'\n')
                        .map(|newline| newline + 1)
                        .unwrap_or_else(|| self.input.len());
                    if let Some(event) = self.flush_markdown() {
                        return Some(event);
                    }
                    let span = Span::new(self.cursor, end_pos);
                    let event = ScanEvent::Code {
                        text: &self.input[self.cursor..end_pos],
                        span,
                    };
                    self.cursor = end_pos;
                    return Some(event);
                } else {
                    // Unclosed fence - add warning
                    use crate::error::ParseWarning;
                    let line_num = self.input[..self.cursor].lines().count() + 1;
                    let context_end = (self.cursor + 40).min(self.input.len());
                    let context = &self.input[self.cursor..context_end];

                    self.diagnostics
                        .borrow_mut()
                        .add_warning(ParseWarning::UnclosedCodeFence {
                            line: line_num,
                            marker: marker as char,
                            context: context.to_string(),
                        });
                }
                self.extend_markdown(self.cursor, self.input.len());
                self.cursor = self.input.len();
                continue;
            }

            if bytes.get(self.cursor) == Some(&b'<') {
                let remaining = &self.input[self.cursor..];
                if let Some((name, consumed)) = parse_jsx_close(remaining) {
                    if let Some(event) = self.flush_markdown() {
                        return Some(event);
                    }
                    let start = self.cursor;
                    let end = start + consumed;
                    self.cursor = end;
                    return Some(ScanEvent::JsxClose {
                        name,
                        span: Span::new(start, end),
                    });
                }
                if let Some((name, attrs, is_self_closing, consumed)) = parse_jsx_open(remaining) {
                    if let Some(event) = self.flush_markdown() {
                        return Some(event);
                    }
                    let start = self.cursor;
                    let end = start + consumed;
                    self.cursor = end;
                    return Some(ScanEvent::JsxOpen {
                        name,
                        attrs,
                        is_self_closing,
                        span: Span::new(start, end),
                    });
                }
            }

            if bytes.get(self.cursor) == Some(&b'`') {
                if let Some(end) = find_inline_code_end(bytes, self.cursor) {
                    // Keep inline code as part of markdown flow to preserve paragraph structure
                    self.extend_markdown(self.cursor, end);
                    self.cursor = end;
                } else {
                    // Unclosed backtick - treat as markdown
                    self.extend_markdown(self.cursor, self.cursor + 1);
                    self.cursor += 1;
                }
                continue;
            }

            let mut next_lt = find_byte(bytes, self.cursor, b'<');
            while let Some(pos) = next_lt {
                let line_start = find_line_start(bytes, pos);
                if is_fence_start(bytes, line_start).is_some() {
                    next_lt = find_byte(bytes, pos + 1, b'<');
                    continue;
                }
                break;
            }
            let next_fence = find_fence_start(bytes, self.cursor);
            let next_tick = find_byte(bytes, self.cursor, b'`');
            let mut next_stop: Option<usize> = None;
            for pos in [next_lt, next_fence, next_tick].into_iter().flatten() {
                next_stop = Some(match next_stop {
                    Some(current) => current.min(pos),
                    None => pos,
                });
            }
            if next_stop.is_none() {
                self.extend_markdown(self.cursor, self.input.len());
                self.cursor = self.input.len();
                continue;
            }

            let pos = next_stop.unwrap();
            if self.cursor < pos {
                self.extend_markdown(self.cursor, pos);
                self.cursor = pos;
                continue;
            }

            self.extend_markdown(self.cursor, self.cursor + 1);
            self.cursor += 1;

            if self.cursor == prev_cursor {
                break;
            }
        }

        self.flush_markdown()
    }
}

pub fn scan(input: &str) -> Vec<ScanEvent<'_>> {
    ScanIter::new(input).collect()
}

pub fn scan_iter(input: &str) -> ScanIter<'_> {
    ScanIter::new(input)
}

fn find_byte(bytes: &[u8], start: usize, target: u8) -> Option<usize> {
    bytes
        .iter()
        .skip(start)
        .position(|&b| b == target)
        .map(|idx| idx + start)
}

fn find_line_start(bytes: &[u8], pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut i = pos;
    while i > 0 {
        if bytes[i - 1] == b'\n' {
            break;
        }
        i -= 1;
    }
    i
}

fn is_line_start(bytes: &[u8], pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    matches!(bytes.get(pos - 1), Some(b'\n'))
}

fn is_fence_start(bytes: &[u8], pos: usize) -> Option<(u8, usize)> {
    let marker_pos = skip_fence_indent(bytes, pos)?;
    let start_byte = *bytes.get(marker_pos)?;
    if !matches!(start_byte, b'`' | b'~') {
        return None;
    }

    let mut count = 0usize;
    let mut i = marker_pos;
    while i < bytes.len() && bytes[i] == start_byte {
        count += 1;
        i += 1;
    }

    if count >= 3 {
        Some((start_byte, count))
    } else {
        None
    }
}

fn find_fence_start(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos < bytes.len() {
        if is_fence_start(bytes, pos).is_some() {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn find_fence_end(bytes: &[u8], start: usize, marker: u8, count: usize) -> Option<usize> {
    let mut pos = start;
    while pos < bytes.len() {
        if is_line_start(bytes, pos)
            && let Some(marker_pos) = skip_fence_indent(bytes, pos)
            && bytes.get(marker_pos) == Some(&marker)
        {
            let mut run = 0usize;
            let mut i = marker_pos;
            while i < bytes.len() && bytes[i] == marker {
                run += 1;
                i += 1;
            }
            if run >= count {
                return Some(marker_pos);
            }
        }
        pos += 1;
    }
    None
}

fn find_inline_code_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'`') {
        return None;
    }

    let mut count = 0usize;
    let mut i = start;
    while i < bytes.len() && bytes[i] == b'`' {
        count += 1;
        i += 1;
    }

    let mut pos = i;
    while pos < bytes.len() {
        if bytes[pos] != b'`' {
            pos += 1;
            continue;
        }
        let mut run = 0usize;
        while pos + run < bytes.len() && bytes[pos + run] == b'`' {
            run += 1;
        }
        if run == count {
            return Some(pos + run);
        }
        pos += run;
    }

    None
}

fn skip_fence_indent(bytes: &[u8], pos: usize) -> Option<usize> {
    if !is_line_start(bytes, pos) {
        return None;
    }

    let mut i = pos;
    let mut spaces = 0usize;

    // Allow up to 12 spaces of indentation to support code fences in nested contexts
    // (e.g., list items, blockquotes). CommonMark allows 0-3 spaces, but in practice
    // code fences in lists can be indented much more (4 spaces per list level + content indent).
    while i < bytes.len() {
        match bytes[i] {
            b' ' if spaces < 12 => {
                spaces += 1;
                i += 1;
            }
            b'\t' if spaces == 0 => {
                i += 1;
                break;
            }
            _ => break,
        }
    }

    Some(i)
}

fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_')
}

fn is_component_tag(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn take_name(input: &str) -> Option<(&str, &str)> {
    let bytes = input.as_bytes();
    let mut end = 0usize;
    while end < bytes.len() && is_name_char(bytes[end]) {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    Some((&input[..end], &input[end..]))
}

fn parse_jsx_close(input: &str) -> Option<(&str, usize)> {
    let rest = input.strip_prefix("</")?;
    let (name, rest) = take_name(rest)?;
    if !is_component_tag(name) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('>')?;
    let consumed = input.len() - rest.len();
    Some((name, consumed))
}

fn parse_jsx_open(input: &str) -> Option<(&str, &str, bool, usize)> {
    let rest = input.strip_prefix('<')?;
    if rest.starts_with('/') {
        return None;
    }
    let (name, rest) = take_name(rest)?;
    if !is_component_tag(name) {
        return None;
    }
    let end = rest.find('>')?;
    let raw_attrs = &rest[..end];
    let rest = &rest[end + 1..];
    let trimmed = raw_attrs.trim();
    let (attrs, is_self_closing) = if let Some(stripped) = trimmed.strip_suffix('/') {
        (stripped.trim_end(), true)
    } else {
        (trimmed, false)
    };
    let consumed = input.len() - rest.len();
    Some((name, attrs, is_self_closing, consumed))
}

#[cfg(test)]
mod tests {
    use super::{ScanEvent, find_fence_end, is_fence_start, scan};

    #[test]
    fn scan_returns_single_markdown_block() {
        let input = "hello";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_coalesces_inline_code_fragments() {
        let input = "Click `button` now";
        let events = scan(input);

        // Inline code is kept as part of markdown flow
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_unclosed_inline_code_is_markdown() {
        let input = "Click `button now";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_empty_returns_empty() {
        let events = scan("");

        assert!(events.is_empty());
    }

    #[test]
    fn scan_emits_self_closing_jsx_element() {
        let events = scan("<Tabs />");

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::JsxOpen {
                name: "Tabs",
                attrs: "",
                is_self_closing: true,
                ..
            }
        ));
    }

    #[test]
    fn scan_emits_jsx_element_with_children() {
        let events = scan("<Steps>hello</Steps>");

        assert!(matches!(
            events.as_slice(),
            [
                ScanEvent::JsxOpen { name: "Steps", .. },
                ScanEvent::Markdown { text: "hello", .. },
                ScanEvent::JsxClose { name: "Steps", .. }
            ]
        ));
    }

    #[test]
    fn scan_keeps_lowercase_html_as_markdown() {
        let input = "<h3>Title</h3>";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_handles_nested_same_tag() {
        let events = scan("<Steps><Steps>inner</Steps>outer</Steps>");

        assert!(matches!(
            events.as_slice(),
            [
                ScanEvent::JsxOpen { name: "Steps", .. },
                ScanEvent::JsxOpen { name: "Steps", .. },
                ScanEvent::Markdown { text: "inner", .. },
                ScanEvent::JsxClose { name: "Steps", .. },
                ScanEvent::Markdown { text: "outer", .. },
                ScanEvent::JsxClose { name: "Steps", .. }
            ]
        ));
    }

    #[test]
    fn scan_ignores_self_closing_in_nested_match() {
        let events = scan("<Steps><Steps />inner</Steps>");

        assert!(matches!(
            events.as_slice(),
            [
                ScanEvent::JsxOpen { name: "Steps", .. },
                ScanEvent::JsxOpen {
                    name: "Steps",
                    is_self_closing: true,
                    ..
                },
                ScanEvent::Markdown { text: "inner", .. },
                ScanEvent::JsxClose { name: "Steps", .. }
            ]
        ));
    }

    #[test]
    fn detects_fence_start_and_end() {
        let input = "```\ncode\n```\n";
        let bytes = input.as_bytes();

        let (marker, count) = is_fence_start(bytes, 0).expect("fence start");
        assert_eq!(marker, b'`');
        assert_eq!(count, 3);

        let end = find_fence_end(bytes, 1, marker, count).expect("fence end");
        assert_eq!(end, 9);
    }

    #[test]
    fn detects_tilde_fence_start() {
        let input = "~~~\ncode\n~~~\n";
        let bytes = input.as_bytes();

        let (marker, count) = is_fence_start(bytes, 0).expect("fence start");
        assert_eq!(marker, b'~');
        assert_eq!(count, 3);
    }

    #[test]
    fn scan_emits_code_block_for_fence() {
        let input = "```\n<Steps>\n```";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Code { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_does_not_parse_jsx_inside_fence() {
        let input = "```\n<Tabs />\n```\n";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Code { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_continues_after_fence() {
        let input = "```\n<Steps>\n```\n<Tabs />";
        let events = scan(input);

        assert!(matches!(
            events.as_slice(),
            [
                ScanEvent::Code {
                    text: "```\n<Steps>\n```\n",
                    ..
                },
                ScanEvent::JsxOpen {
                    name: "Tabs",
                    is_self_closing: true,
                    ..
                }
            ]
        ));
    }

    #[test]
    fn scan_falls_back_on_unclosed_fence() {
        let input = "```\n<Steps>";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_emits_open_when_missing_close_tag() {
        let input = "<Steps>no close";
        let events = scan(input);

        assert!(matches!(
            events.as_slice(),
            [
                ScanEvent::JsxOpen { name: "Steps", .. },
                ScanEvent::Markdown {
                    text: "no close",
                    ..
                }
            ]
        ));
    }

    #[test]
    fn scan_emits_inline_code_event() {
        let input = "Click `button` now";
        let events = scan(input);

        // Inline code is kept as part of markdown flow
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_emits_inline_code_with_jsx_like_content() {
        let input = "Use `<Code>` component";
        let events = scan(input);

        // Inline code with JSX-like content is kept as markdown (not parsed as JSX)
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_treats_unclosed_backtick_as_markdown() {
        let input = "Click `button now";
        let events = scan(input);

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            ScanEvent::Markdown { text, .. } if text == input
        ));
    }

    #[test]
    fn scan_emits_warning_for_unclosed_fence() {
        use super::ScanIter;
        let input = "```rust\nfn main() {\n";
        let mut iter = ScanIter::new(input);
        let _events: Vec<_> = iter.by_ref().collect();

        let diagnostics = iter.diagnostics();
        let diag = diagnostics.borrow();
        assert_eq!(diag.warnings.len(), 1);

        match &diag.warnings[0] {
            crate::error::ParseWarning::UnclosedCodeFence {
                line,
                marker,
                context,
            } => {
                // Line numbers are 1-indexed for user-friendly error messages
                assert_eq!(line, &1);
                assert_eq!(marker, &'`');
                assert!(context.starts_with("```rust"));
            }
            _ => panic!("Expected UnclosedCodeFence warning"),
        }
    }

    #[test]
    fn scan_no_warning_for_closed_fence() {
        use super::ScanIter;
        let input = "```rust\nfn main() {}\n```\n";
        let mut iter = ScanIter::new(input);
        let _events: Vec<_> = iter.by_ref().collect();

        let diagnostics = iter.diagnostics();
        let diag = diagnostics.borrow();
        assert_eq!(diag.warnings.len(), 0);
    }
}
