#![allow(missing_docs)]
use crate::event::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use crate::renderer::multipass::{Block, scan};
use crate::transform::code_fence::collect_root_imports;
use crate::{DirectiveAdapter, HoistAdapter, MarkflowError, RewriteOptions, get_event_iterator};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::rc::Rc;

/// Render Markdown input into a raw JSX-like string, preserving JSX nodes.
pub fn render_to_jsx(input: &str) -> Result<String, MarkflowError> {
    let rewrite_options = RewriteOptions::default();
    let mut seen_imports = HashSet::new();
    let (root_imports, _body_lines) = collect_root_imports(input);
    let blocks = scan(input);
    let mut body = String::with_capacity(input.len());
    render_blocks_into(&blocks, &rewrite_options, &mut body)?;

    let mut output = String::with_capacity(body.len() + input.len());
    for import in root_imports {
        if seen_imports.insert(import.clone()) {
            output.push_str(&import);
            output.push('\n');
        }
    }
    for import in rewrite_options.required_imports.borrow().iter() {
        if seen_imports.insert(import.clone()) {
            output.push_str(import);
            output.push('\n');
        }
    }
    output.push_str(&body);
    Ok(output)
}

fn render_markdown_events(
    input: &str,
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let hoisted = Rc::new(RefCell::new(Vec::new()));
    let mut events: Box<dyn Iterator<Item = Event<'static>>> = Box::new(get_event_iterator(input)?);
    if rewrite_options.enable_hoist {
        events = Box::new(HoistAdapter::new(events, Rc::clone(&hoisted)));
    }
    let events: Box<dyn Iterator<Item = Event<'static>>> = if rewrite_options.enable_directives {
        Box::new(DirectiveAdapter::new(
            events,
            Rc::new(RefCell::new(0)),
            rewrite_options.directive_mapper.clone(),
            rewrite_options.required_imports.clone(),
        ))
    } else {
        events
    };

    let mut stack: Vec<Tag<'_>> = Vec::new();
    let mut heading_stack: Vec<HeadingContext> = Vec::new();
    for event in events {
        match event {
            Event::Start(tag, _) => {
                if let Tag::Heading { level, id, .. } = &tag
                    && let Some(_id) = id.as_ref()
                {
                    write_heading_wrapper_start(output, *level);
                    heading_stack.push(HeadingContext {
                        text: String::new(),
                    });
                }
                write_start_tag(output, &tag);
                stack.push(tag);
            }
            Event::End(end, _) => {
                if let Some(open) = stack.pop()
                    && matches_end(&open, &end)
                {
                    write_end_tag(output, &end);
                    if let Tag::Heading { id, .. } = &open
                        && let Some(id) = id.as_ref()
                    {
                        let heading_text =
                            heading_stack.pop().map(|ctx| ctx.text).unwrap_or_default();
                        write_anchor_link(output, id.as_ref(), &heading_text);
                        output.push_str("</div>");
                    }
                }
            }
            Event::Text(text, _) => {
                let text = text.as_ref();
                if text.starts_with('\0') {
                    continue;
                }
                escape_text_into(output, text);
                push_heading_text(&mut heading_stack, text);
            }
            Event::Code(text, _) => {
                output.push_str("<code>");
                escape_text_into(output, text.as_ref());
                output.push_str("</code>");
                push_heading_text(&mut heading_stack, text.as_ref());
            }
            Event::Html(text, _) | Event::InlineHtml(text, _) => {
                output.push_str(text.as_ref());
            }
            Event::JsxInline(text, _) | Event::JsxFlow(text, _) => {
                output.push_str(text.as_ref());
            }
            Event::InlineMath(math, _) => {
                output.push_str("<span class=\"math-inline\">");
                output.push_str(math.as_ref());
                output.push_str("</span>");
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::DisplayMath(math, _) => {
                output.push_str("<div class=\"math-display\">");
                output.push_str(math.as_ref());
                output.push_str("</div>");
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::FootnoteReference(label, _) => {
                let _ = write!(
                    output,
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{0}\" id=\"fnref-{0}\">{0}</a></sup>",
                    label
                );
                push_heading_text(&mut heading_stack, label.as_ref());
            }
            Event::TaskListMarker(done, _) => {
                if done {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" checked=\"\" />");
                } else {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" />");
                }
            }
            Event::Rule(_) => output.push_str("<hr />\n"),
            Event::HardBreak(_) => {
                output.push_str("<br />\n");
                push_heading_text(&mut heading_stack, " ");
            }
            Event::SoftBreak(_) => {
                output.push('\n');
                push_heading_text(&mut heading_stack, " ");
            }
        }
    }

    Ok(())
}

fn render_blocks_into(
    blocks: &[Block<'_>],
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    for block in blocks {
        render_block_into(block, rewrite_options, output)?;
    }
    Ok(())
}

fn render_block_into(
    block: &Block<'_>,
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    match block {
        Block::Markdown(text) => render_markdown_events(text, rewrite_options, output),
        Block::Code(text) => render_markdown_events(text, rewrite_options, output),
        Block::JsxElement {
            name,
            attrs,
            children,
            is_self_closing,
        } => {
            let rendered_attrs = render_attrs(attrs, *is_self_closing);
            if *is_self_closing {
                output.push('<');
                output.push_str(name);
                output.push_str(&rendered_attrs);
                output.push_str(" />");
            } else {
                output.push('<');
                output.push_str(name);
                output.push_str(&rendered_attrs);
                output.push('>');
                if *name == "Steps" {
                    let mut inner = String::new();
                    render_steps_children_into(children, rewrite_options, &mut inner)?;
                    output.push_str(&inner);
                } else if *name == "FileTree" {
                    let mut inner = String::new();
                    render_file_tree_children_into(children, rewrite_options, &mut inner)?;
                    output.push_str(&inner);
                } else {
                    render_jsx_children_into(children, rewrite_options, output)?;
                }
                output.push_str("</");
                output.push_str(name);
                output.push('>');
            }
            Ok(())
        }
    }
}

fn render_jsx_children_into(
    children: &[Block<'_>],
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    for child in children {
        match child {
            Block::Markdown(text) => {
                let dedented = dedent_one_level(text);
                render_markdown_events(&dedented, rewrite_options, output)?;
            }
            Block::Code(text) => {
                let dedented = dedent_one_level(text);
                render_markdown_events(&dedented, rewrite_options, output)?;
            }
            _ => {
                render_block_into(child, rewrite_options, output)?;
            }
        }
    }

    Ok(())
}

fn render_steps_children_into(
    children: &[Block<'_>],
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut scratch = String::new();
    for child in children {
        scratch.clear();
        match child {
            Block::Markdown(text) => {
                let dedented = dedent_one_level(text);
                render_markdown_events(&dedented, rewrite_options, &mut scratch)?;
                append_steps_fragment(output, &scratch);
            }
            _ => {
                render_block_into(child, rewrite_options, &mut scratch)?;
                insert_into_last_list_item(output, &scratch);
            }
        }
    }

    Ok(())
}

fn append_steps_fragment(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if output.is_empty() {
        output.push_str(fragment);
        return;
    }

    if let Some((pre, inner, post)) = split_ordered_list_fragment(fragment) {
        if !pre.trim().is_empty() {
            insert_into_last_list_item(output, pre);
        }
        if !inner.trim().is_empty() {
            insert_before_list_close(output, inner);
        }
        if !post.trim().is_empty() {
            insert_into_last_list_item(output, post);
        }
    } else if fragment.contains("<li") && !fragment.contains("<ol") {
        insert_before_list_close(output, fragment);
    } else {
        insert_into_last_list_item(output, fragment);
    }
}

fn split_ordered_list_fragment(fragment: &str) -> Option<(&str, &str, &str)> {
    let open_start = fragment.find("<ol")?;
    let open_end = fragment[open_start..].find('>')? + open_start;
    let close_start = fragment.rfind("</ol>")?;
    let close_end = close_start + "</ol>".len();
    if close_start < open_end {
        return None;
    }
    let pre = &fragment[..open_start];
    let inner = &fragment[open_end + 1..close_start];
    let post = &fragment[close_end..];
    Some((pre, inner, post))
}

fn insert_into_last_list_item(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if let Some(idx) = output.rfind("</li>") {
        output.insert_str(idx, fragment);
    } else {
        output.push_str(fragment);
    }
}

fn insert_before_list_close(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if let Some(idx) = output.rfind("</ol>") {
        output.insert_str(idx, fragment);
    } else {
        output.push_str(fragment);
    }
}

fn render_file_tree_children_into(
    children: &[Block<'_>],
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut inner = String::new();
    let mut markdown_buffer = String::new();

    for child in children {
        match child {
            Block::Markdown(text) => {
                markdown_buffer.push_str(&dedent_one_level(text));
            }
            _ => {
                if !markdown_buffer.is_empty() {
                    render_markdown_events(&markdown_buffer, rewrite_options, &mut inner)?;
                    markdown_buffer.clear();
                }
                render_block_into(child, rewrite_options, &mut inner)?;
            }
        }
    }

    if !markdown_buffer.is_empty() {
        render_markdown_events(&markdown_buffer, rewrite_options, &mut inner)?;
    }

    output.push_str(&extract_first_unordered_list(&inner));
    Ok(())
}

fn dedent_one_level(input: &str) -> String {
    let mut any = false;
    let mut can_strip_spaces = true;
    let mut can_strip_tabs = true;

    for line in input.split_inclusive('\n') {
        if line.trim().is_empty() {
            continue;
        }
        any = true;
        if !line.starts_with("    ") {
            can_strip_spaces = false;
        }
        if !line.starts_with('\t') {
            can_strip_tabs = false;
        }
    }

    if !any {
        return input.to_string();
    }

    let strip_spaces = if can_strip_spaces {
        Some(4)
    } else if can_strip_tabs {
        Some(1)
    } else {
        None
    };

    let Some(strip) = strip_spaces else {
        return input.to_string();
    };

    let mut output = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        if strip == 4 && line.starts_with("    ") {
            output.push_str(&line[4..]);
        } else if strip == 1 && line.starts_with('\t') {
            output.push_str(&line[1..]);
        } else {
            output.push_str(line);
        }
    }
    output
}

fn extract_first_unordered_list(input: &str) -> String {
    let bytes = input.as_bytes();
    let Some(start) = find_ul_open(bytes, 0) else {
        return input.to_string();
    };

    let mut depth = 0usize;
    let mut pos = start;
    while pos < bytes.len() {
        if let Some(open_pos) = find_ul_open(bytes, pos)
            && open_pos == pos
        {
            depth += 1;
            pos += 3;
            continue;
        }
        if let Some((close_pos, close_len)) = find_ul_close(bytes, pos)
            && close_pos == pos
        {
            depth = depth.saturating_sub(1);
            pos += close_len;
            if depth == 0 {
                let end = pos;
                return input[start..end].to_string();
            }
            continue;
        }
        pos += 1;
    }

    input.to_string()
}

fn find_ul_open(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 2 < bytes.len() {
        if bytes[pos] == b'<' && bytes[pos + 1] == b'u' && bytes[pos + 2] == b'l' {
            let next = bytes.get(pos + 3).copied().unwrap_or(b'>');
            if matches!(next, b'>' | b' ' | b'\t' | b'\n' | b'\r') {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

fn find_ul_close(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut pos = start;
    while pos + 3 < bytes.len() {
        if bytes[pos] == b'<'
            && bytes[pos + 1] == b'/'
            && bytes[pos + 2] == b'u'
            && bytes[pos + 3] == b'l'
        {
            let mut end = pos + 4;
            while end < bytes.len() {
                match bytes[end] {
                    b'>' => return Some((pos, end + 1 - pos)),
                    b' ' | b'\t' | b'\n' | b'\r' => {
                        end += 1;
                        continue;
                    }
                    _ => break,
                }
            }
        }
        pos += 1;
    }
    None
}

fn render_attrs(attrs: &str, is_self_closing: bool) -> String {
    let mut normalized = attrs;
    if is_self_closing {
        let trimmed = normalized.trim_end();
        if let Some(stripped) = trimmed.strip_suffix('/') {
            normalized = stripped.trim_end();
        }
    }
    if normalized.is_empty() {
        String::new()
    } else {
        format!(" {}", normalized)
    }
}

fn write_start_tag(output: &mut String, tag: &Tag<'_>) {
    match tag {
        Tag::Paragraph => output.push_str("<p>"),
        Tag::Heading {
            level,
            id,
            classes,
            attrs,
        } => {
            let _ = write!(output, "<h{}", *level as u8);
            if let Some(id) = id {
                output.push_str(" id=\"");
                output.push_str(id);
                output.push('"');
            }
            if !classes.is_empty() {
                output.push_str(" class=\"");
                for (idx, class) in classes.iter().enumerate() {
                    if idx > 0 {
                        output.push(' ');
                    }
                    output.push_str(class);
                }
                output.push('"');
            }
            for (key, value) in attrs {
                match value {
                    Some(v) => {
                        output.push(' ');
                        output.push_str(key);
                        output.push_str("=\"");
                        output.push_str(v);
                        output.push('"');
                    }
                    None => {
                        output.push(' ');
                        output.push_str(key);
                    }
                }
            }
            output.push('>');
        }
        Tag::BlockQuote => output.push_str("<blockquote>"),
        Tag::CodeBlock(kind) => match kind {
            CodeBlockKind::Indented => output.push_str("<pre><code>"),
            CodeBlockKind::Fenced(lang) => {
                output.push_str("<pre><code class=\"language-");
                output.push_str(lang);
                output.push_str("\">");
            }
        },
        Tag::List(start) => {
            if start.is_some() {
                output.push_str("<ol>");
            } else {
                output.push_str("<ul>");
            }
        }
        Tag::Item => output.push_str("<li>"),
        Tag::FootnoteDefinition(label) => {
            output.push_str("<section class=\"footnote\" id=\"fn-");
            output.push_str(label);
            output.push_str("\">");
        }
        Tag::Table(_) => output.push_str("<table>"),
        Tag::TableHead => output.push_str("<thead>"),
        Tag::TableRow => output.push_str("<tr>"),
        Tag::TableCell => output.push_str("<td>"),
        Tag::Emphasis => output.push_str("<em>"),
        Tag::Strong => output.push_str("<strong>"),
        Tag::Strikethrough => output.push_str("<del>"),
        Tag::Link {
            dest_url, title, ..
        } => {
            output.push_str("<a href=\"");
            output.push_str(dest_url);
            output.push('"');
            if !title.is_empty() {
                output.push_str(" title=\"");
                output.push_str(title);
                output.push('"');
            }
            output.push('>');
        }
        Tag::Image { .. } => {} // handled elsewhere in HTML renderer; skip here
    }
}

fn write_end_tag(output: &mut String, end: &TagEnd) {
    match end {
        TagEnd::Paragraph => output.push_str("</p>\n"),
        TagEnd::Heading(level) => {
            let _ = write!(output, "</h{}>", *level as u8);
        }
        TagEnd::BlockQuote => output.push_str("</blockquote>\n"),
        TagEnd::CodeBlock => output.push_str("</code></pre>\n"),
        TagEnd::List(ordered) => {
            if *ordered {
                output.push_str("</ol>\n");
            } else {
                output.push_str("</ul>\n");
            }
        }
        TagEnd::Item => output.push_str("</li>"),
        TagEnd::FootnoteDefinition => output.push_str("</section>\n"),
        TagEnd::Table => output.push_str("</table>\n"),
        TagEnd::TableHead => output.push_str("</thead>\n"),
        TagEnd::TableRow => output.push_str("</tr>\n"),
        TagEnd::TableCell => output.push_str("</td>"),
        TagEnd::Emphasis => output.push_str("</em>"),
        TagEnd::Strong => output.push_str("</strong>"),
        TagEnd::Strikethrough => output.push_str("</del>"),
        TagEnd::Link => output.push_str("</a>"),
        TagEnd::Image => {}
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
    text: String,
}

fn write_heading_wrapper_start(output: &mut String, level: HeadingLevel) {
    let _ = write!(
        output,
        "<div class=\"sl-heading-wrapper level-h{}\">",
        level as u8
    );
}

fn write_anchor_link(output: &mut String, id: &str, heading_text: &str) {
    output.push_str("<a class=\"sl-anchor-link\" href=\"#");
    output.push_str(id);
    output.push_str("\"><span aria-hidden=\"true\" class=\"sl-anchor-icon\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\"><path fill=\"currentcolor\" d=\"m12.11 15.39-3.88 3.88a2.52 2.52 0 0 1-3.5 0 2.47 2.47 0 0 1 0-3.5l3.88-3.88a1 1 0 0 0-1.42-1.42l-3.88 3.89a4.48 4.48 0 0 0 6.33 6.33l3.89-3.88a1 1 0 1 0-1.42-1.42Zm8.58-12.08a4.49 4.49 0 0 0-6.33 0l-3.89 3.88a1 1 0 0 0 1.42 1.42l3.88-3.88a2.52 2.52 0 0 1 3.5 0 2.47 2.47 0 0 1 0 3.5l-3.88 3.88a1 1 0 1 0 1.42 1.42l3.88-3.89a4.49 4.49 0 0 0 0-6.33ZM8.83 15.17a1 1 0 0 0 1.1.22 1 1 0 0 0 .32-.22l4.92-4.92a1 1 0 0 0-1.42-1.42l-4.92 4.92a1 1 0 0 0 0 1.42Z\"></path></svg></span><span class=\"sr-only\">");
    output.push_str("Section titled \"");
    escape_text_into(output, heading_text.trim());
    output.push_str("\"</span></a>");
}

fn push_heading_text(heading_stack: &mut [HeadingContext], text: &str) {
    if let Some(current) = heading_stack.last_mut() {
        current.text.push_str(text);
    }
}

fn escape_text_into(out: &mut String, text: &str) {
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
}
