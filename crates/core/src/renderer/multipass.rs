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
    scan_range(input, 0, input.len())
}

fn push_markdown<'a>(blocks: &mut Vec<Block<'a>>, input: &'a str, start: usize, end: usize) {
    if start < end {
        blocks.push(Block::Markdown(&input[start..end]));
    }
}

fn scan_range<'a>(input: &'a str, start: usize, end: usize) -> Vec<Block<'a>> {
    let mut blocks = Vec::new();
    if start >= end {
        return blocks;
    }

    let bytes = input.as_bytes();
    let mut cursor = start;
    while cursor < end {
        let prev_cursor = cursor;
        let next_lt = find_byte(bytes, cursor, b'<');
        if next_lt.is_none() {
            push_markdown(&mut blocks, input, cursor, end);
            break;
        }
        let pos = next_lt.unwrap();
        if cursor < pos {
            push_markdown(&mut blocks, input, cursor, pos);
            cursor = pos;
        }
        if cursor < end && bytes[cursor] == b'<' {
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

                if let Some((close_start, close_end)) =
                    find_matching_close_tag(input, name, open_end + 1)
                {
                    let children = scan_range(input, open_end + 1, close_start);
                    blocks.push(Block::JsxElement {
                        name,
                        attrs,
                        children,
                        is_self_closing: false,
                    });
                    cursor = close_end.saturating_add(1);
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
    blocks
}

fn find_byte(bytes: &[u8], start: usize, target: u8) -> Option<usize> {
    bytes.iter().skip(start).position(|&b| b == target).map(|idx| idx + start)
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
    bytes.get(end).copied().map_or(false, is_tag_terminator)
}

fn find_matching_close_tag(
    input: &str,
    name: &str,
    mut search: usize,
) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let name_bytes = name.as_bytes();
    let mut depth = 0usize;

    while search < bytes.len() {
        let pos = find_byte(bytes, search, b'<')?;
        if is_open_tag(bytes, pos, name_bytes) {
            if let Some((_name, _attrs, open_end)) = parse_open_tag(input, pos) {
                if is_self_closing(bytes, pos, open_end) {
                    search = open_end.saturating_add(1);
                    continue;
                }
            }
            depth = depth.saturating_add(1);
            search = pos + 1;
            continue;
        }
        if is_close_tag(bytes, pos, name_bytes) {
            if depth == 0 {
                let close_end = find_tag_end(input, pos + 2 + name.len())?;
                return Some((pos, close_end));
            }
            depth = depth.saturating_sub(1);
        }
        search = pos + 1;
    }

    None
}

fn is_open_tag(bytes: &[u8], pos: usize, name: &[u8]) -> bool {
    if pos + 1 + name.len() >= bytes.len() {
        return false;
    }
    if bytes[pos] != b'<' || bytes[pos + 1] == b'/' {
        return false;
    }
    if &bytes[pos + 1..pos + 1 + name.len()] != name {
        return false;
    }
    let end = pos + 1 + name.len();
    bytes.get(end).copied().map_or(false, is_tag_terminator)
}

#[cfg(test)]
mod tests {
    use super::{Block, scan};

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
}
