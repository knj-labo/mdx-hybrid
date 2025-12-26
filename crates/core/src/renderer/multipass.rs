#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block<'a> {
    Markdown(&'a str),
    JsxElement {
        name: &'a str,
        open: &'a str,
        children: Vec<Block<'a>>,
        close: &'a str,
    },
    JsxSelfClosing {
        raw: &'a str,
    },
}

pub(crate) fn scan_blocks(input: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
    let mut cursor = 0;
    let bytes = input.as_bytes();
    let fence_ranges = code_fence_ranges(input);
    let inline_code_ranges = inline_code_ranges(input);
    let skip_ranges = merge_ranges(&fence_ranges, &inline_code_ranges);

    while let Some(open_start) = find_next_upper_tag(bytes, cursor, &skip_ranges, input) {
        if open_start > cursor {
            blocks.push(Block::Markdown(&input[cursor..open_start]));
        }

        let Some((name, open_end)) = parse_open_tag(input, open_start) else {
            blocks.push(Block::Markdown(&input[open_start..]));
            return blocks;
        };

        let open = &input[open_start..open_end];
        if is_self_closing(bytes, open_start, open_end) {
            blocks.push(Block::JsxSelfClosing { raw: open });
            cursor = open_end;
            continue;
        }

        let Some((close_start, close_end)) = find_matching_close(input, name, open_end) else {
            blocks.push(Block::Markdown(&input[open_start..]));
            return blocks;
        };

        let inner = &input[open_end..close_start];
        let children = scan_blocks(inner);

        blocks.push(Block::JsxElement {
            name,
            open,
            children,
            close: &input[close_start..close_end],
        });
        cursor = close_end;
    }

    if cursor < input.len() {
        blocks.push(Block::Markdown(&input[cursor..]));
    }

    blocks
}

fn find_next_upper_tag(
    bytes: &[u8],
    start: usize,
    skip_ranges: &[(usize, usize)],
    input: &str,
) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if let Some(end) = fence_range_end(skip_ranges, i) {
            i = end;
            continue;
        }
        if bytes[i] == b'<'
            && bytes[i + 1].is_ascii_uppercase()
            && is_line_start_within_indent(input, i, 3)
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn is_line_start_within_indent(input: &str, pos: usize, max_indent: usize) -> bool {
    let bytes = input.as_bytes();
    let mut idx = pos;
    while idx > 0 {
        let prev = bytes[idx - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        idx -= 1;
    }
    let line = &input[idx..pos];
    let mut count = 0;
    for ch in line.chars() {
        if ch == ' ' {
            count += 1;
            if count > max_indent {
                return false;
            }
            continue;
        }
        if ch == '\t' {
            return false;
        }
        return false;
    }
    true
}

fn fence_range_end(ranges: &[(usize, usize)], pos: usize) -> Option<usize> {
    for (start, end) in ranges {
        if pos >= *start && pos < *end {
            return Some(*end);
        }
    }
    None
}

fn code_fence_ranges(input: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = "";
    let mut offset = 0;

    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(marker) = code_fence_marker_line(trimmed) {
            if in_fence {
                if marker == fence_marker {
                    ranges.push((offset, offset + line.len()));
                    in_fence = false;
                    fence_marker = "";
                    offset += line.len();
                    continue;
                }
            } else {
                in_fence = true;
                fence_marker = marker;
                ranges.push((offset, offset + line.len()));
                offset += line.len();
                continue;
            }
        }

        if in_fence {
            ranges.push((offset, offset + line.len()));
        }
        offset += line.len();
    }

    ranges
}

fn code_fence_marker_line(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn inline_code_ranges(input: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open_tick: Option<(usize, usize)> = None;
    let mut idx = 0;
    let bytes = input.as_bytes();

    while idx < bytes.len() {
        if bytes[idx] == b'`' {
            let run_start = idx;
            let mut run_len = 1;
            while idx + run_len < bytes.len() && bytes[idx + run_len] == b'`' {
                run_len += 1;
            }
            if let Some((open_pos, open_len)) = open_tick {
                if open_len == run_len {
                    let end = run_start + run_len;
                    ranges.push((open_pos, end));
                    open_tick = None;
                    idx = end;
                    continue;
                }
            } else {
                open_tick = Some((run_start, run_len));
                idx = run_start + run_len;
                continue;
            }
        }
        idx += 1;
    }

    ranges
}

fn merge_ranges(a: &[(usize, usize)], b: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = a.iter().chain(b.iter()).copied().collect();
    ranges.sort_by_key(|(start, _)| *start);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if let Some((_last_start, last_end)) = merged.last_mut() {
            if start <= *last_end {
                if end > *last_end {
                    *last_end = end;
                }
                continue;
            }
            if start <= *last_end + 1 {
                if end > *last_end {
                    *last_end = end;
                }
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn parse_open_tag(input: &str, open_start: usize) -> Option<(&str, usize)> {
    let bytes = input.as_bytes();
    let mut name_end = open_start + 1;
    while name_end < bytes.len() && is_name_char(bytes[name_end]) {
        name_end += 1;
    }
    if name_end == open_start + 1 {
        return None;
    }
    let name = &input[open_start + 1..name_end];
    let open_end = find_tag_end(input, name_end)?;
    Some((name, open_end))
}

fn find_matching_close(input: &str, name: &str, mut search: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut depth = 1usize;
    let name_bytes = name.as_bytes();

    while let Some(lt) = find_byte(bytes, search, b'<') {
        if is_close_tag(bytes, lt, name_bytes) {
            let close_end = find_tag_end(input, lt + 2 + name_bytes.len())?;
            depth -= 1;
            if depth == 0 {
                return Some((lt, close_end));
            }
            search = close_end;
            continue;
        }

        if is_open_tag(bytes, lt, name_bytes) {
            let open_end = find_tag_end(input, lt + 1 + name_bytes.len())?;
            if !is_self_closing(bytes, lt, open_end) {
                depth += 1;
            }
            search = open_end;
            continue;
        }

        search = lt + 1;
    }
    None
}

fn is_open_tag(bytes: &[u8], pos: usize, name: &[u8]) -> bool {
    if pos + 1 + name.len() > bytes.len() {
        return false;
    }
    if bytes[pos] != b'<' {
        return false;
    }
    if bytes[pos + 1..].starts_with(name) {
        let next = pos + 1 + name.len();
        return next < bytes.len() && is_tag_terminator(bytes[next]);
    }
    false
}

fn is_close_tag(bytes: &[u8], pos: usize, name: &[u8]) -> bool {
    if pos + 2 + name.len() > bytes.len() {
        return false;
    }
    if bytes[pos] != b'<' || bytes[pos + 1] != b'/' {
        return false;
    }
    if bytes[pos + 2..].starts_with(name) {
        let next = pos + 2 + name.len();
        return next < bytes.len() && is_tag_terminator(bytes[next]);
    }
    false
}

fn is_tag_terminator(byte: u8) -> bool {
    byte == b'>' || byte == b'/' || byte.is_ascii_whitespace()
}

fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut i = start;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
        } else if b == b'\'' || b == b'"' {
            quote = Some(b);
        } else if b == b'>' {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn is_self_closing(bytes: &[u8], open_start: usize, open_end: usize) -> bool {
    if open_end <= open_start + 1 {
        return false;
    }
    let mut i = open_end - 2;
    while i > open_start && bytes[i].is_ascii_whitespace() {
        i -= 1;
    }
    bytes[i] == b'/'
}

fn find_byte(bytes: &[u8], start: usize, target: u8) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == target {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Block, scan_blocks};

    #[test]
    fn scan_blocks_markdown_only() {
        let input = "Just markdown\n\nSecond line.";
        let blocks = scan_blocks(input);
        assert_eq!(blocks, vec![Block::Markdown(input)]);
    }

    #[test]
    fn scan_blocks_self_closing_jsx() {
        let input = "<Tabs />\n";
        let blocks = scan_blocks(input);
        assert_eq!(
            blocks,
            vec![
                Block::JsxSelfClosing { raw: "<Tabs />" },
                Block::Markdown("\n"),
            ]
        );
    }

    #[test]
    fn scan_blocks_nested_same_tag() {
        let input = "<Steps>\n<Steps>Inner</Steps>\n</Steps>";
        let blocks = scan_blocks(input);
        assert_eq!(
            blocks,
            vec![Block::JsxElement {
                name: "Steps",
                open: "<Steps>",
                children: vec![
                    Block::Markdown("\n"),
                    Block::JsxElement {
                        name: "Steps",
                        open: "<Steps>",
                        children: vec![Block::Markdown("Inner")],
                        close: "</Steps>",
                    },
                    Block::Markdown("\n"),
                ],
                close: "</Steps>",
            }]
        );
    }

    #[test]
    fn scan_blocks_ignores_fenced_jsx() {
        let input = "```\n<Steps>\n```\n<Tabs />\n";
        let blocks = scan_blocks(input);
        assert_eq!(
            blocks,
            vec![
                Block::Markdown("```\n<Steps>\n```\n"),
                Block::JsxSelfClosing { raw: "<Tabs />" },
                Block::Markdown("\n"),
            ]
        );
    }

    #[test]
    fn scan_blocks_ignores_inline_code_jsx() {
        let input = "`<BUCKET_NAME>`\n<Tabs />\n";
        let blocks = scan_blocks(input);
        assert_eq!(
            blocks,
            vec![
                Block::Markdown("`<BUCKET_NAME>`\n"),
                Block::JsxSelfClosing { raw: "<Tabs />" },
                Block::Markdown("\n"),
            ]
        );
    }

    #[test]
    fn scan_blocks_ignores_indented_jsx() {
        let input = "1. item\n    <Tabs />\n";
        let blocks = scan_blocks(input);
        assert_eq!(blocks, vec![Block::Markdown(input)]);
    }
}
