use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

use crate::Span;
use crate::event::{Event, Tag, TagEnd};
use crate::transform::directives::rewrite_with_mapper;
use crate::transform::directives::{DirectiveMapper, DirectiveOpening};

/// Rewrites directive markers (`:::note`, etc.) into Aside HTML events while streaming.
pub struct DirectiveAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    inner: I,
    pending: VecDeque<Event<'a>>,
    stack: Vec<DirectiveOpening>,
    directive_count: Rc<RefCell<usize>>,
    in_code_block: usize,
    mapper: Arc<dyn DirectiveMapper + Send + Sync>,
    required_imports: Rc<RefCell<Vec<String>>>,
}

impl<'a, I> DirectiveAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub(crate) fn new(
        inner: I,
        directive_count: Rc<RefCell<usize>>,
        mapper: Arc<dyn DirectiveMapper + Send + Sync>,
        required_imports: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            stack: Vec::new(),
            directive_count,
            in_code_block: 0,
            mapper,
            required_imports,
        }
    }

    fn push_unclosed_closers(&mut self) {
        while let Some(opened) = self.stack.pop() {
            if let Some(transform) = self.mapper.map_opening(&opened) {
                self.pending
                    .push_back(Event::Html(transform.end_tag.into(), Span::default()));
            } else {
                self.pending
                    .push_back(Event::Html("</div>".into(), Span::default()));
            }
        }
    }

    fn capture_paragraph(&mut self, start: Event<'a>) -> Vec<Event<'a>> {
        let mut buf = Vec::new();
        buf.push(start);

        for ev in self.inner.by_ref() {
            let is_end = matches!(ev, Event::End(TagEnd::Paragraph, _));
            buf.push(ev.clone());
            if is_end {
                break;
            }
        }

        buf
    }
}

impl<'a, I> Iterator for DirectiveAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ev) = self.pending.pop_front() {
            return Some(ev);
        }

        if let Some(event) = self.inner.next() {
            match event {
                Event::Start(Tag::CodeBlock(_), _) => {
                    self.in_code_block += 1;
                    return Some(event);
                }
                Event::End(TagEnd::CodeBlock, _) => {
                    self.in_code_block = self.in_code_block.saturating_sub(1);
                    return Some(event);
                }
                Event::Start(Tag::Paragraph, span) if self.in_code_block == 0 => {
                    let paragraph_events =
                        self.capture_paragraph(Event::Start(Tag::Paragraph, span));
                    let mut content = String::new();
                    for ev in paragraph_events.iter() {
                        match ev {
                            Event::Text(t, _) if !t.as_ref().starts_with('\0') => {
                                content.push_str(t.as_ref());
                            }
                            Event::SoftBreak(_) => content.push('\n'),
                            _ => {}
                        }
                    }

                    let (rewritten, count, imports) = rewrite_with_mapper(&content, &*self.mapper);
                    if count > 0 {
                        *self.directive_count.borrow_mut() += count;
                        self.required_imports.borrow_mut().extend(imports);
                        self.pending.push_back(Event::Html(rewritten.into(), span));
                        return self.pending.pop_front();
                    }
                    // Not a directive marker; emit original paragraph.
                    for ev in paragraph_events {
                        self.pending.push_back(ev);
                    }
                    return self.pending.pop_front();
                }
                _ => {
                    return Some(event);
                }
            }
        }

        // No more inner events; close any unclosed directives.
        if !self.stack.is_empty() {
            self.push_unclosed_closers();
            return self.pending.pop_front();
        }

        None
    }
}
