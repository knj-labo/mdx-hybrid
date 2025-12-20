#![allow(missing_docs)]
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::convert::TryFrom;

use html_escape::encode_text_to_string;
use log::warn;
use markdown::{MdxEsmParse, MdxSignal, ParseOptions, mdast, message::Message, to_mdast};

use crate::event::{Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Tag};
use crate::parser::parse_config::ParseConfig;
use crate::slug::Slugger;

pub struct MarkdownParser {
    stack: Vec<Frame>,
    pending_events: VecDeque<Event<'static>>,
    tight_list_stack: Vec<usize>,
    source: String,
    definitions: HashMap<String, DefinitionTarget>,
    #[allow(dead_code)]
    config: ParseConfig,
    slugger: Slugger,
}

struct DefinitionTarget {
    url: String,
    title: Option<String>,
}

impl MarkdownParser {
    pub fn new(input: &str) -> Result<Self, Message> {
        Self::new_with_config(input, ParseConfig::default())
    }

    pub fn new_with_config(input: &str, config: ParseConfig) -> Result<Self, Message> {
        let options = build_parse_options(config);
        let tree = to_mdast(input, &options)?;

        let mut iter = Self {
            stack: Vec::new(),
            pending_events: VecDeque::new(),
            tight_list_stack: Vec::new(),
            source: input.to_owned(),
            definitions: HashMap::new(),
            config,
            slugger: Slugger::new(),
        };
        iter.collect_definitions(&tree);
        iter.push_node(tree);
        Ok(iter)
    }

    fn push_node(&mut self, node: mdast::Node) {
        match node {
            mdast::Node::Root(root) => self.stack.push(Frame::transparent(root.children)),
            mdast::Node::Paragraph(paragraph) => {
                if self.is_direct_child_of_tight_list() {
                    self.stack.push(Frame::transparent(paragraph.children));
                } else {
                    self.stack
                        .push(Frame::container(Tag::Paragraph, paragraph.children));
                }
            }
            mdast::Node::Heading(heading) => {
                let heading_id = self.heading_slug(&heading.children);
                let tag = Tag::Heading {
                    level: HeadingLevel::try_from(heading.depth as usize)
                        .unwrap_or(HeadingLevel::H6),
                    id: heading_id.map(Cow::Owned),
                    classes: Vec::new(),
                    attrs: Vec::new(),
                };
                self.stack.push(Frame::container(tag, heading.children));
            }
            mdast::Node::Blockquote(block) => {
                self.stack
                    .push(Frame::container(Tag::BlockQuote, block.children));
            }
            mdast::Node::List(list) => {
                let start = if list.ordered {
                    let raw = list.start.unwrap_or(1);
                    Some(if raw < 1 { 1 } else { raw } as u64)
                } else {
                    None
                };
                self.stack
                    .push(Frame::container(Tag::List(start), list.children));
            }
            mdast::Node::ListItem(item) => {
                let mut frame = Frame::container(Tag::Item, item.children);
                if let Some(checked) = item.checked {
                    frame
                        .pending_after_start
                        .push_back(Event::TaskListMarker(checked));
                }
                if !item.spread {
                    frame.tight_state = TightState::PendingIncrement;
                }
                self.stack.push(frame);
            }
            mdast::Node::FootnoteDefinition(def) => {
                let tag = Tag::FootnoteDefinition(Cow::Owned(def.identifier));
                self.stack.push(Frame::container(tag, def.children));
            }
            mdast::Node::ThematicBreak(_) => self.pending_events.push_back(Event::Rule),
            mdast::Node::Code(code) => self.push_code_block(code),
            mdast::Node::Text(text) => {
                self.pending_events
                    .push_back(Event::Text(Cow::Owned(text.value)));
            }
            mdast::Node::Emphasis(node) => {
                self.stack
                    .push(Frame::container(Tag::Emphasis, node.children));
            }
            mdast::Node::Strong(node) => {
                self.stack
                    .push(Frame::container(Tag::Strong, node.children));
            }
            mdast::Node::Delete(node) => {
                self.stack
                    .push(Frame::container(Tag::Strikethrough, node.children));
            }
            mdast::Node::InlineCode(code) => {
                self.pending_events
                    .push_back(Event::Code(Cow::Owned(code.value)));
            }
            mdast::Node::InlineMath(math) => {
                self.pending_events
                    .push_back(Event::InlineMath(Cow::Owned(math.value)));
            }
            mdast::Node::Math(math) => {
                self.pending_events
                    .push_back(Event::DisplayMath(Cow::Owned(math.value)));
            }
            mdast::Node::Break(_) => self.pending_events.push_back(Event::HardBreak),
            mdast::Node::Link(link) => {
                let tag = Tag::Link {
                    link_type: LinkType::Inline,
                    dest_url: Cow::Owned(link.url),
                    title: link.title.map_or_else(|| Cow::Borrowed(""), Cow::Owned),
                    id: Cow::Owned(String::new()),
                };
                self.stack.push(Frame::container(tag, link.children));
            }
            mdast::Node::LinkReference(link) => {
                let target = self.lookup_definition(&link.identifier);
                let tag = Tag::Link {
                    link_type: LinkType::Reference,
                    dest_url: target
                        .map(|t| Cow::Owned(t.url.clone()))
                        .unwrap_or(Cow::Borrowed("")),
                    title: target
                        .and_then(|t| t.title.clone())
                        .map(Cow::Owned)
                        .unwrap_or(Cow::Borrowed("")),
                    id: Cow::Owned(link.identifier),
                };
                self.stack.push(Frame::container(tag, link.children));
            }
            mdast::Node::Image(image) => self.push_inline_image(image),
            mdast::Node::ImageReference(image) => self.push_image_reference(image),
            mdast::Node::Html(html) => {
                self.pending_events
                    .push_back(Event::Html(Cow::Owned(html.value)));
            }
            mdast::Node::Table(table) => {
                let alignments = table
                    .align
                    .iter()
                    .map(|align| match align {
                        mdast::AlignKind::Left => Alignment::Left,
                        mdast::AlignKind::Right => Alignment::Right,
                        mdast::AlignKind::Center => Alignment::Center,
                        mdast::AlignKind::None => Alignment::None,
                    })
                    .collect();
                self.stack
                    .push(Frame::container(Tag::Table(alignments), table.children));
            }
            mdast::Node::TableRow(row) => {
                self.stack
                    .push(Frame::container(Tag::TableRow, row.children));
            }
            mdast::Node::TableCell(cell) => {
                self.stack
                    .push(Frame::container(Tag::TableCell, cell.children));
            }
            mdast::Node::Toml(doc) => {
                self.pending_events
                    .push_back(Event::Html(Cow::Owned(format_frontmatter(
                        "toml", &doc.value,
                    ))));
            }
            mdast::Node::Yaml(doc) => {
                self.pending_events
                    .push_back(Event::Html(Cow::Owned(format_frontmatter(
                        "yaml", &doc.value,
                    ))));
            }
            mdast::Node::MdxjsEsm(doc) => {
                self.pending_events
                    .push_back(Event::Html(Cow::Owned(doc.value)));
            }
            mdast::Node::FootnoteReference(reference) => {
                self.pending_events
                    .push_back(Event::FootnoteReference(Cow::Owned(reference.identifier)));
            }
            mdast::Node::MdxFlowExpression(expr) => {
                self.push_mdx_html(
                    expr.position.as_ref(),
                    Some(format!("{{{}}}", expr.value)),
                    true,
                    "mdxFlowExpression",
                );
            }
            mdast::Node::MdxTextExpression(expr) => {
                self.push_mdx_html(
                    expr.position.as_ref(),
                    Some(format!("{{{}}}", expr.value)),
                    false,
                    "mdxTextExpression",
                );
            }
            mdast::Node::MdxJsxFlowElement(element) => {
                let raw = element
                    .position
                    .as_ref()
                    .and_then(|pos| self.slice_from_position(pos))
                    .unwrap_or_default();
                self.pending_events
                    .push_back(Event::JsxFlow(Cow::Owned(raw)));
            }
            mdast::Node::MdxJsxTextElement(element) => {
                let raw = element
                    .position
                    .as_ref()
                    .and_then(|pos| self.slice_from_position(pos))
                    .unwrap_or_default();
                self.pending_events
                    .push_back(Event::JsxInline(Cow::Owned(raw)));
            }
            mdast::Node::Definition(_) => { /* already recorded */ }
        }
    }

    fn collect_definitions(&mut self, node: &mdast::Node) {
        match node {
            mdast::Node::Definition(def) => {
                let key = normalize_identifier(&def.identifier);
                self.definitions.insert(
                    key,
                    DefinitionTarget {
                        url: def.url.clone(),
                        title: def.title.clone(),
                    },
                );
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        self.collect_definitions(child);
                    }
                }
            }
        }
    }

    fn push_code_block(&mut self, code: mdast::Code) {
        let kind = match code.lang {
            Some(lang) => CodeBlockKind::Fenced(Cow::Owned(lang)),
            None => CodeBlockKind::Indented,
        };
        let tag = Tag::CodeBlock(kind);
        self.pending_events.push_back(Event::Start(tag.clone()));
        self.pending_events
            .push_back(Event::Text(Cow::Owned(code.value)));
        self.pending_events.push_back(Event::End(tag.to_end()));
    }

    fn push_inline_image(&mut self, image: mdast::Image) {
        let tag = Tag::Image {
            link_type: LinkType::Inline,
            dest_url: Cow::Owned(image.url),
            title: image.title.map_or_else(|| Cow::Borrowed(""), Cow::Owned),
            id: Cow::Owned(String::new()),
        };
        self.pending_events.push_back(Event::Start(tag.clone()));
        if !image.alt.is_empty() {
            self.pending_events
                .push_back(Event::Text(Cow::Owned(image.alt)));
        }
        self.pending_events.push_back(Event::End(tag.to_end()));
    }

    fn push_image_reference(&mut self, image: mdast::ImageReference) {
        let target = self.lookup_definition(&image.identifier);
        let tag = Tag::Image {
            link_type: LinkType::Reference,
            dest_url: target
                .map(|t| Cow::Owned(t.url.clone()))
                .unwrap_or(Cow::Borrowed("")),
            title: target
                .and_then(|t| t.title.clone())
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed("")),
            id: Cow::Owned(image.identifier),
        };
        self.pending_events.push_back(Event::Start(tag.clone()));
        if !image.alt.is_empty() {
            self.pending_events
                .push_back(Event::Text(Cow::Owned(image.alt)));
        }
        self.pending_events.push_back(Event::End(tag.to_end()));
    }

    fn lookup_definition(&self, identifier: &str) -> Option<&DefinitionTarget> {
        self.definitions.get(&normalize_identifier(identifier))
    }

    fn warn_unsupported(&self, node_name: &str) {
        warn!("Skipping unsupported markdown node: {node_name}");
    }

    fn push_mdx_html(
        &mut self,
        position: Option<&markdown::unist::Position>,
        fallback: Option<String>,
        block: bool,
        label: &str,
    ) {
        if let Some(position) = position
            && let Some(raw) = self.slice_from_position(position)
        {
            let event = if block {
                Event::Html(Cow::Owned(raw))
            } else {
                Event::InlineHtml(Cow::Owned(raw))
            };
            self.pending_events.push_back(event);
            return;
        }

        if let Some(fallback) = fallback {
            let event = if block {
                Event::Html(Cow::Owned(fallback))
            } else {
                Event::InlineHtml(Cow::Owned(fallback))
            };
            self.pending_events.push_back(event);
            return;
        }

        self.warn_unsupported(label);
    }

    fn slice_from_position(&self, position: &markdown::unist::Position) -> Option<String> {
        let start = position.start.offset;
        let end = position.end.offset;
        if start <= end && end <= self.source.len() {
            Some(self.source[start..end].to_string())
        } else {
            None
        }
    }

    fn is_direct_child_of_tight_list(&self) -> bool {
        self.tight_list_stack
            .last()
            .is_some_and(|depth| *depth == self.stack.len())
    }

    fn heading_slug(&mut self, children: &[mdast::Node]) -> Option<String> {
        let mut raw = String::new();
        collect_text(children, &mut raw);
        let slug = self.slugger.next_slug(raw.trim());
        if slug.is_empty() { None } else { Some(slug) }
    }
}

impl Iterator for MarkdownParser {
    type Item = Event<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.pending_events.pop_front() {
                return Some(event);
            }

            let current_depth = self.stack.len();
            if current_depth == 0 {
                return None;
            }

            let frame = self.stack.last_mut()?;

            match frame.kind {
                FrameKind::Transparent => {
                    if let Some(child) = frame.children.pop_front() {
                        self.push_node(child);
                    } else {
                        self.stack.pop();
                    }
                }
                FrameKind::Container(_) => match frame.phase {
                    FramePhase::Start => {
                        frame.phase = FramePhase::Prologue;
                        if let FrameKind::Container(tag) = &frame.kind {
                            return Some(Event::Start(tag.clone()));
                        }
                    }
                    FramePhase::Prologue => {
                        if let Some(event) = frame.pending_after_start.pop_front() {
                            return Some(event);
                        }
                        frame.phase = FramePhase::Children;
                        if matches!(frame.tight_state, TightState::PendingIncrement) {
                            self.tight_list_stack.push(current_depth);
                            frame.tight_state = TightState::Active;
                        }
                    }
                    FramePhase::Children => {
                        if let Some(child) = frame.children.pop_front() {
                            self.push_node(child);
                        } else {
                            frame.phase = FramePhase::End;
                            if matches!(frame.tight_state, TightState::Active) {
                                self.tight_list_stack.pop();
                                frame.tight_state = TightState::None;
                            }
                        }
                    }
                    FramePhase::End => {
                        let end = match &frame.kind {
                            FrameKind::Container(tag) => tag.to_end(),
                            FrameKind::Transparent => {
                                unreachable!("transparent frames never emit closing tags")
                            }
                        };
                        self.stack.pop();
                        return Some(Event::End(end));
                    }
                },
            }
        }
    }
}

struct Frame {
    kind: FrameKind,
    phase: FramePhase,
    children: VecDeque<mdast::Node>,
    pending_after_start: VecDeque<Event<'static>>,
    tight_state: TightState,
}

enum FrameKind {
    Container(Tag<'static>),
    Transparent,
}

#[derive(Clone, Copy)]
enum FramePhase {
    Start,
    Prologue,
    Children,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TightState {
    None,
    PendingIncrement,
    Active,
}

impl Frame {
    fn container(tag: Tag<'static>, children: Vec<mdast::Node>) -> Self {
        Self {
            kind: FrameKind::Container(tag),
            phase: FramePhase::Start,
            children: VecDeque::from(children),
            pending_after_start: VecDeque::new(),
            tight_state: TightState::None,
        }
    }

    fn transparent(children: Vec<mdast::Node>) -> Self {
        Self {
            kind: FrameKind::Transparent,
            phase: FramePhase::Children,
            children: VecDeque::from(children),
            pending_after_start: VecDeque::new(),
            tight_state: TightState::None,
        }
    }
}

fn collect_text(nodes: &[mdast::Node], buf: &mut String) {
    for node in nodes {
        match node {
            mdast::Node::Text(text) => buf.push_str(&text.value),
            mdast::Node::InlineCode(code) => buf.push_str(&code.value),
            mdast::Node::Code(code) => buf.push_str(&code.value),
            mdast::Node::Strong(_)
            | mdast::Node::Emphasis(_)
            | mdast::Node::Delete(_)
            | mdast::Node::Link(_)
            | mdast::Node::LinkReference(_)
            | mdast::Node::Paragraph(_)
            | mdast::Node::Heading(_)
            | mdast::Node::Blockquote(_)
            | mdast::Node::ListItem(_)
            | mdast::Node::List(_)
            | mdast::Node::MdxJsxFlowElement(_)
            | mdast::Node::MdxJsxTextElement(_)
            | mdast::Node::Root(_)
            | mdast::Node::Table(_)
            | mdast::Node::TableRow(_)
            | mdast::Node::TableCell(_)
            | mdast::Node::FootnoteDefinition(_)
            | mdast::Node::Image(_)
            | mdast::Node::ImageReference(_) => {
                if let Some(children) = node.children() {
                    collect_text(children, buf);
                }
            }
            _ => {}
        }
    }
}

fn format_frontmatter(kind: &str, value: &str) -> String {
    let mut output = String::new();
    output.push_str("<pre class=\"frontmatter\" data-kind=\"");
    output.push_str(kind);
    output.push_str("\">");
    encode_text_to_string(value, &mut output);
    output.push_str("</pre>");
    output
}

fn normalize_identifier(id: &str) -> String {
    id.trim().to_lowercase()
}

fn build_parse_options(config: ParseConfig) -> ParseOptions {
    let mut options = ParseOptions::gfm();
    options.constructs.frontmatter = true;
    options.constructs.math_flow = true;
    options.constructs.math_text = true;

    options.constructs.mdx_esm = config.constructs.esm;
    options.constructs.mdx_expression_flow = config.constructs.expression;
    options.constructs.mdx_expression_text = config.constructs.expression;
    options.constructs.mdx_jsx_flow = config.constructs.jsx;
    options.constructs.mdx_jsx_text = config.constructs.jsx;

    if config.constructs.jsx {
        options.constructs.html_flow = false;
        options.constructs.html_text = false;
    } else {
        options.constructs.html_flow = true;
        options.constructs.html_text = true;
    }

    options.mdx_esm_parse = if config.constructs.esm {
        Some(Box::new(parse_mdx_esm_ok) as Box<MdxEsmParse>)
    } else {
        None
    };

    options
}

fn parse_mdx_esm_ok(_: &str) -> MdxSignal {
    MdxSignal::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event as MfEvent, Tag, TagEnd};
    use crate::parser::parse_config::ParseConfig;

    fn collect_events(input: &str) -> Vec<Event<'static>> {
        MarkdownParser::new(input).unwrap().collect()
    }

    fn collect_events_with_config(input: &str, config: ParseConfig) -> Vec<Event<'static>> {
        MarkdownParser::new_with_config(input, config)
            .unwrap()
            .collect()
    }

    #[test]
    fn tight_list_strips_paragraphs() {
        let events = collect_events("* foo\n* bar\n");
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, MfEvent::Start(Tag::Paragraph)))
        );
    }

    #[test]
    fn loose_list_retains_paragraphs() {
        let events = collect_events("* foo\n\n  bar\n");
        assert!(
            events
                .iter()
                .any(|event| matches!(event, MfEvent::Start(Tag::Paragraph)))
        );
    }

    #[test]
    fn task_list_emits_marker_before_text() {
        let events = collect_events("- [x] done");
        let marker_index = events
            .iter()
            .position(|event| matches!(event, MfEvent::TaskListMarker(true)))
            .expect("marker present");
        let text_index = events
            .iter()
            .position(|event| matches!(event, MfEvent::Text(text) if text.as_ref() == "done"))
            .expect("text present");
        assert!(marker_index < text_index);
    }

    #[test]
    fn tight_list_preserves_blockquote_paragraphs() {
        let events = collect_events("* > foo");
        let mut inside_blockquote = false;
        let mut found_paragraph = false;
        for event in events {
            match event {
                MfEvent::Start(Tag::BlockQuote) => inside_blockquote = true,
                MfEvent::End(TagEnd::BlockQuote) => inside_blockquote = false,
                MfEvent::Start(Tag::Paragraph) if inside_blockquote => {
                    found_paragraph = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(found_paragraph, "paragraph inside blockquote should remain");
    }

    #[test]
    fn markdown_profile_treats_import_as_text() {
        let input = "import Button from './Button.jsx'";
        let events = collect_events_with_config(input, ParseConfig::markdown());
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Text(text) if text.contains("import Button"))),
            "import statement should remain as text: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::Html(html) if html.contains("import Button"))),
            "markdown mode should not emit HTML events for imports"
        );
    }

    #[test]
    fn mdx_profile_emits_html_for_imports() {
        let input = "import Button from './Button.jsx'";
        let events = collect_events_with_config(input, ParseConfig::mdx());
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Html(html) if html.contains("import Button"))),
            "mdx mode should surface mdx ESM blocks"
        );
    }

    #[test]
    fn markdown_profile_keeps_inline_expression_as_text() {
        let input = "Hello {props.name}";
        let events = collect_events_with_config(input, ParseConfig::markdown());
        assert!(
            events
                .iter()
                .any(|event| matches!(event, Event::Text(text) if text.contains("{props.name}"))),
            "expression should remain literal text in markdown mode"
        );
    }

    #[test]
    fn mdx_profile_emits_inline_expression_nodes() {
        let input = "Hello {props.name}";
        let events = collect_events_with_config(input, ParseConfig::mdx());
        assert!(
            events.iter().any(|event| matches!(
                event,
                Event::InlineHtml(html) if html.contains("{props.name}")
            )),
            "mdx mode should emit inline HTML for expressions"
        );
    }
}
