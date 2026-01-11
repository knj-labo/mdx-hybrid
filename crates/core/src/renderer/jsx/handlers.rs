use super::{
    Block, MarkflowError, RewriteOptions, dedent_one_level, render_block_into,
    render_markdown_events,
};
use std::collections::HashMap;

pub trait ComponentHandler {
    fn tag_name(&self) -> &'static str;
    fn render_children(
        &self,
        children: &[Block<'_>],
        rewrite_options: &RewriteOptions,
        handlers: &ComponentHandlers,
        output: &mut String,
    ) -> Result<(), MarkflowError>;
}

pub struct ComponentHandlers {
    handlers: HashMap<&'static str, Box<dyn ComponentHandler>>,
}

impl ComponentHandlers {
    pub fn new() -> Self {
        let mut handlers: HashMap<&'static str, Box<dyn ComponentHandler>> = HashMap::new();
        let steps = StepsHandler;
        let filetree = FileTreeHandler;
        handlers.insert(steps.tag_name(), Box::new(steps));
        handlers.insert(filetree.tag_name(), Box::new(filetree));
        Self { handlers }
    }

    pub fn render_children(
        &self,
        name: &str,
        children: &[Block<'_>],
        rewrite_options: &RewriteOptions,
        output: &mut String,
    ) -> Result<bool, MarkflowError> {
        if let Some(handler) = self.handlers.get(name) {
            handler.render_children(children, rewrite_options, self, output)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

struct StepsHandler;

impl ComponentHandler for StepsHandler {
    fn tag_name(&self) -> &'static str {
        "Steps"
    }

    fn render_children(
        &self,
        children: &[Block<'_>],
        rewrite_options: &RewriteOptions,
        handlers: &ComponentHandlers,
        output: &mut String,
    ) -> Result<(), MarkflowError> {
        let mut scratch = String::new();
        for child in children {
            scratch.clear();
            match child {
                Block::Markdown(text) => {
                    let dedented = dedent_one_level(text);
                    render_markdown_events(&dedented, rewrite_options, &mut scratch)?;
                    append_steps_fragment(output, &scratch);
                }
                _ => {
                    render_block_into(child, rewrite_options, handlers, &mut scratch)?;
                    insert_into_last_list_item(output, &scratch);
                }
            }
        }
        Ok(())
    }
}

struct FileTreeHandler;

impl ComponentHandler for FileTreeHandler {
    fn tag_name(&self) -> &'static str {
        "FileTree"
    }

    fn render_children(
        &self,
        children: &[Block<'_>],
        rewrite_options: &RewriteOptions,
        handlers: &ComponentHandlers,
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
                        render_markdown_events(&markdown_buffer, rewrite_options, &mut inner)?;
                        markdown_buffer.clear();
                    }
                    render_block_into(child, rewrite_options, handlers, &mut inner)?;
                }
            }
        }

        if !markdown_buffer.is_empty() {
            render_markdown_events(&markdown_buffer, rewrite_options, &mut inner)?;
        }

        output.push_str(&extract_first_unordered_list(&inner));
        Ok(())
    }
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
