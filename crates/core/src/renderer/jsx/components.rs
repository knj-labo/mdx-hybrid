use std::collections::HashMap;

use super::{
    Block, MarkflowError, RewriteOptions, dedent_one_level, render_block_into,
    render_markdown_events,
};

/// Lightweight view of a JSX element emitted by the multipass scanner.
#[derive(Debug, Clone, Copy)]
pub struct JsxElement<'a> {
    /// Element tag name.
    pub name: &'a str,
    /// Raw attribute string from the source.
    pub attrs: &'a str,
    /// Child blocks for the element.
    pub children: &'a [Block<'a>],
    /// Whether the element is self-closing.
    pub is_self_closing: bool,
}

/// Shared context passed to JSX component plugins.
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

    /// Renders nested blocks using the same component registry.
    pub fn render_children(
        &self,
        children: &[Block<'_>],
        output: &mut String,
    ) -> Result<(), MarkflowError> {
        super::render_jsx_children_into(children, self, output)
    }

    /// Renders a single block using the same component registry.
    pub fn render_block(
        &self,
        block: &Block<'_>,
        output: &mut String,
    ) -> Result<(), MarkflowError> {
        render_block_into(block, self, output)
    }

    /// Renders Markdown text into HTML using the current rewrite options.
    pub fn render_markdown(&self, input: &str, output: &mut String) -> Result<(), MarkflowError> {
        render_markdown_events(input, self.rewrite_options, output)
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
    /// Optionally renders the component's children or performs side effects.
    fn render_children(
        &self,
        element: &JsxElement<'_>,
        ctx: &RenderContext<'_>,
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

    pub(crate) fn render_children(
        &self,
        element: &JsxElement<'_>,
        ctx: &RenderContext<'_>,
        output: &mut String,
    ) -> Result<bool, MarkflowError> {
        if let Some(import) = self.import_map.get(element.name) {
            ctx.require_import(import.clone());
        }

        for plugin in &self.plugins {
            if plugin.matches(element.name) {
                match plugin.render_children(element, ctx, output)? {
                    RenderOutcome::Handled => return Ok(true),
                    RenderOutcome::Skipped => {}
                }
            }
        }

        match element.name {
            "Steps" => {
                render_steps_children(element.children, ctx, output)?;
                Ok(true)
            }
            "FileTree" => {
                render_file_tree_children(element.children, ctx, output)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn render_steps_children(
    children: &[Block<'_>],
    ctx: &RenderContext<'_>,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut scratch = String::new();
    for child in children {
        scratch.clear();
        match child {
            Block::Markdown(text) => {
                let dedented = dedent_one_level(text);
                ctx.render_markdown(&dedented, &mut scratch)?;
                append_steps_fragment(output, &scratch);
            }
            _ => {
                ctx.render_block(child, &mut scratch)?;
                insert_into_last_list_item(output, &scratch);
            }
        }
    }

    Ok(())
}

fn render_file_tree_children(
    children: &[Block<'_>],
    ctx: &RenderContext<'_>,
    output: &mut String,
) -> Result<(), MarkflowError> {
    let mut inner = String::new();
    let mut markdown_buffer = String::new();

    for child in children {
        match child {
            Block::Markdown(text) => {
                markdown_buffer.push_str(&dedent_one_level(text));
            }
            _ => {
                if !markdown_buffer.is_empty() {
                    ctx.render_markdown(&markdown_buffer, &mut inner)?;
                    markdown_buffer.clear();
                }
                ctx.render_block(child, &mut inner)?;
            }
        }
    }

    if !markdown_buffer.is_empty() {
        ctx.render_markdown(&markdown_buffer, &mut inner)?;
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
                match bytes[end] {
                    b'>' => return Some((pos, end + 1 - pos)),
                    b' ' | b'\t' | b'\n' | b'\r' => {
                        end += 1;
                        continue;
                    }
                    _ => break,
                }
            }
        }
        pos += 1;
    }
    None
}
