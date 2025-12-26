#![allow(missing_docs)]
use crate::event::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use crate::renderer::multipass::{Block, scan_blocks};
use crate::{
    DirectiveAdapter, HoistAdapter, MarkflowError, ParseResult, RewriteOptions, get_event_iterator,
    parse,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Render Markdown input into a raw JSX-like string, preserving JSX nodes.
pub fn render_to_jsx(input: &str) -> Result<String, MarkflowError> {
    let ParseResult { html: _, imports } = parse(input)?;
    let mut output = String::new();

    let mut seen_imports = std::collections::HashSet::new();
    for import in imports {
        if seen_imports.insert(import.source.clone()) {
            output.push_str(&import.source);
            output.push('\n');
        }
    }

    let preprocessed = preprocess_jsx_block_lines(input)?;
    output.push_str(&render_to_jsx_body(&preprocessed.body, false)?);
    let mut restored = output;
    for (idx, replacement) in preprocessed.replacements.iter().enumerate() {
        let token = format!("<mf-block data-mf-idx=\"{}\"></mf-block>", idx);
        restored = restored.replace(&token, replacement);
    }
    Ok(restored)
}

fn render_to_jsx_body(input: &str, _preprocess_jsx_blocks: bool) -> Result<String, MarkflowError> {
    let input = input.to_string();
    let rewrite_options = RewriteOptions::default();
    let hoisted = Rc::new(RefCell::new(Vec::new()));
    let mut events: Box<dyn Iterator<Item = Event<'static>>> =
        Box::new(get_event_iterator(&input)?);
    if rewrite_options.enable_hoist {
        events = Box::new(HoistAdapter::new(events, Rc::clone(&hoisted)));
    }
    let events = DirectiveAdapter::new(
        events,
        Rc::new(RefCell::new(0)),
        rewrite_options.directive_mapper.clone(),
        rewrite_options.required_imports.clone(),
    );
    let mut output = String::new();

    let mut stack: Vec<Tag<'_>> = Vec::new();
    let mut heading_stack: Vec<HeadingContext> = Vec::new();

    for event in events {
        match event {
            Event::Start(tag) => {
                if let Tag::Heading { level, id, .. } = &tag {
                    if let Some(id) = id.as_ref() {
                        output.push_str(&heading_wrapper_start(*level));
                        heading_stack.push(HeadingContext {
                            id: id.to_string(),
                            text: String::new(),
                        });
                    }
                }
                output.push_str(&start_tag(&tag));
                stack.push(tag);
            }
            Event::End(end) => {
                if let Some(open) = stack.pop()
                    && matches_end(&open, &end)
                {
                    output.push_str(&end_tag(&end));
                    if let Tag::Heading { id, .. } = &open {
                        if let Some(id) = id.as_ref() {
                            let heading_text =
                                heading_stack.pop().map(|ctx| ctx.text).unwrap_or_default();
                            output.push_str(&anchor_link(id.as_ref(), &heading_text));
                            output.push_str("</div>");
                        }
                    }
                }
            }
            Event::Text(text) => {
                output.push_str(&escape_text(text.as_ref()));
                push_heading_text(&mut heading_stack, text.as_ref());
            }
            Event::Code(text) => {
                output.push_str("<code>");
                output.push_str(&escape_text(text.as_ref()));
                output.push_str("</code>");
                push_heading_text(&mut heading_stack, text.as_ref());
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                output.push_str(text.as_ref());
            }
            Event::JsxInline(text) => {
                output.push_str(text.as_ref());
            }
            Event::JsxFlow(text) => {
                output.push_str(text.as_ref());
            }
            Event::InlineMath(math) => {
                output.push_str(&format!("<span class=\"math-inline\">{}</span>", math));
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::DisplayMath(math) => {
                output.push_str(&format!("<div class=\"math-display\">{}</div>", math));
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::FootnoteReference(label) => {
                output.push_str(&format!(
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{0}\" id=\"fnref-{0}\">{0}</a></sup>",
                    label
                ));
                push_heading_text(&mut heading_stack, label.as_ref());
            }
            Event::TaskListMarker(done) => {
                if done {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" checked=\"\" />");
                } else {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" />");
                }
            }
            Event::Rule => output.push_str("<hr />\n"),
            Event::HardBreak => {
                output.push_str("<br />\n");
                push_heading_text(&mut heading_stack, " ");
            }
            Event::SoftBreak => {
                output.push('\n');
                push_heading_text(&mut heading_stack, " ");
            }
        }
    }

    Ok(output)
}

struct JsxOpening {
    name: String,
    open_start: usize,
    open_end: usize,
    self_closing: bool,
}

struct JsxBlockState {
    name: String,
    buffer: String,
}

struct JsxPreprocessResult {
    body: String,
    replacements: Vec<String>,
}

fn preprocess_jsx_block_lines(input: &str) -> Result<JsxPreprocessResult, MarkflowError> {
    let mut output = String::new();
    let mut replacements: Vec<String> = Vec::new();
    let mut in_code_fence = false;
    let mut fence_marker = "";
    let mut block: Option<JsxBlockState> = None;

    for line in input.split_inclusive('\n') {
        let (line_text, line_ending) = split_line_ending(line);
        let line_text =
            normalize_code_fence_line(line_text).unwrap_or_else(|| line_text.to_string());
        let trimmed = line_text.trim_start();

        if let Some(marker) = code_fence_marker(trimmed) {
            if in_code_fence {
                if marker == fence_marker {
                    in_code_fence = false;
                    fence_marker = "";
                }
            } else {
                in_code_fence = true;
                fence_marker = marker;
            }
        }

        if let Some(state) = block.as_mut() {
            if !in_code_fence && is_jsx_closing_line(trimmed, &state.name) {
                let rendered = if state.name == "Steps" {
                    let blocks = scan_blocks(&state.buffer);
                    render_steps_children(&blocks)?
                } else {
                    render_blocks(&state.buffer)?
                };
                let idx = replacements.len();
                replacements.push(rendered);
                output.push_str(&format!("<mf-block data-mf-idx=\"{}\"></mf-block>", idx));
                output.push_str(&line_text);
                output.push_str(line_ending);
                block = None;
                continue;
            }

            state.buffer.push_str(&line_text);
            state.buffer.push_str(line_ending);
            continue;
        }

        if !in_code_fence {
            if let Some(name) = parse_jsx_opening_line(trimmed) {
                output.push_str(&line_text);
                output.push_str(line_ending);
                block = Some(JsxBlockState {
                    name,
                    buffer: String::new(),
                });
                continue;
            }
        }

        output.push_str(&line_text);
        output.push_str(line_ending);
    }

    if let Some(state) = block {
        let rendered = if state.name == "Steps" {
            let blocks = scan_blocks(&state.buffer);
            render_steps_children(&blocks)?
        } else {
            render_blocks(&state.buffer)?
        };
        let idx = replacements.len();
        replacements.push(rendered);
        output.push_str(&format!("<mf-block data-mf-idx=\"{}\"></mf-block>", idx));
    }

    Ok(JsxPreprocessResult {
        body: output,
        replacements,
    })
}

fn render_blocks(input: &str) -> Result<String, MarkflowError> {
    let blocks = scan_blocks(input);
    render_block_list(&blocks)
}

fn render_block_list(blocks: &[Block<'_>]) -> Result<String, MarkflowError> {
    let mut output = String::new();
    for block in blocks {
        match block {
            Block::Markdown(text) => {
                if text.is_empty() {
                    continue;
                }
                output.push_str(&render_markdown_with_inline_jsx(text)?);
            }
            Block::JsxElement {
                name,
                open,
                children,
                close,
            } => {
                if *name == "Steps" {
                    let rendered = render_steps_children(children)?;
                    output.push_str(open);
                    output.push_str(&rendered);
                    output.push_str(close);
                } else {
                    let rendered = render_block_list(children)?;
                    output.push_str(open);
                    output.push_str(&rendered);
                    output.push_str(close);
                }
            }
            Block::JsxSelfClosing { raw } => {
                output.push_str(raw);
            }
        }
    }
    Ok(output)
}

fn render_steps_children(children: &[Block<'_>]) -> Result<String, MarkflowError> {
    let mut rendered = String::new();

    for child in children {
        match child {
            Block::Markdown(text) => {
                if text.is_empty() {
                    continue;
                }
                rendered.push_str(&render_markdown_with_inline_jsx(text)?);
            }
            _ => {
                let child_rendered = render_block_list(std::slice::from_ref(child))?;
                if child_rendered.is_empty() {
                    continue;
                }
                if let Some(insert_pos) = rendered.rfind("</li>") {
                    rendered.insert_str(insert_pos, &child_rendered);
                } else {
                    rendered.push_str(&child_rendered);
                }
            }
        }
    }

    Ok(normalize_steps_list_items(&rendered))
}

fn render_markdown_with_inline_jsx(input: &str) -> Result<String, MarkflowError> {
    let original = input.to_string();
    let mut output = String::new();
    let mut markdown_buf = String::new();
    let mut in_code_fence = false;
    let mut fence_marker = "";
    let mut placeholders: Vec<String> = Vec::new();

    for line in input.split_inclusive('\n') {
        let (line_text, line_ending) = split_line_ending(line);
        let trimmed = line_text.trim_start();

        if let Some(marker) = code_fence_marker(trimmed) {
            if in_code_fence {
                if marker == fence_marker {
                    in_code_fence = false;
                    fence_marker = "";
                }
            } else {
                in_code_fence = true;
                fence_marker = marker;
            }
        }

        if !in_code_fence && is_jsx_line(trimmed) {
            let idx = placeholders.len();
            placeholders.push(format!("{}{}", line_text, line_ending));
            let indent_len = line_text.len().saturating_sub(trimmed.len());
            let indent = &line_text[..indent_len];
            markdown_buf.push_str(indent);
            markdown_buf.push_str(&jsx_placeholder(idx));
            markdown_buf.push_str(line_ending);
            continue;
        }

        markdown_buf.push_str(line_text);
        markdown_buf.push_str(line_ending);
    }

    if flush_markdown_buffer(&mut output, &mut markdown_buf).is_err() {
        return Ok(original);
    }
    if placeholders.is_empty() {
        return Ok(output);
    }

    let mut restored = strip_placeholder_paragraphs(&output, placeholders.len());
    for (idx, original) in placeholders.iter().enumerate() {
        let token = jsx_placeholder(idx);
        restored = restored.replace(&token, original);
    }

    Ok(normalize_steps_blocks(&strip_jsx_block_paragraphs(
        &restored,
    )))
}

fn strip_placeholder_paragraphs(input: &str, placeholder_count: usize) -> String {
    let mut output = input.to_string();
    for idx in 0..placeholder_count {
        let token = jsx_placeholder(idx);
        output = strip_single_placeholder_paragraph(&output, &token);
    }
    output
}

fn strip_single_placeholder_paragraph(input: &str, token: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(p_start_rel) = input[cursor..].find("<p>") {
        let p_start = cursor + p_start_rel;
        let content_start = p_start + 3;
        let Some(p_end_rel) = input[content_start..].find("</p>") else {
            break;
        };
        let p_end = content_start + p_end_rel;
        let content = &input[content_start..p_end];

        result.push_str(&input[cursor..p_start]);
        if content.trim() == token {
            result.push_str(token);
        } else {
            result.push_str("<p>");
            result.push_str(content);
            result.push_str("</p>");
        }
        cursor = p_end + 4;
    }

    result.push_str(&input[cursor..]);
    result
}

fn strip_jsx_block_paragraphs(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(p_start_rel) = input[cursor..].find("<p>") {
        let p_start = cursor + p_start_rel;
        let content_start = p_start + 3;
        let Some(p_end_rel) = input[content_start..].find("</p>") else {
            break;
        };
        let p_end = content_start + p_end_rel;
        let content = &input[content_start..p_end];

        result.push_str(&input[cursor..p_start]);
        if is_jsx_block_paragraph(content) {
            result.push_str(content);
        } else {
            result.push_str("<p>");
            result.push_str(content);
            result.push_str("</p>");
        }
        cursor = p_end + 4;
    }

    result.push_str(&input[cursor..]);
    result
}

fn normalize_steps_list_items(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();

    while let Some(li_rel) = html[cursor..].find("<li") {
        let li_start = cursor + li_rel;
        output.push_str(&html[cursor..li_start]);

        let after_li = li_start + 3;
        if after_li >= bytes.len() {
            output.push_str(&html[li_start..]);
            return output;
        }
        let next = bytes[after_li];
        if next != b'>' && !next.is_ascii_whitespace() {
            output.push_str(&html[li_start..after_li]);
            cursor = after_li;
            continue;
        }

        let Some(open_end) = find_tag_end(html, after_li) else {
            output.push_str(&html[li_start..]);
            return output;
        };
        let open_tag = &html[li_start..open_end];

        let Some((close_start, close_end)) = find_matching_li_close(html, open_end) else {
            output.push_str(&html[li_start..]);
            return output;
        };
        let content = &html[open_end..close_start];
        let trimmed = content.trim_start();

        output.push_str(open_tag);
        if trimmed.starts_with("<p") || trimmed.starts_with('<') || trimmed.is_empty() {
            output.push_str(content);
        } else {
            output.push_str("<p>");
            output.push_str(content);
            output.push_str("</p>");
        }
        output.push_str(&html[close_start..close_end]);
        cursor = close_end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn normalize_steps_blocks(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();

    while let Some(open_rel) = html[cursor..].find("<ol") {
        let open_start = cursor + open_rel;
        output.push_str(&html[cursor..open_start]);

        let after_ol = open_start + 3;
        if after_ol >= bytes.len()
            || !(bytes[after_ol] == b'>' || bytes[after_ol].is_ascii_whitespace())
        {
            output.push_str(&html[open_start..after_ol]);
            cursor = after_ol;
            continue;
        }

        let Some(open_end) = find_tag_end(html, after_ol) else {
            output.push_str(&html[open_start..]);
            return output;
        };

        let open_tag = &html[open_start..open_end];
        if !is_steps_ol(open_tag) {
            output.push_str(open_tag);
            cursor = open_end;
            continue;
        }

        let Some((close_start, close_end)) = find_matching_ol_close(html, open_end) else {
            output.push_str(&html[open_start..]);
            return output;
        };

        let inner = &html[open_end..close_start];
        let close_tag = &html[close_start..close_end];

        output.push_str(open_tag);
        output.push_str(&normalize_steps_list_items(inner));
        output.push_str(close_tag);

        cursor = close_end;
    }

    output.push_str(&html[cursor..]);
    output
}

fn is_steps_ol(open_tag: &str) -> bool {
    let Some(class_value) = extract_attribute_value(open_tag, "class") else {
        return false;
    };
    class_value
        .split_whitespace()
        .any(|token| token == "sl-steps")
}

fn extract_attribute_value<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=");
    let start = tag.find(&needle)?;
    let mut i = start + needle.len();
    let bytes = tag.as_bytes();
    if i >= bytes.len() {
        return None;
    }
    let quote = bytes[i];
    if quote == b'"' || quote == b'\'' {
        i += 1;
        let rest = &tag[i..];
        let end = rest.find(quote as char)?;
        return Some(&tag[i..i + end]);
    }
    let rest = &tag[i..];
    let end = rest
        .find(|c: char| c.is_ascii_whitespace() || c == '>')
        .unwrap_or(rest.len());
    Some(&tag[i..i + end])
}

fn find_matching_ol_close(input: &str, mut search: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut depth = 1usize;

    while let Some(lt) = find_byte(bytes, search, b'<') {
        if is_close_ol(bytes, lt) {
            let close_end = find_tag_end(input, lt + 4)?;
            depth -= 1;
            if depth == 0 {
                return Some((lt, close_end));
            }
            search = close_end;
            continue;
        }
        if is_open_ol(bytes, lt) {
            let open_end = find_tag_end(input, lt + 3)?;
            depth += 1;
            search = open_end;
            continue;
        }
        search = lt + 1;
    }

    None
}

fn is_open_ol(bytes: &[u8], idx: usize) -> bool {
    if !bytes[idx..].starts_with(b"<ol") {
        return false;
    }
    let next = bytes.get(idx + 3).copied();
    matches!(
        next,
        Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
    )
}

fn is_close_ol(bytes: &[u8], idx: usize) -> bool {
    if !bytes[idx..].starts_with(b"</ol") {
        return false;
    }
    let next = bytes.get(idx + 4).copied();
    matches!(
        next,
        Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
    )
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

fn find_matching_li_close(input: &str, mut search: usize) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let mut depth = 1usize;

    while let Some(lt) = find_byte(bytes, search, b'<') {
        if is_close_li(bytes, lt) {
            let close_end = find_tag_end(input, lt + 4)?;
            depth -= 1;
            if depth == 0 {
                return Some((lt, close_end));
            }
            search = close_end;
            continue;
        }

        if is_open_li(bytes, lt) {
            let open_end = find_tag_end(input, lt + 3)?;
            depth += 1;
            search = open_end;
            continue;
        }

        search = lt + 1;
    }

    None
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

fn is_open_li(bytes: &[u8], pos: usize) -> bool {
    if pos + 3 > bytes.len() {
        return false;
    }
    if bytes[pos] != b'<' || bytes[pos + 1] != b'l' || bytes[pos + 2] != b'i' {
        return false;
    }
    let next = pos + 3;
    if next >= bytes.len() {
        return false;
    }
    bytes[next].is_ascii_whitespace() || bytes[next] == b'>'
}

fn is_close_li(bytes: &[u8], pos: usize) -> bool {
    if pos + 5 > bytes.len() {
        return false;
    }
    bytes[pos..pos + 5].starts_with(b"</li>")
}

fn jsx_placeholder(idx: usize) -> String {
    format!("@@MF_JSX_{}@@", idx)
}

fn is_jsx_block_paragraph(content: &str) -> bool {
    let trimmed = content.trim_start();
    let trimmed = match trimmed.strip_prefix('<') {
        Some(rest) => rest,
        None => return false,
    };
    let trimmed = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let mut name = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            name.push(ch);
            continue;
        }
        break;
    }
    if name.is_empty() {
        return false;
    }
    if name == "Fragment" {
        return true;
    }
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn normalize_code_fence_line(line_text: &str) -> Option<String> {
    let trimmed = line_text.trim_start();
    let indent_len = line_text.len().saturating_sub(trimmed.len());
    let indent = &line_text[..indent_len];
    let marker = if trimmed.starts_with("```") {
        "```"
    } else if trimmed.starts_with("~~~") {
        "~~~"
    } else {
        return None;
    };

    let mut rest = trimmed[marker.len()..].trim();
    if rest.is_empty() {
        return Some(format!("{indent}{marker}"));
    }
    if let Some(first) = rest.split_whitespace().next() {
        rest = first;
    }
    Some(format!("{indent}{marker}{rest}"))
}

fn flush_markdown_buffer(output: &mut String, buffer: &mut String) -> Result<(), MarkflowError> {
    if buffer.is_empty() {
        return Ok(());
    }
    let rendered = render_to_jsx_body(buffer, false)?;
    output.push_str(&rendered);
    buffer.clear();
    Ok(())
}

fn is_jsx_line(trimmed: &str) -> bool {
    if !trimmed.starts_with('<') {
        return false;
    }
    parse_jsx_opening_line(trimmed).is_some()
        || trimmed.starts_with("</")
        || trimmed.starts_with("<Fragment")
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(stripped) = line.strip_suffix("\r\n") {
        return (stripped, "\r\n");
    }
    if let Some(stripped) = line.strip_suffix('\n') {
        return (stripped, "\n");
    }
    (line, "")
}

fn code_fence_marker(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn parse_jsx_opening_line(trimmed: &str) -> Option<String> {
    let opening = parse_jsx_opening(trimmed)?;
    if opening.open_start != 0 || opening.self_closing {
        return None;
    }
    if !trimmed[opening.open_end..].trim().is_empty() {
        return None;
    }
    Some(opening.name)
}

fn is_jsx_closing_line(trimmed: &str, name: &str) -> bool {
    let prefix = format!("</{}", name);
    if !trimmed.starts_with(&prefix) {
        return false;
    }
    let rest = &trimmed[prefix.len()..];
    let end = match rest.find('>') {
        Some(idx) => idx,
        None => return false,
    };
    rest[end + 1..].trim().is_empty()
}

fn parse_jsx_opening(input: &str) -> Option<JsxOpening> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'<' {
        return None;
    }
    if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
        return None;
    }

    let name_start = i + 1;
    let mut name_end = name_start;
    while name_end < bytes.len() {
        let ch = bytes[name_end] as char;
        if ch.is_alphanumeric() || ch == '_' || ch == ':' || ch == '-' || ch == '.' {
            name_end += 1;
        } else {
            break;
        }
    }
    if name_end == name_start {
        return None;
    }

    let name = &input[name_start..name_end];
    let is_component = name == "Fragment"
        || name
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false);
    if !is_component {
        return None;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut j = name_end;
    while j < bytes.len() {
        let ch = bytes[j] as char;
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '>' if !in_single && !in_double => {
                j += 1;
                break;
            }
            _ => {}
        }
        j += 1;
    }
    if j > bytes.len() {
        return None;
    }

    let open_end = j;
    let self_closing = input[i..open_end].trim_end().ends_with("/>");

    Some(JsxOpening {
        name: name.to_string(),
        open_start: i,
        open_end,
        self_closing,
    })
}

fn start_tag(tag: &Tag<'_>) -> String {
    match tag {
        Tag::Paragraph => "<p>".to_string(),
        Tag::Heading {
            level,
            id,
            classes,
            attrs,
        } => {
            let mut s = format!("<h{}", *level as u8);
            if let Some(id) = id {
                s.push_str(&format!(" id=\"{}\"", id));
            }
            if !classes.is_empty() {
                s.push_str(" class=\"");
                for (idx, class) in classes.iter().enumerate() {
                    if idx > 0 {
                        s.push(' ');
                    }
                    s.push_str(class);
                }
                s.push('"');
            }
            for (key, value) in attrs {
                match value {
                    Some(v) => s.push_str(&format!(" {}=\"{}\"", key, v)),
                    None => s.push_str(&format!(" {}", key)),
                }
            }
            s.push('>');
            s
        }
        Tag::BlockQuote => "<blockquote>".to_string(),
        Tag::CodeBlock(kind) => match kind {
            CodeBlockKind::Indented => "<pre><code>".to_string(),
            CodeBlockKind::Fenced(lang) => format!("<pre><code class=\"language-{}\">", lang),
        },
        Tag::List(start) => {
            if start.is_some() {
                "<ol>".to_string()
            } else {
                "<ul>".to_string()
            }
        }
        Tag::Item => "<li>".to_string(),
        Tag::FootnoteDefinition(label) => {
            format!("<section class=\"footnote\" id=\"fn-{}\">", label)
        }
        Tag::Table(_) => "<table>".to_string(),
        Tag::TableHead => "<thead>".to_string(),
        Tag::TableRow => "<tr>".to_string(),
        Tag::TableCell => "<td>".to_string(),
        Tag::Emphasis => "<em>".to_string(),
        Tag::Strong => "<strong>".to_string(),
        Tag::Strikethrough => "<del>".to_string(),
        Tag::Link {
            dest_url, title, ..
        } => {
            let mut s = format!("<a href=\"{}\"", dest_url);
            if !title.is_empty() {
                s.push_str(&format!(" title=\"{}\"", title));
            }
            s.push('>');
            s
        }
        Tag::Image { .. } => String::new(), // handled elsewhere in HTML renderer; skip here
    }
}

fn end_tag(end: &TagEnd) -> String {
    match end {
        TagEnd::Paragraph => "</p>\n".to_string(),
        TagEnd::Heading(level) => format!("</h{}>", *level as u8),
        TagEnd::BlockQuote => "</blockquote>\n".to_string(),
        TagEnd::CodeBlock => "</code></pre>\n".to_string(),
        TagEnd::List(ordered) => {
            if *ordered {
                "</ol>\n".to_string()
            } else {
                "</ul>\n".to_string()
            }
        }
        TagEnd::Item => "</li>".to_string(),
        TagEnd::FootnoteDefinition => "</section>\n".to_string(),
        TagEnd::Table => "</table>\n".to_string(),
        TagEnd::TableHead => "</thead>\n".to_string(),
        TagEnd::TableRow => "</tr>\n".to_string(),
        TagEnd::TableCell => "</td>".to_string(),
        TagEnd::Emphasis => "</em>".to_string(),
        TagEnd::Strong => "</strong>".to_string(),
        TagEnd::Strikethrough => "</del>".to_string(),
        TagEnd::Link => "</a>".to_string(),
        TagEnd::Image => String::new(),
    }
}

fn matches_end(tag: &Tag<'_>, end: &TagEnd) -> bool {
    matches!(
        (tag, end),
        (Tag::Paragraph, TagEnd::Paragraph)
            | (Tag::Heading { .. }, TagEnd::Heading(_))
            | (Tag::BlockQuote, TagEnd::BlockQuote)
            | (Tag::CodeBlock(_), TagEnd::CodeBlock)
            | (Tag::List(_), TagEnd::List(_))
            | (Tag::Item, TagEnd::Item)
            | (Tag::FootnoteDefinition(_), TagEnd::FootnoteDefinition)
            | (Tag::Table(_), TagEnd::Table)
            | (Tag::TableHead, TagEnd::TableHead)
            | (Tag::TableRow, TagEnd::TableRow)
            | (Tag::TableCell, TagEnd::TableCell)
            | (Tag::Emphasis, TagEnd::Emphasis)
            | (Tag::Strong, TagEnd::Strong)
            | (Tag::Strikethrough, TagEnd::Strikethrough)
            | (Tag::Link { .. }, TagEnd::Link)
            | (Tag::Image { .. }, TagEnd::Image)
    )
}

struct HeadingContext {
    id: String,
    text: String,
}

fn heading_wrapper_start(level: HeadingLevel) -> String {
    format!("<div class=\"sl-heading-wrapper level-h{}\">", level as u8)
}

fn anchor_link(id: &str, heading_text: &str) -> String {
    let label = format!("Section titled \"{}\"", heading_text.trim());
    format!(
        "<a class=\"sl-anchor-link\" href=\"#{id}\"><span aria-hidden=\"true\" class=\"sl-anchor-icon\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\"><path fill=\"currentcolor\" d=\"m12.11 15.39-3.88 3.88a2.52 2.52 0 0 1-3.5 0 2.47 2.47 0 0 1 0-3.5l3.88-3.88a1 1 0 0 0-1.42-1.42l-3.88 3.89a4.48 4.48 0 0 0 6.33 6.33l3.89-3.88a1 1 0 1 0-1.42-1.42Zm8.58-12.08a4.49 4.49 0 0 0-6.33 0l-3.89 3.88a1 1 0 0 0 1.42 1.42l3.88-3.88a2.52 2.52 0 0 1 3.5 0 2.47 2.47 0 0 1 0 3.5l-3.88 3.88a1 1 0 1 0 1.42 1.42l3.88-3.89a4.49 4.49 0 0 0 0-6.33ZM8.83 15.17a1 1 0 0 0 1.1.22 1 1 0 0 0 .32-.22l4.92-4.92a1 1 0 0 0-1.42-1.42l-4.92 4.92a1 1 0 0 0 0 1.42Z\"></path></svg></span><span class=\"sr-only\">{}</span></a>",
        escape_text(&label)
    )
}

fn push_heading_text(heading_stack: &mut Vec<HeadingContext>, text: &str) {
    if let Some(current) = heading_stack.last_mut() {
        current.text.push_str(text);
    }
}

fn escape_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '{' => out.push_str("&#123;"),
            '}' => out.push_str("&#125;"),
            _ => out.push(ch),
        }
    }
    out
}
