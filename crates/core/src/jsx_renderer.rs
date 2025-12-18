use crate::event::{CodeBlockKind, Event, Tag, TagEnd};
use crate::code_fence::collect_root_imports;
use crate::{get_event_iterator, MarkflowError};

/// Render Markdown input into a raw JSX-like string, preserving JSX nodes.
pub fn render_to_jsx(input: &str) -> Result<String, MarkflowError> {
    let (hoisted, body_lines) = collect_root_imports(input);
    let body = body_lines.join("\n");
    let events = get_event_iterator(&body)?;
    let mut output = String::new();

    for import in hoisted {
        output.push_str(&import);
        output.push('\n');
    }

    let mut stack: Vec<Tag<'_>> = Vec::new();

    for event in events {
        match event {
            Event::Start(tag) => {
                output.push_str(&start_tag(&tag));
                stack.push(tag);
            }
            Event::End(end) => {
                if let Some(open) = stack.pop() {
                    if matches_end(&open, &end) {
                        output.push_str(&end_tag(&end));
                    }
                }
            }
            Event::Text(text) => {
                output.push_str(&escape_text(text.as_ref()));
            }
            Event::Code(text) => {
                output.push_str("<code>");
                output.push_str(&escape_text(text.as_ref()));
                output.push_str("</code>");
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                output.push_str(text.as_ref());
            }
            Event::JsxInline(text) | Event::JsxFlow(text) => {
                output.push_str(text.as_ref());
            }
            Event::InlineMath(math) => {
                output.push_str(&format!("<span class=\"math-inline\">{}</span>", math));
            }
            Event::DisplayMath(math) => {
                output.push_str(&format!("<div class=\"math-display\">{}</div>", math));
            }
            Event::FootnoteReference(label) => {
                output.push_str(&format!(
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{0}\" id=\"fnref-{0}\">{0}</a></sup>",
                    label
                ));
            }
            Event::TaskListMarker(done) => {
                if done {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" checked=\"\" />");
                } else {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" />");
                }
            }
            Event::Rule => output.push_str("<hr />\n"),
            Event::HardBreak => output.push_str("<br />\n"),
            Event::SoftBreak => output.push('\n'),
        }
    }

    Ok(output)
}

fn start_tag(tag: &Tag<'_>) -> String {
    match tag {
        Tag::Paragraph => "<p>".to_string(),
        Tag::Heading { level, id, classes, attrs } => {
            let mut s = format!("<h{}", *level as u8);
            if let Some(id) = id {
                s.push_str(&format!(" id=\"{}\"", id));
            }
            if !classes.is_empty() {
                s.push_str(" class=\"");
                for (idx, class) in classes.iter().enumerate() {
                    if idx > 0 { s.push(' '); }
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
        Tag::FootnoteDefinition(label) => format!("<section class=\"footnote\" id=\"fn-{}\">", label),
        Tag::Table(_) => "<table>".to_string(),
        Tag::TableHead => "<thead>".to_string(),
        Tag::TableRow => "<tr>".to_string(),
        Tag::TableCell => "<td>".to_string(),
        Tag::Emphasis => "<em>".to_string(),
        Tag::Strong => "<strong>".to_string(),
        Tag::Strikethrough => "<del>".to_string(),
        Tag::Link { dest_url, title, .. } => {
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
            if *ordered { "</ol>\n".to_string() } else { "</ul>\n".to_string() }
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
    match (tag, end) {
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
        | (Tag::Image { .. }, TagEnd::Image) => true,
        _ => false,
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
