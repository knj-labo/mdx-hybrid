//! Heading extraction helpers.

use crate::types::HeadingEntry;
use crate::{FileType, convert_error};
use markflow_core::event::{Event as CoreEvent, HeadingLevel, Tag as CoreTag, TagEnd as CoreTagEnd};
use napi::bindgen_prelude::Result;

pub(crate) struct HeadingCollector {
    headings: Vec<HeadingEntry>,
    slugger: markflow_core::Slugger,
    current_heading: Option<HeadingCapture>,
}

impl HeadingCollector {
    pub(crate) fn new() -> Self {
        Self {
            headings: Vec::new(),
            slugger: markflow_core::Slugger::new(),
            current_heading: None,
        }
    }

    fn record_heading(&mut self, text: &str, level: HeadingLevel, id: Option<String>) {
        let slug = id.unwrap_or_else(|| self.slugger.next_slug(text));
        self.headings.push(HeadingEntry {
            depth: level as u8,
            slug,
            text: text.to_string(),
        });
    }

    fn begin_heading(&mut self, level: HeadingLevel, id: Option<String>) {
        self.current_heading = Some(HeadingCapture {
            level,
            buffer: String::new(),
            id,
        });
    }

    fn push_heading_text(&mut self, text: &str) {
        if let Some(capture) = self.current_heading.as_mut() {
            capture.buffer.push_str(text);
        }
    }

    fn end_heading(&mut self, level: HeadingLevel) {
        if let Some(capture) = self.current_heading.take()
            && level == capture.level
        {
            let text = capture.buffer.trim();
            if !text.is_empty() {
                self.record_heading(text, level, capture.id.clone());
            }
        }
    }

    fn observe<'a>(&mut self, event: &CoreEvent<'a>) {
        match event {
            CoreEvent::Start(CoreTag::Heading { level, id, .. }) => {
                let slug_from_tag = id.as_ref().map(|cow| cow.to_string());
                self.begin_heading(*level, slug_from_tag)
            }
            CoreEvent::End(CoreTagEnd::Heading(level)) => self.end_heading(*level),
            CoreEvent::Text(text)
            | CoreEvent::Code(text)
            | CoreEvent::Html(text)
            | CoreEvent::InlineHtml(text)
            | CoreEvent::InlineMath(text)
            | CoreEvent::DisplayMath(text) => self.push_heading_text(text.as_ref()),
            CoreEvent::SoftBreak | CoreEvent::HardBreak => self.push_heading_text(" "),
            _ => {}
        }
    }

    fn into_entries(self) -> Vec<HeadingEntry> {
        self.headings
    }
}

struct HeadingCapture {
    level: HeadingLevel,
    buffer: String,
    id: Option<String>,
}

/// Collects heading information from the raw markdown body.
///
/// This function parses the input `raw_body` and extracts all headings,
/// returning them as a vector of `HeadingEntry`. The `file_type`
/// parameter can be used to configure the parser for Markdown or MDX.
pub(crate) fn collect_headings(raw_body: &str, file_type: FileType) -> Result<Vec<HeadingEntry>> {
    let mut collector = HeadingCollector::new();
    let config = match file_type {
        FileType::Markdown => markflow_core::ParseConfig::markdown(),
        FileType::Mdx => markflow_core::ParseConfig::mdx(),
    };
    let event_iterator =
        markflow_core::get_event_iterator_with_config(raw_body, config).map_err(convert_error)?;

    for event in event_iterator {
        collector.observe(&event);
    }

    Ok(collector.into_entries())
}
