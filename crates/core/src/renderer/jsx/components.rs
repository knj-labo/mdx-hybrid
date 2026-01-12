use std::collections::HashMap;

use crate::renderer::multipass::ScanEvent;

use super::{JsxStreamRenderer, MarkflowError, RewriteOptions, render_attrs};

/// Shared context passed to JSX component plugins.
#[derive(Clone, Copy)]
pub struct RenderContext<'a> {
    pub(super) rewrite_options: &'a RewriteOptions,
    pub(super) components: &'a ComponentRegistry,
}

impl<'a> RenderContext<'a> {
    pub(super) fn new(
        rewrite_options: &'a RewriteOptions,
        components: &'a ComponentRegistry,
    ) -> Self {
        Self {
            rewrite_options,
            components,
        }
    }

    /// Registers an additional import that should be hoisted into the output.
    pub fn require_import(&self, import: impl Into<String>) {
        self.rewrite_options
            .required_imports
            .borrow_mut()
            .push(import.into());
    }
}

/// Outcome of plugin rendering.
pub enum RenderOutcome {
    /// The plugin handled rendering of the children.
    Handled,
    /// The plugin chose not to handle the element.
    Skipped,
}

/// Plugin hook for customizing JSX component rendering.
pub trait JsxComponentPlugin {
    /// Returns true if the plugin wants to handle the given component name.
    fn matches(&self, name: &str) -> bool;
    /// Optionally renders the component stream or performs side effects.
    fn render_stream<'a>(
        &self,
        event: &ScanEvent<'a>,
        stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
        renderer: &mut JsxStreamRenderer<'a>,
        output: &mut String,
    ) -> Result<RenderOutcome, MarkflowError>;
}

/// Registry that manages built-in JSX component handlers plus optional plugins.
#[derive(Default)]
pub struct ComponentRegistry {
    plugins: Vec<Box<dyn JsxComponentPlugin>>,
    import_map: HashMap<String, String>,
}

impl ComponentRegistry {
    /// Creates a new registry without additional plugins.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a plugin handler for JSX components.
    pub fn register_plugin(&mut self, plugin: impl JsxComponentPlugin + 'static) {
        self.plugins.push(Box::new(plugin));
    }

    /// Registers a component import mapping (tag name -> import statement).
    pub fn register_import(&mut self, name: impl Into<String>, import: impl Into<String>) {
        self.import_map.insert(name.into(), import.into());
    }

    pub(crate) fn render_stream<'a>(
        &self,
        event: &ScanEvent<'a>,
        stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
        renderer: &mut JsxStreamRenderer<'a>,
        output: &mut String,
    ) -> Result<bool, MarkflowError> {
        let ScanEvent::JsxOpen {
            name,
            attrs,
            is_self_closing,
            ..
        } = event
        else {
            return Ok(false);
        };

        if let Some(import) = self.import_map.get(*name) {
            renderer.require_import(import.clone());
        }

        for plugin in &self.plugins {
            if plugin.matches(name) {
                let mut child = renderer.child();
                match plugin.render_stream(event, stream, &mut child, output)? {
                    RenderOutcome::Handled => return Ok(true),
                    RenderOutcome::Skipped => {}
                }
            }
        }

        match *name {
            "Steps" if !*is_self_closing => {
                let mut child = renderer.child();
                render_steps_stream(name, attrs, stream, &mut child, output)?;
                Ok(true)
            }
            "FileTree" if !*is_self_closing => {
                let mut child = renderer.child();
                render_file_tree_stream(stream, &mut child, output)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn render_steps_stream<'a>(
    name: &str,
    attrs: &str,
    stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
    renderer: &mut JsxStreamRenderer<'a>,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut inner = String::new();
    let mut scratch = String::new();
    while let Some(event) = stream.next() {
        match event {
            ScanEvent::JsxClose { name: "Steps", .. } => break,
            ScanEvent::Markdown { .. } => {
                scratch.clear();
                renderer.render_event(event, stream, &mut scratch)?;
                append_steps_fragment(&mut inner, &scratch);
            }
            _ => {
                scratch.clear();
                renderer.render_event(event, stream, &mut scratch)?;
                insert_into_last_list_item(&mut inner, &scratch);
            }
        }
    }
    output.push('<');
    output.push_str(name);
    output.push_str(&render_attrs(attrs, false));
    output.push('>');
    output.push_str(&inner);
    output.push_str("</");
    output.push_str(name);
    output.push('>');
    Ok(())
}

fn render_file_tree_stream<'a>(
    stream: &mut dyn Iterator<Item = ScanEvent<'a>>,
    renderer: &mut JsxStreamRenderer<'a>,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut inner = String::new();
    let mut markdown_buffer = String::new();

    while let Some(event) = stream.next() {
        match event {
            ScanEvent::JsxClose {
                name: "FileTree", ..
            } => break,
            ScanEvent::Markdown { text, .. } => {
                markdown_buffer.push_str(text);
            }
            _ => {
                if !markdown_buffer.is_empty() {
                    renderer.render_markdown(&markdown_buffer, &mut inner)?;
                    markdown_buffer.clear();
                }
                renderer.render_event(event, stream, &mut inner)?;
            }
        }
    }

    if !markdown_buffer.is_empty() {
        renderer.render_markdown(&markdown_buffer, &mut inner)?;
    }

    output.push_str(&extract_first_unordered_list(&inner));
    Ok(())
}

fn append_steps_fragment(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if output.is_empty() {
        output.push_str(fragment);
        return;
    }

    if let Some((pre, inner, post)) = split_ordered_list_fragment(fragment) {
        if !pre.trim().is_empty() {
            insert_into_last_list_item(output, pre);
        }
        if !inner.trim().is_empty() {
            insert_before_list_close(output, inner);
        }
        if !post.trim().is_empty() {
            insert_into_last_list_item(output, post);
        }
    } else if fragment.contains("<li") && !fragment.contains("<ol") {
        insert_before_list_close(output, fragment);
    } else {
        insert_into_last_list_item(output, fragment);
    }
}

fn split_ordered_list_fragment(fragment: &str) -> Option<(&str, &str, &str)> {
    let open_start = fragment.find("<ol")?;
    let open_end = fragment[open_start..].find('>')? + open_start;
    let close_start = fragment.rfind("</ol>")?;
    let close_end = close_start + "</ol>".len();
    if close_start < open_end {
        return None;
    }
    let pre = &fragment[..open_start];
    let inner = &fragment[open_end + 1..close_start];
    let post = &fragment[close_end..];
    Some((pre, inner, post))
}

fn insert_into_last_list_item(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if let Some(idx) = output.rfind("</li>") {
        output.insert_str(idx, fragment);
    } else {
        output.push_str(fragment);
    }
}

fn insert_before_list_close(output: &mut String, fragment: &str) {
    if fragment.is_empty() {
        return;
    }
    if let Some(idx) = output.rfind("</ol>") {
        output.insert_str(idx, fragment);
    } else {
        output.push_str(fragment);
    }
}

fn extract_first_unordered_list(input: &str) -> String {
    let bytes = input.as_bytes();
    let Some(start) = find_ul_open(bytes, 0) else {
        return input.to_string();
    };

    let mut depth = 0usize;
    let mut pos = start;
    while pos < bytes.len() {
        if let Some(open_pos) = find_ul_open(bytes, pos)
            && open_pos == pos
        {
            depth += 1;
            pos += 3;
            continue;
        }
        if let Some((close_pos, close_len)) = find_ul_close(bytes, pos)
            && close_pos == pos
        {
            depth = depth.saturating_sub(1);
            pos += close_len;
            if depth == 0 {
                let end = pos;
                return input[start..end].to_string();
            }
            continue;
        }
        pos += 1;
    }

    input.to_string()
}

fn find_ul_open(bytes: &[u8], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 2 < bytes.len() {
        if bytes[pos] == b'<' && bytes[pos + 1] == b'u' && bytes[pos + 2] == b'l' {
            let next = bytes.get(pos + 3).copied().unwrap_or(b'>');
            if matches!(next, b'>' | b' ' | b'\t' | b'\n' | b'\r') {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

fn find_ul_close(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut pos = start;
    while pos + 3 < bytes.len() {
        if bytes[pos] == b'<'
            && bytes[pos + 1] == b'/'
            && bytes[pos + 2] == b'u'
            && bytes[pos + 3] == b'l'
        {
            let mut end = pos + 4;
            while end < bytes.len() {
                if bytes[end] == b'>' {
                    return Some((pos, end - pos + 1));
                }
                end += 1;
            }
        }
        pos += 1;
    }
    None
}
