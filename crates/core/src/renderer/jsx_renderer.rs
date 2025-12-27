#![allow(missing_docs)]
use crate::event::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use crate::renderer::multipass::{Block, scan};
use crate::{
    DirectiveAdapter, HoistAdapter, MarkflowError, ParseResult, RewriteOptions, get_event_iterator,
    parse,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Render Markdown input into a raw JSX-like string, preserving JSX nodes.
pub fn render_to_jsx(input: &str) -> Result<String, MarkflowError> {
    let ParseResult { html: _, imports } = parse(input)?;
    let mut output = String::new();

    let mut seen_imports = HashSet::new();
    for import in imports {
        if seen_imports.insert(import.source.clone()) {
            output.push_str(&import.source);
            output.push('\n');
        }
    }

    let blocks = scan(input);
    output.push_str(&render_blocks(&blocks)?);
    Ok(output)
}

fn render_markdown_events(input: &str) -> Result<String, MarkflowError> {
    let rewrite_options = RewriteOptions::default();
    let hoisted = Rc::new(RefCell::new(Vec::new()));
    let mut events: Box<dyn Iterator<Item = Event<'static>>> = Box::new(get_event_iterator(input)?);
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
                if let Tag::Heading { level, id, .. } = &tag
                    && let Some(_id) = id.as_ref()
                {
                    output.push_str(&heading_wrapper_start(*level));
                    heading_stack.push(HeadingContext {
                        text: String::new(),
                    });
                }
                output.push_str(&start_tag(&tag));
                stack.push(tag);
            }
            Event::End(end) => {
                if let Some(open) = stack.pop()
                    && matches_end(&open, &end)
                {
                    output.push_str(&end_tag(&end));
                    if let Tag::Heading { id, .. } = &open
                        && let Some(id) = id.as_ref()
                    {
                        let heading_text =
                            heading_stack.pop().map(|ctx| ctx.text).unwrap_or_default();
                        output.push_str(&anchor_link(id.as_ref(), &heading_text));
                        output.push_str("</div>");
                    }
                }
            }
            Event::Text(text) => {
                let text = text.as_ref();
                if text.starts_with('\0') {
                    continue;
                }
                output.push_str(&escape_text(text));
                push_heading_text(&mut heading_stack, text);
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
            Event::JsxInline(text) | Event::JsxFlow(text) => {
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

fn render_blocks(blocks: &[Block<'_>]) -> Result<String, MarkflowError> {
    let mut output = String::new();
    for block in blocks {
        output.push_str(&render_block(block)?);
    }
    Ok(output)
}

fn render_block(block: &Block<'_>) -> Result<String, MarkflowError> {
    match block {
        Block::Markdown(text) => render_markdown_events(text),
        Block::Code(text) => Ok(text.to_string()),
        Block::JsxElement {
            name,
            attrs,
            children,
            is_self_closing,
        } => {
            let rendered_attrs = render_attrs(attrs, *is_self_closing);
            if *is_self_closing {
                Ok(format!("<{}{} />", name, rendered_attrs))
            } else {
                let inner = if *name == "Steps" {
                    render_steps_children(children)?
                } else {
                    render_blocks(children)?
                };
                Ok(format!("<{}{}>{}</{}>", name, rendered_attrs, inner, name))
            }
        }
    }
}

fn render_steps_children(children: &[Block<'_>]) -> Result<String, MarkflowError> {
    let mut markdown_output = String::new();
    let mut sibling_output = String::new();

    for child in children {
        match child {
            Block::Markdown(text) => markdown_output.push_str(&render_markdown_events(text)?),
            _ => sibling_output.push_str(&render_block(child)?),
        }
    }

    if !sibling_output.is_empty() {
        if let Some(idx) = markdown_output.rfind("</li>") {
            markdown_output.insert_str(idx, &sibling_output);
        } else {
            markdown_output.push_str(&sibling_output);
        }
    }

    Ok(markdown_output)
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

fn push_heading_text(heading_stack: &mut [HeadingContext], text: &str) {
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
