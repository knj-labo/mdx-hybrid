use std::borrow::Cow;
use std::io::{self, Write};

use crate::event::{Alignment, CodeBlockKind, Event, LinkType, Tag, TagEnd};

pub struct HtmlRenderer<W: Write> {
    writer: W,
    table_head_depth: usize,
    table_stack: Vec<TableState>,
    image_stack: Vec<ImageContext>,
}

struct TableState {
    alignments: Vec<Alignment>,
    column_index: usize,
}

struct ImageContext {
    dest_url: String,
    title: String,
    alt: String,
}

impl<W: Write> HtmlRenderer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            table_head_depth: 0,
            table_stack: Vec::new(),
            image_stack: Vec::new(),
        }
    }

    pub fn render<'a, I>(mut self, iter: I) -> io::Result<W>
    where
        I: IntoIterator<Item = Event<'a>>,
    {
        for event in iter.into_iter() {
            if self.handle_image_text(&event) {
                continue;
            }

            match event {
                Event::Start(tag) => {
                    if let Tag::Image {
                        link_type,
                        dest_url,
                        title,
                        id,
                    } = tag
                    {
                        self.start_image(link_type, dest_url, title, id);
                    } else {
                        self.write_start_tag(tag)?;
                    }
                }
                Event::End(end) => {
                    if matches!(end, TagEnd::Image) {
                        self.finish_image()?;
                    } else {
                        self.write_end_tag(end)?;
                    }
                }
                Event::Text(text) => {
                    self.write_text(text.as_ref())?;
                }
                Event::Code(text) => {
                    self.writer.write_all(b"<code>")?;
                    self.escape_html(text.as_ref())?;
                    self.writer.write_all(b"</code>")?;
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    self.writer.write_all(html.as_ref().as_bytes())?;
                }
                Event::JsxInline(snippet) | Event::JsxFlow(snippet) => {
                    self.writer.write_all(snippet.as_ref().as_bytes())?;
                }
                Event::InlineMath(math) => {
                    self.writer.write_all(b"<span class=\"math-inline\">")?;
                    self.escape_html(math.as_ref())?;
                    self.writer.write_all(b"</span>")?;
                }
                Event::DisplayMath(math) => {
                    self.writer.write_all(b"<div class=\"math-display\">")?;
                    self.escape_html(math.as_ref())?;
                    self.writer.write_all(b"</div>")?;
                }
                Event::FootnoteReference(label) => {
                    write!(
                        self.writer,
                        "<sup class=\"footnote-ref\"><a href=\"#fn-{0}\" id=\"fnref-{0}\">{0}</a></sup>",
                        label.as_ref()
                    )?;
                }
                Event::TaskListMarker(done) => {
                    if done {
                        self.writer
                            .write_all(b"<input type=\"checkbox\" disabled=\"\" checked=\"\" />")?;
                    } else {
                        self.writer
                            .write_all(b"<input type=\"checkbox\" disabled=\"\" />")?;
                    }
                }
                Event::Rule => {
                    self.writer.write_all(b"<hr />\n")?;
                }
                Event::HardBreak => {
                    self.writer.write_all(b"<br />\n")?;
                }
                Event::SoftBreak => {
                    self.writer.write_all(b"\n")?;
                }
            }
        }

        Ok(self.writer)
    }

    fn write_start_tag(&mut self, tag: Tag<'_>) -> io::Result<()> {
        match tag {
            Tag::Paragraph => self.writer.write_all(b"<p>"),
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                write!(self.writer, "<h{}", level as u8)?;
                if let Some(id) = id {
                    self.write_attr("id", id.as_ref())?;
                }
                if !classes.is_empty() {
                    self.writer.write_all(b" class=\"")?;
                    for (idx, class) in classes.iter().enumerate() {
                        if idx > 0 {
                            self.writer.write_all(b" ")?;
                        }
                        self.escape_html(class.as_ref())?;
                    }
                    self.writer.write_all(b"\"")?;
                }
                for (key, value) in attrs {
                    if let Some(value) = value {
                        self.write_attr(key.as_ref(), value.as_ref())?;
                    } else {
                        write!(self.writer, " {}", key.as_ref())?;
                    }
                }
                self.writer.write_all(b">")
            }
            Tag::BlockQuote => self.writer.write_all(b"<blockquote>"),
            Tag::CodeBlock(kind) => match kind {
                CodeBlockKind::Indented => self.writer.write_all(b"<pre><code>"),
                CodeBlockKind::Fenced(lang) => {
                    self.writer.write_all(b"<pre><code class=\"language-")?;
                    self.escape_html(lang.as_ref())?;
                    self.writer.write_all(b"\">")
                }
            },
            Tag::List(start) => {
                if let Some(idx) = start {
                    write!(self.writer, "<ol start=\"{}\">", idx)
                } else {
                    self.writer.write_all(b"<ul>")
                }
            }
            Tag::Item => self.writer.write_all(b"<li>"),
            Tag::FootnoteDefinition(label) => {
                write!(
                    self.writer,
                    "<section class=\"footnote\" id=\"fn-{label}\">"
                )
            }
            Tag::Table(alignments) => {
                self.table_stack.push(TableState {
                    alignments,
                    column_index: 0,
                });
                self.writer.write_all(b"<table>")
            }
            Tag::TableHead => {
                self.table_head_depth += 1;
                self.writer.write_all(b"<thead>")
            }
            Tag::TableRow => {
                if let Some(state) = self.table_stack.last_mut() {
                    state.column_index = 0;
                }
                self.writer.write_all(b"<tr>")
            }
            Tag::TableCell => {
                let tag = if self.table_head_depth > 0 {
                    b"th"
                } else {
                    b"td"
                };
                self.writer.write_all(b"<")?;
                self.writer.write_all(tag)?;
                if let Some(state) = self.table_stack.last_mut()
                    && let Some(alignment) = state.alignments.get(state.column_index)
                {
                    if !matches!(alignment, Alignment::None) {
                        self.writer.write_all(b" style=\"text-align:")?;
                        self.writer.write_all(match alignment {
                            Alignment::Left => b"left",
                            Alignment::Right => b"right",
                            Alignment::Center => b"center",
                            Alignment::None => b"left",
                        })?;
                        self.writer.write_all(b"\"")?;
                    }
                    state.column_index += 1;
                }
                self.writer.write_all(b">")
            }
            Tag::Emphasis => self.writer.write_all(b"<em>"),
            Tag::Strong => self.writer.write_all(b"<strong>"),
            Tag::Strikethrough => self.writer.write_all(b"<del>"),
            Tag::Link {
                dest_url, title, ..
            } => {
                self.writer.write_all(b"<a href=\"")?;
                self.escape_attr(dest_url.as_ref())?;
                self.writer.write_all(b"\"")?;
                if !title.is_empty() {
                    self.writer.write_all(b" title=\"")?;
                    self.escape_attr(title.as_ref())?;
                    self.writer.write_all(b"\"")?;
                }
                self.writer.write_all(b">")
            }
            Tag::Image { .. } => unreachable!("image handled separately"),
        }
    }

    fn write_end_tag(&mut self, end: TagEnd) -> io::Result<()> {
        match end {
            TagEnd::Paragraph => self.writer.write_all(b"</p>\n"),
            TagEnd::Heading(level) => writeln!(self.writer, "</h{}>", level as u8),
            TagEnd::BlockQuote => self.writer.write_all(b"</blockquote>\n"),
            TagEnd::CodeBlock => self.writer.write_all(b"</code></pre>\n"),
            TagEnd::List(ordered) => {
                if ordered {
                    self.writer.write_all(b"</ol>\n")
                } else {
                    self.writer.write_all(b"</ul>\n")
                }
            }
            TagEnd::Item => self.writer.write_all(b"</li>"),
            TagEnd::FootnoteDefinition => self.writer.write_all(b"</section>\n"),
            TagEnd::Table => {
                self.table_stack.pop();
                self.writer.write_all(b"</table>\n")
            }
            TagEnd::TableHead => {
                self.table_head_depth = self.table_head_depth.saturating_sub(1);
                self.writer.write_all(b"</thead>\n")
            }
            TagEnd::TableRow => self.writer.write_all(b"</tr>\n"),
            TagEnd::TableCell => {
                let tag = if self.table_head_depth > 0 {
                    b"th"
                } else {
                    b"td"
                };
                self.writer.write_all(b"</")?;
                self.writer.write_all(tag)?;
                self.writer.write_all(b">")
            }
            TagEnd::Emphasis => self.writer.write_all(b"</em>"),
            TagEnd::Strong => self.writer.write_all(b"</strong>"),
            TagEnd::Strikethrough => self.writer.write_all(b"</del>"),
            TagEnd::Link => self.writer.write_all(b"</a>"),
            TagEnd::Image => unreachable!("image handled separately"),
        }
    }

    fn write_text(&mut self, text: &str) -> io::Result<()> {
        self.escape_html(text)
    }

    fn escape_html(&mut self, text: &str) -> io::Result<()> {
        for ch in text.chars() {
            match ch {
                '&' => self.writer.write_all(b"&amp;")?,
                '<' => self.writer.write_all(b"&lt;")?,
                '>' => self.writer.write_all(b"&gt;")?,
                '"' => self.writer.write_all(b"&quot;")?,
                '\'' => self.writer.write_all(b"&#39;")?,
                _ => self
                    .writer
                    .write_all(ch.encode_utf8(&mut [0; 4]).as_bytes())?,
            }
        }
        Ok(())
    }

    fn escape_attr(&mut self, value: &str) -> io::Result<()> {
        self.escape_html(value)
    }

    fn write_attr(&mut self, key: &str, value: &str) -> io::Result<()> {
        write!(self.writer, " {}=\"", key)?;
        self.escape_attr(value)?;
        self.writer.write_all(b"\"")
    }

    fn start_image(
        &mut self,
        _: LinkType,
        dest_url: Cow<'_, str>,
        title: Cow<'_, str>,
        _: Cow<'_, str>,
    ) {
        self.image_stack.push(ImageContext {
            dest_url: dest_url.into_owned(),
            title: title.into_owned(),
            alt: String::new(),
        });
    }

    fn finish_image(&mut self) -> io::Result<()> {
        if let Some(image) = self.image_stack.pop() {
            self.writer.write_all(b"<img src=\"")?;
            self.escape_attr(&image.dest_url)?;
            self.writer.write_all(b"\" alt=\"")?;
            self.escape_attr(&image.alt)?;
            self.writer.write_all(b"\"")?;
            if !image.title.is_empty() {
                self.writer.write_all(b" title=\"")?;
                self.escape_attr(&image.title)?;
                self.writer.write_all(b"\"")?;
            }
            self.writer.write_all(b" loading=\"lazy\" />")
        } else {
            Ok(())
        }
    }

    fn handle_image_text<'a>(&mut self, event: &Event<'a>) -> bool {
        if let Some(current) = self.image_stack.last_mut() {
            match event {
                Event::Text(text) | Event::Code(text) => {
                    current.alt.push_str(text.as_ref());
                    return true;
                }
                Event::SoftBreak | Event::HardBreak => {
                    current.alt.push(' ');
                    return true;
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::adapter::MarkdownStream;
    use crate::event::Event as MfEvent;
    use std::borrow::Cow;

    #[test]
    fn renders_jsx_events_without_panicking() {
        let mut output_buffer = Vec::new();
        let events = vec![
            MfEvent::JsxInline(Cow::Borrowed("<Inline />")),
            MfEvent::JsxFlow(Cow::Borrowed("<Block>\n</Block>")),
        ];

        events
            .into_iter()
            .stream_to_writer(&mut output_buffer)
            .expect("render should succeed");

        let output_str = String::from_utf8(output_buffer).unwrap();
        assert!(output_str.contains("<Inline />"));
        assert!(output_str.contains("<Block>"));
    }
}
