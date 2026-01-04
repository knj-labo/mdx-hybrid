#![allow(missing_docs)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    Markdown(&'a str),
    Code(&'a str),
    JsxElement {
        name: &'a str,
        attrs: &'a str,
        children: Vec<Block<'a>>,
        is_self_closing: bool,
    },
}

pub fn scan(input: &str) -> Vec<Block<'_>> {
    let (blocks, _cursor, _closed) = scan_nodes(input, 0, None);
    blocks
}

fn push_markdown<'a>(blocks: &mut Vec<Block<'a>>, input: &'a str, start: usize, end: usize) {
    if start < end {
        blocks.push(Block::Markdown(&input[start..end]));
    }
}

fn scan_nodes<'a>(
    input: &'a str,
    mut cursor: usize,
    until: Option<&'a str>,
) -> (Vec<Block<'a>>, usize, bool) {
    let mut blocks = Vec::new();
    let bytes = input.as_bytes();

    while cursor < bytes.len() {
        let prev_cursor = cursor;
        let line_start = find_line_start(bytes, cursor);
        if is_fence_start(bytes, line_start).is_some() && cursor != line_start {
            cursor = line_start;
        }
        if let Some(tag_name) = until
            && is_close_tag(bytes, cursor, tag_name.as_bytes())
        {
            if let Some(close_end) = find_tag_end(input, cursor + 2 + tag_name.len()) {
                return (blocks, close_end.saturating_add(1), true);
            }
            return (blocks, cursor, false);
        }
        if let Some((marker, count)) = is_fence_start(bytes, cursor) {
            if let Some(fence_end) = find_fence_end(bytes, cursor + 1, marker, count) {
                let end_pos = find_byte(bytes, fence_end, b'\n')
                    .map(|newline| newline + 1)
                    .unwrap_or_else(|| input.len());
                blocks.push(Block::Code(&input[cursor..end_pos]));
                cursor = end_pos;
                continue;
            }
            push_markdown(&mut blocks, input, cursor, cursor + 1);
            cursor += 1;
            if cursor < input.len() {
                push_markdown(&mut blocks, input, cursor, input.len());
            }
            return (blocks, input.len(), false);
        }
        if bytes.get(cursor) == Some(&b'`') {
            if let Some(end) = find_inline_code_end(bytes, cursor) {
                push_markdown(&mut blocks, input, cursor, end);
                cursor = end;
                continue;
            }
        }
        let mut next_lt = find_byte(bytes, cursor, b'<');
        while let Some(pos) = next_lt {
            let line_start = find_line_start(bytes, pos);
            if is_fence_start(bytes, line_start).is_some() {
                next_lt = find_byte(bytes, pos + 1, b'<');
                continue;
            }
            break;
        }
        let next_fence = find_fence_start(bytes, cursor);
        let next_tick = find_byte(bytes, cursor, b'`');
        let mut next_stop: Option<usize> = None;
        for candidate in [next_lt, next_fence, next_tick] {
            if let Some(pos) = candidate {
                next_stop = Some(match next_stop {
                    Some(current) => current.min(pos),
                    None => pos,
                });
            }
        }
        if next_stop.is_none() {
            push_markdown(&mut blocks, input, cursor, input.len());
            return (blocks, input.len(), false);
        }
        let pos = next_stop.unwrap();
        if cursor < pos {
            push_markdown(&mut blocks, input, cursor, pos);
            cursor = pos;
            continue;
        }
        if cursor < bytes.len() && bytes[cursor] == b'<' {
            if let Some((name, attrs, open_end)) = parse_open_tag(input, cursor) {
                let is_self_closing = is_self_closing(bytes, cursor, open_end);
                if is_self_closing {
                    blocks.push(Block::JsxElement {
                        name,
                        attrs,
                        children: Vec::new(),
                        is_self_closing: true,
                    });
                    cursor = open_end.saturating_add(1);
                    continue;
                }
                let (children, new_cursor, closed) = scan_nodes(input, open_end + 1, Some(name));
                if closed {
                    blocks.push(Block::JsxElement {
                        name,
                        attrs,
                        children,
                        is_self_closing: false,
                    });
                    cursor = new_cursor;
                    continue;
                }
                push_markdown(&mut blocks, input, cursor, cursor + 1);
                cursor += 1;
            } else {
                push_markdown(&mut blocks, input, cursor, cursor + 1);
                cursor += 1;
            }
        }
        if cursor == prev_cursor {
            break;
        }
    }
    (blocks, cursor, false)
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

fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    find_byte(input.as_bytes(), start, b'>')
}

fn is_self_closing(bytes: &[u8], open_start: usize, open_end: usize) -> bool {
    let _ = open_start;
    if open_end == 0 {
        return false;
    }
    bytes.get(open_end.saturating_sub(1)) == Some(&b'/')
}

fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_')
}

fn is_tag_terminator(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\t' | b'\r' | b'/' | b'>')
}

fn parse_open_tag(input: &str, open_start: usize) -> Option<(&str, &str, usize)> {
    let bytes = input.as_bytes();
    let name_start = open_start + 1;
    let mut name_end = name_start;

    while name_end < bytes.len() && !is_tag_terminator(bytes[name_end]) {
        if !is_name_char(bytes[name_end]) {
            return None;
        }
        name_end += 1;
    }

    if name_end == name_start {
        return None;
    }

    let open_end = find_tag_end(input, name_end)?;
    let attrs = input[name_end..open_end].trim();
    Some((&input[name_start..name_end], attrs, open_end))
}

fn is_close_tag(bytes: &[u8], pos: usize, name: &[u8]) -> bool {
    if pos + 2 + name.len() > bytes.len() {
        return false;
    }
    if bytes[pos] != b'<' || bytes[pos + 1] != b'/' {
        return false;
    }
    if &bytes[pos + 2..pos + 2 + name.len()] != name {
        return false;
    }
    let end = pos + 2 + name.len();
    bytes.get(end).copied().is_some_and(is_tag_terminator)
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
        if is_line_start(bytes, pos) {
            if let Some(marker_pos) = skip_fence_indent(bytes, pos)
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

    while i < bytes.len() {
        match bytes[i] {
            b' ' if spaces < 4 => {
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

#[cfg(test)]
mod tests {
    use super::{Block, find_fence_end, is_fence_start, scan};

    #[test]
    fn scan_returns_single_markdown_block() {
        let input = "hello";
        let blocks = scan(input);

        assert_eq!(blocks, vec![Block::Markdown(input)]);
    }

    #[test]
    fn scan_empty_returns_empty() {
        let blocks = scan("");

        assert!(blocks.is_empty());
    }

    #[test]
    fn scan_emits_self_closing_jsx_element() {
        let blocks = scan("<Tabs />");

        assert_eq!(
            blocks,
            vec![Block::JsxElement {
                name: "Tabs",
                attrs: "/",
                children: Vec::new(),
                is_self_closing: true,
            }]
        );
    }

    #[test]
    fn scan_emits_jsx_element_with_children() {
        let blocks = scan("<Steps>hello</Steps>");

        assert_eq!(
            blocks,
            vec![Block::JsxElement {
                name: "Steps",
                attrs: "",
                children: vec![Block::Markdown("hello")],
                is_self_closing: false,
            }]
        );
    }

    #[test]
    fn scan_handles_nested_same_tag() {
        let blocks = scan("<Steps><Steps>inner</Steps>outer</Steps>");

        assert_eq!(
            blocks,
            vec![Block::JsxElement {
                name: "Steps",
                attrs: "",
                children: vec![
                    Block::JsxElement {
                        name: "Steps",
                        attrs: "",
                        children: vec![Block::Markdown("inner")],
                        is_self_closing: false,
                    },
                    Block::Markdown("outer"),
                ],
                is_self_closing: false,
            }]
        );
    }

    #[test]
    fn scan_ignores_self_closing_in_nested_match() {
        let blocks = scan("<Steps><Steps />inner</Steps>");

        assert_eq!(
            blocks,
            vec![Block::JsxElement {
                name: "Steps",
                attrs: "",
                children: vec![
                    Block::JsxElement {
                        name: "Steps",
                        attrs: "/",
                        children: Vec::new(),
                        is_self_closing: true,
                    },
                    Block::Markdown("inner"),
                ],
                is_self_closing: false,
            }]
        );
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
        let blocks = scan(input);

        assert_eq!(blocks, vec![Block::Code(input)]);
    }

    #[test]
    fn scan_does_not_parse_jsx_inside_fence() {
        let input = "```\n<Tabs />\n```\n";
        let blocks = scan(input);

        assert_eq!(blocks, vec![Block::Code(input)]);
    }

    #[test]
    fn scan_continues_after_fence() {
        let input = "```\n<Steps>\n```\n<Tabs />";
        let blocks = scan(input);

        assert_eq!(
            blocks,
            vec![
                Block::Code("```\n<Steps>\n```\n"),
                Block::JsxElement {
                    name: "Tabs",
                    attrs: "/",
                    children: Vec::new(),
                    is_self_closing: true,
                },
            ]
        );
    }

    #[test]
    fn scan_falls_back_on_unclosed_fence() {
        let input = "```\n<Steps>";
        let blocks = scan(input);

        assert_eq!(
            blocks,
            vec![Block::Markdown("`"), Block::Markdown("``\n<Steps>")]
        );
    }

    #[test]
    fn scan_falls_back_on_missing_close_tag() {
        let input = "<Steps>no close";
        let blocks = scan(input);

        assert_eq!(
            blocks,
            vec![Block::Markdown("<"), Block::Markdown("Steps>no close")]
        );
    }
}
