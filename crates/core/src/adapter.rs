use crate::event::Event;
use crate::renderer::html::HtmlRenderer;
use std::io::{self, Write};

/// Extension trait to pipe Markdown events directly to a Writer.
///
/// This replaces the struct-based `PipeAdapter` with a zero-cost abstraction,
/// allowing for a more fluent method chain style.
pub trait MarkdownStream: Sized {
    /// Drives the iterator events into the writer, converting Markdown to HTML on the fly.
    ///
    /// # Arguments
    /// * `writer` - The destination writer (e.g. `StreamingRewriter`).
    ///
    /// Returns the writer back to the caller upon success.
    fn stream_to_writer<W: Write>(self, writer: W) -> io::Result<W>;
}

impl<'a, I> MarkdownStream for I
where
    I: Iterator<Item = Event<'a>>,
{
    fn stream_to_writer<W: Write>(self, writer: W) -> io::Result<W> {
        HtmlRenderer::new(writer).render(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event as MfEvent, HeadingLevel, Tag};
    use std::borrow::Cow;

    #[test]
    fn test_streaming_output() {
        let mut output_buffer = Vec::new(); // Implements Write

        let heading = Tag::Heading {
            level: HeadingLevel::H1,
            id: None,
            classes: Vec::new(),
            attrs: Vec::new(),
        };
        let events = vec![
            MfEvent::Start(heading.clone()),
            MfEvent::Text(Cow::Borrowed("Hello Stream")),
            MfEvent::End(heading.to_end()),
        ];

        events
            .into_iter()
            .stream_to_writer(&mut output_buffer)
            .expect("Failed to drive stream");

        let output_str = String::from_utf8(output_buffer).unwrap();

        assert!(output_str.contains("<h1>Hello Stream</h1>"));
    }
}
