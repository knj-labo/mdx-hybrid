//! Streaming HTML rewriter glue that feeds markdown HTML into lol_html without buffering.

use lol_html::errors::RewritingError;
use lol_html::{HtmlRewriter, OutputSink, Selector, Settings, element};
use std::borrow::Cow;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::Arc;

use crate::transform::directives::{AsideDirectiveMapper, DirectiveMapper};

/// Configuration flags that control how the streaming rewriter manipulates HTML.
#[derive(Clone)]
pub struct RewriteOptions {
    /// When enabled, missing `loading` attributes on `<img>` tags are defaulted to `lazy`.
    pub enforce_img_loading_lazy: bool,
    /// Mapper used to rewrite directives; boxed to allow dynamic dispatch.
    pub directive_mapper: Arc<dyn DirectiveMapper + Send + Sync>,
    /// Collected imports required by the mapper (populated during streaming).
    pub required_imports: Rc<RefCell<Vec<String>>>,
}

impl Default for RewriteOptions {
    fn default() -> Self {
        RewriteOptions {
            enforce_img_loading_lazy: true,
            directive_mapper: Arc::new(AsideDirectiveMapper::new()),
            required_imports: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// Implements [`Write`] so the streaming events API (via the `MarkdownStream` trait) can push raw HTML directly into lol_html.
///
/// Internally we share the destination writer through a single `Rc<RefCell<Option<W>>>`, which is
/// the only heap allocation required to satisfy lol_html's `OutputSink` trait without buffering
/// large chunks of rewritten output.
pub struct StreamingRewriter<W: Write> {
    rewriter: Option<HtmlRewriter<'static, OutputProxy<W>>>,
    target: Rc<RefCell<Option<W>>>,
    sink_error: Rc<RefCell<Option<io::Error>>>,
}

impl<W: Write> StreamingRewriter<W> {
    /// Creates a new streaming rewriter that forwards lol_html output into `writer` while applying
    /// the supplied rewrite options.
    pub fn new(writer: W, options: RewriteOptions) -> Self {
        let target = Rc::new(RefCell::new(Some(writer)));
        let sink_error = Rc::new(RefCell::new(None));
        let output_sink = OutputProxy::new(Rc::clone(&target), Rc::clone(&sink_error));
        let settings = options.as_settings();
        let rewriter = HtmlRewriter::new(settings, output_sink);

        Self {
            rewriter: Some(rewriter),
            target,
            sink_error,
        }
    }

    /// Consumes the rewriter, ensures lol_html has flushed, and returns the underlying writer.
    pub fn into_inner(mut self) -> io::Result<W> {
        self.finalize_if_needed()?;

        let cell =
            Rc::try_unwrap(self.target).map_err(|_| io::Error::other("rewriter still borrowed"))?;

        cell.into_inner()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "writer missing"))
    }

    fn finalize_if_needed(&mut self) -> io::Result<()> {
        if let Some(rewriter) = self.rewriter.take() {
            rewriter.end().map_err(rewriting_error_to_io)?;
        }

        Self::take_sink_error(&self.sink_error)
    }

    fn take_sink_error(cell: &Rc<RefCell<Option<io::Error>>>) -> io::Result<()> {
        if let Some(err) = cell.borrow_mut().take() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl<W: Write> Write for StreamingRewriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let rewriter = self
            .rewriter
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "rewriter finalized"))?;

        rewriter.write(buf).map_err(rewriting_error_to_io)?;
        Self::take_sink_error(&self.sink_error)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.finalize_if_needed()
    }
}

impl RewriteOptions {
    fn as_settings(&self) -> Settings<'static, 'static> {
        let mut settings = Settings::default();
        let mut handlers = Vec::new();

        if self.enforce_img_loading_lazy {
            handlers.push(lazy_img_handler());
        }

        settings.element_content_handlers = handlers;
        settings
    }
}

fn lazy_img_handler() -> (
    Cow<'static, Selector>,
    lol_html::ElementContentHandlers<'static>,
) {
    element!("img", |el| {
        if el.get_attribute("loading").is_none() {
            el.set_attribute("loading", "lazy")?;
        }

        Ok(())
    })
}

fn rewriting_error_to_io(err: RewritingError) -> io::Error {
    io::Error::other(err)
}

struct OutputProxy<W: Write> {
    target: Rc<RefCell<Option<W>>>,
    sink_error: Rc<RefCell<Option<io::Error>>>,
}

impl<W: Write> OutputProxy<W> {
    fn new(target: Rc<RefCell<Option<W>>>, sink_error: Rc<RefCell<Option<io::Error>>>) -> Self {
        OutputProxy { target, sink_error }
    }
}

impl<W: Write> OutputSink for OutputProxy<W> {
    fn handle_chunk(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        if self.sink_error.borrow().is_some() {
            return;
        }

        let mut borrow = self.target.borrow_mut();

        if let Some(writer) = borrow.as_mut()
            && let Err(err) = writer.write_all(chunk)
        {
            *self.sink_error.borrow_mut() = Some(err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn adds_lazy_loading_when_missing() {
        let mut rewriter = StreamingRewriter::new(Vec::new(), RewriteOptions::default());
        rewriter
            .write_all(br#"<img src="/hero.png">"#)
            .expect("stream write should succeed");
        let output = String::from_utf8(rewriter.into_inner().unwrap()).unwrap();

        assert!(output.contains("loading=\"lazy\""));
    }

    #[test]
    fn preserves_existing_loading_attribute() {
        let mut rewriter = StreamingRewriter::new(Vec::new(), RewriteOptions::default());
        rewriter
            .write_all(br#"<img src="/hero.png" loading="eager">"#)
            .expect("stream write should succeed");
        let output = String::from_utf8(rewriter.into_inner().unwrap()).unwrap();

        assert!(output.contains("loading=\"eager\""));
    }
}
