use std::cell::RefCell;
use std::rc::Rc;

use crate::event::{Event, Tag, TagEnd};

/// Adapts an event stream to hoist top-level `import`/`export` statements while streaming.
pub struct HoistAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    inner: I,
    hoisted: Rc<RefCell<Vec<String>>>,
    in_code_block: usize,
    depth: usize,
}

impl<'a, I> HoistAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub(crate) fn new(inner: I, hoisted: Rc<RefCell<Vec<String>>>) -> Self {
        Self {
            inner,
            hoisted,
            in_code_block: 0,
            depth: 0,
        }
    }
}

impl<'a, I> Iterator for HoistAdapter<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for event in self.inner.by_ref() {
            match &event {
                Event::Start(Tag::CodeBlock(_)) => {
                    self.in_code_block += 1;
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.in_code_block = self.in_code_block.saturating_sub(1);
                }
                Event::Start(_) => {
                    self.depth = self.depth.saturating_add(1);
                }
                Event::End(_) => {
                    self.depth = self.depth.saturating_sub(1);
                }
                Event::Html(text) if self.depth == 0 && self.in_code_block == 0 => {
                    let trimmed = text.trim_start();
                    if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                        self.hoisted.borrow_mut().push(trimmed.to_string());
                        continue;
                    }
                }
                _ => {}
            }

            return Some(event);
        }

        None
    }
}
