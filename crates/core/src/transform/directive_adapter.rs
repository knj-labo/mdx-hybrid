use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::directives::{DirectiveOpening, rewrite_directives_to_asides};
use crate::event::{Event, Tag, TagEnd};

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
}

impl<'a, I> DirectiveAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub(crate) fn new(inner: I, directive_count: Rc<RefCell<usize>>) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            stack: Vec::new(),
            directive_count,
            in_code_block: 0,
        }
    }

    fn push_unclosed_closers(&mut self) {
        while let Some(opened) = self.stack.pop() {
            self.pending
                .push_back(Event::Html(opened.to_aside_end().into()));
        }
    }

    fn capture_paragraph(&mut self, start: Event<'a>) -> Vec<Event<'a>> {
        let mut buf = Vec::new();
        buf.push(start);

        for ev in self.inner.by_ref() {
            let is_end = matches!(ev, Event::End(TagEnd::Paragraph));
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
            match &event {
                Event::Start(Tag::CodeBlock(_)) => {
                    self.in_code_block += 1;
                    return Some(event);
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.in_code_block = self.in_code_block.saturating_sub(1);
                    return Some(event);
                }
                Event::Start(Tag::Paragraph) if self.in_code_block == 0 => {
                    let paragraph_events = self.capture_paragraph(event);
                    let mut content = String::new();
                    for ev in paragraph_events.iter() {
                        match ev {
                            Event::Text(t) => content.push_str(t.as_ref()),
                            Event::SoftBreak => content.push('\n'),
                            _ => {}
                        }
                    }

                    let (rewritten, count) = rewrite_directives_to_asides(&content);
                    if count > 0 {
                        *self.directive_count.borrow_mut() += count;
                        self.pending.push_back(Event::Html(rewritten.into()));
                    } else {
                        // Not a directive marker; emit original paragraph.
                        for ev in paragraph_events {
                            self.pending.push_back(ev);
                        }
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
