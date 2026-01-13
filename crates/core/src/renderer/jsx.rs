#![allow(missing_docs)]
use crate::error::ParseDiagnostics;
use crate::event::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};
use crate::renderer::multipass::ScanEvent;
use crate::transform::code_fence::collect_root_imports;
use crate::{DirectiveAdapter, HoistAdapter, MarkflowError, RewriteOptions};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::rc::Rc;

mod components;
pub use components::{ComponentRegistry, JsxComponentPlugin, RenderContext, RenderOutcome};

/// Options for JSX rendering.
#[derive(Default)]
pub struct JsxOptions {
    /// Rewrite options for directive/hoist/smartypants behavior.
    pub rewrite_options: RewriteOptions,
    /// Component registry (built-ins + plugins).
    pub components: ComponentRegistry,
}

/// Result of JSX rendering with diagnostics
pub struct JsxRenderResult {
    /// The rendered JSX string
    pub jsx: String,
    /// Parse diagnostics (warnings)
    pub diagnostics: ParseDiagnostics,
}

/// Render Markdown input into a raw JSX-like string, preserving JSX nodes.
pub fn render_to_jsx(input: &str) -> Result<String, MarkflowError> {
    render_to_jsx_with_options(input, JsxOptions::default())
}

/// Render Markdown input into a raw JSX-like string with custom options.
pub fn render_to_jsx_with_options(
    input: &str,
    options: JsxOptions,
) -> Result<String, MarkflowError> {
    let result = render_to_jsx_with_options_full(input, options)?;
    Ok(result.jsx)
}

/// Render Markdown input into a raw JSX-like string with custom options and diagnostics.
pub fn render_to_jsx_with_options_full(
    input: &str,
    options: JsxOptions,
) -> Result<JsxRenderResult, MarkflowError> {
    let rewrite_options = options.rewrite_options;
    let components = options.components;
    let mut seen_imports = HashSet::new();
    let (root_imports, _body_lines) = collect_root_imports(input);
    let preprocessed = rewrite_html_wrapper_components(input);
    let ctx = RenderContext::new(&rewrite_options, &components);
    let mut renderer = JsxStreamRenderer::new(ctx);
    let mut body = String::with_capacity(preprocessed.len());
    let diagnostics = Rc::new(RefCell::new(ParseDiagnostics::new()));
    let mut stream = crate::renderer::multipass::ScanIter::new_with_diagnostics(
        &preprocessed,
        Rc::clone(&diagnostics),
    );
    while let Some(event) = stream.next() {
        renderer.render_event(event, &mut stream, &mut body)?;
    }

    let mut output = String::with_capacity(body.len() + input.len());
    if rewrite_options.enable_hoist {
        for import in root_imports {
            if seen_imports.insert(import.clone()) {
                output.push_str(&import);
                output.push('\n');
            }
        }
    }
    for import in rewrite_options.required_imports.borrow().iter() {
        if seen_imports.insert(import.clone()) {
            output.push_str(import);
            output.push('\n');
        }
    }
    output.push_str(&body);
    let jsx = restore_html_wrapper_components(output);
    let diagnostics_result = Rc::try_unwrap(diagnostics)
        .map(|cell| cell.into_inner())
        .unwrap_or_else(|rc| rc.borrow().clone());
    Ok(JsxRenderResult {
        jsx,
        diagnostics: diagnostics_result,
    })
}

const HTML_WRAPPER_TAG: &str = "MfP";

fn rewrite_html_wrapper_components(input: &str) -> String {
    if !input.contains("<p") {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut in_fence = false;
    let mut fence_marker: Option<char> = None;
    let mut pending_wrapper: Option<WrapperBlock> = None;

    for line in input.split_inclusive('\n') {
        let (line_body, line_ending) = if let Some(stripped) = line.strip_suffix('\n') {
            let stripped = stripped.strip_suffix('\r').unwrap_or(stripped);
            (stripped, &line[stripped.len()..])
        } else {
            (line, "")
        };

        let trimmed_start = line_body.trim_start();
        let is_fence = trimmed_start.starts_with("```") || trimmed_start.starts_with("~~~");
        if is_fence {
            let marker = trimmed_start.chars().next();
            if in_fence {
                if marker == fence_marker {
                    in_fence = false;
                    fence_marker = None;
                }
            } else {
                in_fence = true;
                fence_marker = marker;
            }
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        }

        if let Some(wrapper) = pending_wrapper.as_mut() {
            wrapper.lines.push(format!("{line_body}{line_ending}"));
            if line_body.contains("</p>") {
                let mut lines = wrapper.lines.clone();
                if let Some(first) = lines.get_mut(0) {
                    *first = first.replacen("<p", "<MfP", 1);
                }
                for line in &mut lines {
                    if line.contains("</p>") {
                        *line = line.replace("</p>", "</MfP>");
                    }
                }
                for line in lines {
                    output.push_str(&line);
                }
                pending_wrapper = None;
            }
            continue;
        }

        if !in_fence {
            if is_p_open_tag(trimmed_start) {
                let line_text = format!("{line_body}{line_ending}");
                if line_body.contains("</p>") {
                    let replaced = line_body
                        .replacen("<p", "<MfP", 1)
                        .replace("</p>", "</MfP>");
                    output.push_str(&replaced);
                    output.push_str(line_ending);
                } else {
                    pending_wrapper = Some(WrapperBlock {
                        lines: vec![line_text],
                    });
                }
                continue;
            }
        }

        output.push_str(line_body);
        output.push_str(line_ending);
    }

    if let Some(wrapper) = pending_wrapper.take() {
        for line in wrapper.lines {
            output.push_str(&line);
        }
    }

    output
}

fn restore_html_wrapper_components(output: String) -> String {
    if !output.contains(HTML_WRAPPER_TAG) {
        return output;
    }
    output.replace("</MfP>", "</p>").replace("<MfP", "<p")
}

struct WrapperBlock {
    lines: Vec<String>,
}

fn is_p_open_tag(trimmed: &str) -> bool {
    if !trimmed.starts_with("<p") {
        return false;
    }
    if trimmed.starts_with("</p") {
        return false;
    }
    matches!(
        trimmed.as_bytes().get(2),
        Some(b'>') | Some(b' ') | Some(b'\t') | Some(b'/') | None
    )
}

// Intentionally left empty: previously used to gate wrapper rewrites.

fn render_markdown_events(
    input: &str,
    rewrite_options: &RewriteOptions,
    output: &mut String,
) -> Result<(), MarkflowError> {
    render_markdown_events_with_config(input, rewrite_options, output, crate::ParseConfig::mdx())
}

fn render_markdown_events_with_config(
    input: &str,
    rewrite_options: &RewriteOptions,
    output: &mut String,
    parse_config: crate::ParseConfig,
) -> Result<(), MarkflowError> {
    let hoisted = Rc::new(RefCell::new(Vec::new()));
    let mut events: Box<dyn Iterator<Item = Event<'static>>> =
        Box::new(crate::get_event_iterator_with_config(input, parse_config)?);
    if rewrite_options.enable_hoist {
        events = Box::new(HoistAdapter::new(events, Rc::clone(&hoisted)));
    }
    let events: Box<dyn Iterator<Item = Event<'static>>> = if rewrite_options.enable_directives {
        Box::new(DirectiveAdapter::new(
            events,
            Rc::new(RefCell::new(0)),
            rewrite_options.directive_mapper.clone(),
            rewrite_options.required_imports.clone(),
        ))
    } else {
        events
    };

    let mut stack: Vec<Tag<'_>> = Vec::new();
    let mut heading_stack: Vec<HeadingContext> = Vec::new();
    for event in events {
        match event {
            Event::Start(tag, _) => {
                if let Tag::Heading { level, id, .. } = &tag
                    && let Some(_id) = id.as_ref()
                {
                    write_heading_wrapper_start(output, *level);
                    heading_stack.push(HeadingContext {
                        text: String::new(),
                    });
                }
                write_start_tag(output, &tag);
                stack.push(tag);
            }
            Event::End(end, _) => {
                if let Some(open) = stack.pop()
                    && matches_end(&open, &end)
                {
                    write_end_tag(output, &end);
                    if let Tag::Heading { id, .. } = &open
                        && let Some(id) = id.as_ref()
                    {
                        let heading_text =
                            heading_stack.pop().map(|ctx| ctx.text).unwrap_or_default();
                        write_anchor_link(output, id.as_ref(), &heading_text);
                        output.push_str("</div>");
                    }
                }
            }
            Event::Text(text, _) => {
                let text = text.as_ref();
                if text.starts_with('\0') {
                    continue;
                }
                escape_text_into(output, text);
                push_heading_text(&mut heading_stack, text);
            }
            Event::Code(text, _) => {
                output.push_str("<code>");
                escape_text_into(output, text.as_ref());
                output.push_str("</code>");
                push_heading_text(&mut heading_stack, text.as_ref());
            }
            Event::Html(text, _) | Event::InlineHtml(text, _) => {
                output.push_str(text.as_ref());
            }
            Event::JsxInline(text, _) | Event::JsxFlow(text, _) => {
                output.push_str(text.as_ref());
            }
            Event::InlineMath(math, _) => {
                output.push_str("<span class=\"math-inline\">");
                output.push_str(math.as_ref());
                output.push_str("</span>");
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::DisplayMath(math, _) => {
                output.push_str("<div class=\"math-display\">");
                output.push_str(math.as_ref());
                output.push_str("</div>");
                push_heading_text(&mut heading_stack, math.as_ref());
            }
            Event::FootnoteReference(label, _) => {
                let _ = write!(
                    output,
                    "<sup class=\"footnote-ref\"><a href=\"#fn-{0}\" id=\"fnref-{0}\">{0}</a></sup>",
                    label
                );
                push_heading_text(&mut heading_stack, label.as_ref());
            }
            Event::TaskListMarker(done, _) => {
                if done {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" checked=\"\" />");
                } else {
                    output.push_str("<input type=\"checkbox\" disabled=\"\" />");
                }
            }
            Event::Rule(_) => output.push_str("<hr />\n"),
            Event::HardBreak(_) => {
                output.push_str("<br />\n");
                push_heading_text(&mut heading_stack, " ");
            }
            Event::SoftBreak(_) => {
                output.push('\n');
                push_heading_text(&mut heading_stack, " ");
            }
        }
    }

    Ok(())
}

pub struct JsxStreamRenderer<'a> {
    ctx: RenderContext<'a>,
    depth: usize,
}

impl<'a> JsxStreamRenderer<'a> {
    pub fn new(ctx: RenderContext<'a>) -> Self {
        Self { ctx, depth: 0 }
    }

    pub fn child(&self) -> Self {
        Self {
            ctx: self.ctx,
            depth: self.depth + 1,
        }
    }

    pub fn require_import(&self, import: impl Into<String>) {
        self.ctx.require_import(import);
    }

    pub fn render_markdown(&self, input: &str, output: &mut String) -> Result<(), MarkflowError> {
        if self.depth == 0 {
            return render_markdown_events(input, self.ctx.rewrite_options, output);
        }

        let mut dedented = input.to_string();
        for _ in 0..self.depth {
            dedented = dedent_one_level(&dedented);
        }
        render_markdown_events(&dedented, self.ctx.rewrite_options, output)
    }

    pub fn render_event(
        &mut self,
        event: ScanEvent<'a>,
        stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
        output: &mut String,
    ) -> Result<(), MarkflowError> {
        match event {
            ScanEvent::Markdown { text, .. } => self.render_markdown(text, output),
            ScanEvent::Code { text, .. } => self.render_markdown(text, output),
            ScanEvent::JsxOpen {
                name,
                attrs,
                is_self_closing,
                ..
            } => {
                let components = self.ctx.components;

                // Check if component is registered as code sample component
                let is_code_sample = components.is_code_sample_component(name);

                // Let plugins handle first
                if components.render_stream(&event, stream, self, output)? {
                    return Ok(());
                }

                let rendered_attrs = render_attrs(attrs, is_self_closing);
                output.push('<');
                output.push_str(name);
                output.push_str(&rendered_attrs);

                if is_self_closing {
                    output.push_str(" />");
                } else {
                    output.push('>');

                    // If code sample component, collect children as raw text
                    if is_code_sample {
                        let raw_children = collect_raw_children_until_close(name, stream);
                        output.push_str(&raw_children);
                        // Write closing tag since we consumed it from the stream
                        output.push_str("</");
                        output.push_str(name);
                        output.push('>');
                    } else {
                        self.depth = self.depth.saturating_add(1);
                    }
                }
                Ok(())
            }
            ScanEvent::JsxClose { name, .. } => {
                if self.depth > 0 {
                    self.depth -= 1;
                }
                output.push_str("</");
                output.push_str(name);
                output.push('>');
                Ok(())
            }
        }
    }
}

/// Collects raw children text until the matching close tag without markdown processing.
/// Handles nested components with the same name by tracking depth.
fn collect_raw_children_until_close<'a>(
    target_name: &str,
    stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
) -> String {
    let mut output = String::new();
    let mut depth = 0usize;

    while let Some(event) = stream.next() {
        match event {
            ScanEvent::JsxOpen {
                name,
                attrs,
                is_self_closing,
                ..
            } => {
                if name == target_name {
                    depth += 1;
                }
                // Reconstruct JSX syntax
                output.push('<');
                output.push_str(name);
                if !attrs.is_empty() {
                    output.push(' ');
                    output.push_str(attrs);
                }
                if is_self_closing {
                    output.push_str(" />");
                } else {
                    output.push('>');
                }
            }
            ScanEvent::JsxClose { name, .. } => {
                if name == target_name {
                    if depth == 0 {
                        // Found matching close tag
                        break;
                    }
                    depth -= 1;
                }
                // Reconstruct closing tag
                output.push_str("</");
                output.push_str(name);
                output.push('>');
            }
            ScanEvent::Markdown { text, .. } | ScanEvent::Code { text, .. } => {
                // Append raw text without processing
                output.push_str(text);
            }
        }
    }

    output
}

fn dedent_one_level(input: &str) -> String {
    let mut any = false;
    let mut can_strip_spaces = true;
    let mut can_strip_tabs = true;

    for line in input.split_inclusive('\n') {
        if line.trim().is_empty() {
            continue;
        }
        any = true;
        if !line.starts_with("    ") {
            can_strip_spaces = false;
        }
        if !line.starts_with('\t') {
            can_strip_tabs = false;
        }
    }

    if !any {
        return input.to_string();
    }

    let strip_spaces = if can_strip_spaces {
        Some(4)
    } else if can_strip_tabs {
        Some(1)
    } else {
        None
    };

    let Some(strip) = strip_spaces else {
        return input.to_string();
    };

    let mut output = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        if strip == 4 && line.starts_with("    ") {
            output.push_str(&line[4..]);
        } else if strip == 1 && line.starts_with('\t') {
            output.push_str(&line[1..]);
        } else {
            output.push_str(line);
        }
    }
    output
}

pub(super) fn render_attrs(attrs: &str, is_self_closing: bool) -> String {
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

fn write_start_tag(output: &mut String, tag: &Tag<'_>) {
    match tag {
        Tag::Paragraph => output.push_str("<p>"),
        Tag::Heading {
            level,
            id,
            classes,
            attrs,
        } => {
            let _ = write!(output, "<h{}", *level as u8);
            if let Some(id) = id {
                output.push_str(" id=\"");
                output.push_str(id);
                output.push('"');
            }
            if !classes.is_empty() {
                output.push_str(" class=\"");
                for (idx, class) in classes.iter().enumerate() {
                    if idx > 0 {
                        output.push(' ');
                    }
                    output.push_str(class);
                }
                output.push('"');
            }
            for (key, value) in attrs {
                match value {
                    Some(v) => {
                        output.push(' ');
                        output.push_str(key);
                        output.push_str("=\"");
                        output.push_str(v);
                        output.push('"');
                    }
                    None => {
                        output.push(' ');
                        output.push_str(key);
                    }
                }
            }
            output.push('>');
        }
        Tag::BlockQuote => output.push_str("<blockquote>"),
        Tag::CodeBlock(kind) => match kind {
            CodeBlockKind::Indented => output.push_str("<pre><code>"),
            CodeBlockKind::Fenced(lang) => {
                output.push_str("<pre><code class=\"language-");
                output.push_str(lang);
                output.push_str("\">");
            }
        },
        Tag::List(start) => {
            if start.is_some() {
                output.push_str("<ol>");
            } else {
                output.push_str("<ul>");
            }
        }
        Tag::Item => output.push_str("<li>"),
        Tag::FootnoteDefinition(label) => {
            output.push_str("<section class=\"footnote\" id=\"fn-");
            output.push_str(label);
            output.push_str("\">");
        }
        Tag::Table(_) => output.push_str("<table>"),
        Tag::TableHead => output.push_str("<thead>"),
        Tag::TableRow => output.push_str("<tr>"),
        Tag::TableCell => output.push_str("<td>"),
        Tag::Emphasis => output.push_str("<em>"),
        Tag::Strong => output.push_str("<strong>"),
        Tag::Strikethrough => output.push_str("<del>"),
        Tag::Link {
            dest_url, title, ..
        } => {
            output.push_str("<a href=\"");
            output.push_str(dest_url);
            output.push('"');
            if !title.is_empty() {
                output.push_str(" title=\"");
                output.push_str(title);
                output.push('"');
            }
            output.push('>');
        }
        Tag::Image { .. } => {} // handled elsewhere in HTML renderer; skip here
    }
}

fn write_end_tag(output: &mut String, end: &TagEnd) {
    match end {
        TagEnd::Paragraph => output.push_str("</p>\n"),
        TagEnd::Heading(level) => {
            let _ = write!(output, "</h{}>", *level as u8);
        }
        TagEnd::BlockQuote => output.push_str("</blockquote>\n"),
        TagEnd::CodeBlock => output.push_str("</code></pre>\n"),
        TagEnd::List(ordered) => {
            if *ordered {
                output.push_str("</ol>\n");
            } else {
                output.push_str("</ul>\n");
            }
        }
        TagEnd::Item => output.push_str("</li>"),
        TagEnd::FootnoteDefinition => output.push_str("</section>\n"),
        TagEnd::Table => output.push_str("</table>\n"),
        TagEnd::TableHead => output.push_str("</thead>\n"),
        TagEnd::TableRow => output.push_str("</tr>\n"),
        TagEnd::TableCell => output.push_str("</td>"),
        TagEnd::Emphasis => output.push_str("</em>"),
        TagEnd::Strong => output.push_str("</strong>"),
        TagEnd::Strikethrough => output.push_str("</del>"),
        TagEnd::Link => output.push_str("</a>"),
        TagEnd::Image => {}
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

fn write_heading_wrapper_start(output: &mut String, level: HeadingLevel) {
    let _ = write!(
        output,
        "<div class=\"sl-heading-wrapper level-h{}\">",
        level as u8
    );
}

fn write_anchor_link(output: &mut String, id: &str, heading_text: &str) {
    output.push_str("<a class=\"sl-anchor-link\" href=\"#");
    output.push_str(id);
    output.push_str("\"><span aria-hidden=\"true\" class=\"sl-anchor-icon\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\"><path fill=\"currentcolor\"
  d=\"m12.11 15.39-3.88 3.88a2.52 2.52 0 0 1-3.5 0 2.47 2.47 0 0 1 0-3.5l3.88-3.88a1 1 0 0 0-1.42-1.42l-3.88 3.89a4.48 4.48 0 0 0 6.33 6.33l3.89-3.88a1 1 0 1 0-
  1.42-1.42Zm8.58-12.08a4.49 4.49 0 0 0-6.33 0l-3.89 3.88a1 1 0 0 0 1.42 1.42l3.88-3.88a2.52 2.52 0 0 1 3.5 0 2.47 2.47 0 0 1 0 3.5l-3.88 3.88a1 1 0 1 0 1.42
  1.42l3.88-3.89a4.49 4.49 0 0 0 0-6.33ZM8.83 15.17a1 1 0 0 0 1.1.22 1 1 0 0 0 .32-.22l4.92-4.92a1 1 0 0 0-1.42-1.42l-4.92 4.92a1 1 0 0 0 0 1.42Z\"></path></svg></
  span><span class=\"sr-only\">");
    output.push_str("Section titled \"");
    escape_text_into(output, heading_text.trim());
    output.push_str("\"</span></a>");
}

fn push_heading_text(heading_stack: &mut [HeadingContext], text: &str) {
    if let Some(current) = heading_stack.last_mut() {
        current.text.push_str(text);
    }
}

fn escape_text_into(out: &mut String, text: &str) {
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
}
