#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use lol_html::html_content::ContentType;
use lol_html::{element, rewrite_str, RewriteStrSettings};
use markdown::{CompileOptions, Options, ParseOptions, to_html_with_options};
use serde_json::to_string;

use crate::error::MarkflowError;
use crate::parser::markdown_adapter::normalize_mdx_jsx_indentation;
use crate::transform::code_fence::collect_root_imports;

const STARLIGHT_COMPONENTS_CSS: &str = include_str!("starlight_components.css");
static STARLIGHT_CSS_EMITTED: AtomicBool = AtomicBool::new(false);

pub fn render_to_html_mdast(input: &str) -> Result<String, MarkflowError> {
    let (root_imports, body_lines) = collect_root_imports(input);
    let body_input = body_lines.join("\n");
    let preprocessed = preprocess_directives(&body_input);
    let options = build_mdast_html_options();
    let normalized = normalize_mdx_jsx_indentation(&preprocessed);
    let html = to_html_with_options(&normalized, &options)
        .map_err(|err| MarkflowError::MarkdownAdapter(err.to_string()))?;
    let rewritten = rewrite_mdast_html(&html)?;
    let literal = to_string(&rewritten).unwrap_or_else(|_| "\"\"".to_string());
    let mut output = String::new();
    for import in root_imports {
        output.push_str(&import);
        output.push('\n');
    }
    if should_inject_starlight_css(&rewritten)
        && !STARLIGHT_CSS_EMITTED.swap(true, Ordering::SeqCst)
    {
        output.push_str("<style is:global>");
        output.push_str(STARLIGHT_COMPONENTS_CSS);
        output.push_str("</style>\n");
    }
    output.push_str(&format!("<Fragment set:html={{{}}} />", literal));
    Ok(output)
}

fn rewrite_mdast_html(input: &str) -> Result<String, MarkflowError> {
    MdxProcessor::new(input).process()
}

#[derive(Debug, Clone)]
struct AsideReplacement {
    class_name: String,
    label: String,
    inner_html: String,
}

#[derive(Debug, Clone)]
struct ChecklistReplacement {
    key: String,
    inner_html: String,
}

struct MdxProcessor {
    normalized: String,
    has_pre: bool,
    steps: Vec<String>,
    file_trees: Vec<String>,
    asides: Vec<AsideReplacement>,
    tabs: Vec<String>,
    astro_jsx_tabs: Vec<String>,
    package_tabs: Vec<String>,
    checklists: Vec<ChecklistReplacement>,
}

impl MdxProcessor {
    fn new(input: &str) -> Self {
        let normalized = unescape_target_tags(input);
        let steps = collect_tag_replacements_multi(&normalized, &["Steps", "steps"], normalize_steps_html);
        let file_trees = collect_tag_replacements_multi(
            &normalized,
            &["FileTree", "filetree"],
            normalize_file_tree_html,
        );
        let asides = collect_aside_replacements(&normalized);
        let tabs = collect_tag_replacements_multi(&normalized, &["Tabs", "tabs"], normalize_tabs_html);
        let astro_jsx_tabs = collect_tag_replacements_multi(
            &normalized,
            &["AstroJSXTabs", "astrojsxtabs"],
            normalize_tabs_html,
        );
        let package_tabs = collect_tag_replacements_multi(
            &normalized,
            &["PackageManagerTabs", "packagemanagertabs"],
            normalize_tabs_html,
        );
        let checklists = collect_checklist_replacements(&normalized);
        let has_pre = normalized.contains("<pre");

        Self {
            normalized,
            has_pre,
            steps,
            file_trees,
            asides,
            tabs,
            astro_jsx_tabs,
            package_tabs,
            checklists,
        }
    }

    fn process(self) -> Result<String, MarkflowError> {
        if self.steps.is_empty()
            && self.file_trees.is_empty()
            && self.asides.is_empty()
            && self.tabs.is_empty()
            && self.astro_jsx_tabs.is_empty()
            && self.package_tabs.is_empty()
            && self.checklists.is_empty()
            && !self.has_pre
        {
            return Ok(self.normalized);
        }

        let steps_queue = Rc::new(RefCell::new(VecDeque::from(self.steps)));
        let file_tree_queue = Rc::new(RefCell::new(VecDeque::from(self.file_trees)));
        let aside_queue = Rc::new(RefCell::new(VecDeque::from(self.asides)));
        let tabs_queue = Rc::new(RefCell::new(VecDeque::from(self.tabs)));
        let astro_jsx_tabs_queue = Rc::new(RefCell::new(VecDeque::from(self.astro_jsx_tabs)));
        let package_tabs_queue = Rc::new(RefCell::new(VecDeque::from(self.package_tabs)));
        let checklist_queue = Rc::new(RefCell::new(VecDeque::from(self.checklists)));

        let rewritten = rewrite_str(
            &self.normalized,
            RewriteStrSettings {
                element_content_handlers: vec![
                    {
                        let steps_queue = steps_queue.clone();
                        element!("Steps, steps", move |el| {
                            if let Some(next) = steps_queue.borrow_mut().pop_front() {
                                let inner = extract_ordered_list_inner(&next).unwrap_or(next);
                                let _ = el.set_tag_name("ol");
                                let class = el.get_attribute("class").unwrap_or_default();
                                let class = append_class(&class, "sl-steps");
                                let _ = el.set_attribute("class", &class);
                                let _ = el.set_attribute("role", "list");
                                if let Some(start_value) = el.get_attribute("start") {
                                    if let Ok(start) = start_value.trim().parse::<i64>() {
                                        let offset = start.saturating_sub(1);
                                        let style = format!("--sl-steps-start: {}", offset);
                                        let existing = el.get_attribute("style").unwrap_or_default();
                                        let next_style = if existing.trim().is_empty() {
                                            style
                                        } else {
                                            format!("{};{}", existing, style)
                                        };
                                        let _ = el.set_attribute("style", &next_style);
                                    }
                                }
                                el.set_inner_content(&inner, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let file_tree_queue = file_tree_queue.clone();
                        element!("FileTree, filetree", move |el| {
                            if let Some(next) = file_tree_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("starlight-file-tree");
                                let _ = el.set_attribute("class", "not-content");
                                let _ = el.set_attribute("data-pagefind-ignore", "true");
                                el.set_inner_content(&next, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let aside_queue = aside_queue.clone();
                        element!("Aside, aside", move |el| {
                            if let Some(next) = aside_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("aside");
                                let _ = el.set_attribute("class", &next.class_name);
                                let _ = el.set_attribute("aria-label", &next.label);
                                el.set_inner_content(&next.inner_html, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let tabs_queue = tabs_queue.clone();
                        element!("Tabs, tabs", move |el| {
                            if let Some(next) = tabs_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("div");
                                let _ = el.set_attribute("class", "starlight-tabs");
                                el.set_inner_content(&next, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let astro_jsx_tabs_queue = astro_jsx_tabs_queue.clone();
                        element!("AstroJSXTabs, astrojsxtabs", move |el| {
                            if let Some(next) = astro_jsx_tabs_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("div");
                                let _ = el.set_attribute("class", "starlight-tabs");
                                el.set_inner_content(&next, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let package_tabs_queue = package_tabs_queue.clone();
                        element!("PackageManagerTabs, packagemanagertabs", move |el| {
                            if let Some(next) = package_tabs_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("div");
                                let _ = el.set_attribute("class", "starlight-tabs");
                                el.set_inner_content(&next, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        let checklist_queue = checklist_queue.clone();
                        element!("Checklist, checklist", move |el| {
                            if let Some(next) = checklist_queue.borrow_mut().pop_front() {
                                let _ = el.set_tag_name("check-list");
                                let _ = el.set_attribute("data-key", &next.key);
                                el.set_inner_content(&next.inner_html, ContentType::Html);
                            }
                            Ok(())
                        })
                    },
                    {
                        element!("pre", |el| {
                            let class = el.get_attribute("class").unwrap_or_default();
                            if !class.split_whitespace().any(|name| name == "astro-code") {
                                let mut next = class;
                                if !next.is_empty() {
                                    next.push(' ');
                                }
                                next.push_str("astro-code");
                                let _ = el.set_attribute("class", &next);
                            }
                            Ok(())
                        })
                    },
                ],
                ..RewriteStrSettings::new()
            },
        )
        .map_err(|err| MarkflowError::MarkdownAdapter(err.to_string()))?;

        let rewritten = rewrite_tabs_only(&rewritten)?;
        Ok(ensure_pre_class(&rewritten))
    }
}

struct TagScanner<'a> {
    bytes: &'a [u8],
}

impl<'a> TagScanner<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
        }
    }

    fn find_tag_end(&self, start: usize) -> Option<usize> {
        let mut pos = start;
        while pos < self.bytes.len() {
            if self.bytes[pos] == b'>' {
                return Some(pos);
            }
            pos += 1;
        }
        None
    }

    fn matches_tag_at(&self, pos: usize, tag: &[u8]) -> bool {
        if pos + tag.len() > self.bytes.len() {
            return false;
        }
        if &self.bytes[pos..pos + tag.len()] != tag {
            return false;
        }
        let next = self.bytes.get(pos + tag.len()).copied().unwrap_or(b'>');
        matches!(next, b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r')
    }

    fn find_open_tag_multi(
        &self,
        pos: usize,
        pairs: &[(Vec<u8>, Vec<u8>)],
    ) -> Option<(usize, usize)> {
        for (idx, (open, _)) in pairs.iter().enumerate() {
            if self.matches_tag_at(pos, open) {
                if let Some(end) = self.find_tag_end(pos) {
                    return Some((idx, end));
                }
            }
        }
        None
    }

    fn find_matching_close_multi(
        &self,
        mut pos: usize,
        closes: &[&[u8]],
    ) -> Option<(usize, usize)> {
        while pos < self.bytes.len() {
            for close in closes {
                if self.matches_tag_at(pos, close) {
                    if let Some(end) = self.find_tag_end(pos) {
                        return Some((pos, end + 1));
                    }
                }
            }
            pos += 1;
        }
        None
    }
}

struct AttributeReader<'a> {
    tag: &'a str,
}

impl<'a> AttributeReader<'a> {
    fn new(tag: &'a str) -> Self {
        Self { tag }
    }

    fn value(&self, name: &str) -> Option<String> {
        parse_attribute_value(self.tag, name)
    }
}

fn collect_tag_replacements_multi<F>(input: &str, tags: &[&str], normalize: F) -> Vec<String>
where
    F: Fn(&str) -> String,
{
    let mut replacements = Vec::new();
    let scanner = TagScanner::new(input);
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = tags
        .iter()
        .map(|tag| (format!("<{}", tag).into_bytes(), format!("</{}", tag).into_bytes()))
        .collect();
    let mut pos = 0;
    let mut depth = 0usize;
    let mut inner_start = 0usize;
    let mut active_idx: Option<usize> = None;

    while pos < scanner.bytes.len() {
        if let Some((idx, open_end)) = scanner.find_open_tag_multi(pos, &pairs) {
            depth += 1;
            if depth == 1 {
                inner_start = open_end + 1;
                active_idx = Some(idx);
            }
            pos = open_end + 1;
            continue;
        }
        if let Some(idx) = active_idx {
            if scanner.matches_tag_at(pos, &pairs[idx].1) {
                if let Some(close_end) = scanner.find_tag_end(pos) {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 && inner_start <= pos {
                        let inner = &input[inner_start..pos];
                        replacements.push(normalize(inner));
                        active_idx = None;
                    }
                    pos = close_end + 1;
                    continue;
                }
            }
        }
        pos += 1;
    }

    replacements
}

fn preprocess_directives(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 64);
    let mut in_fence = false;
    let mut fence_char = b'\0';
    let mut fence_len = 0usize;
    let mut aside_open = false;

    for line in input.split_inclusive('\n') {
        let (line_body, line_ending) = split_line_ending(line);

        if let Some((marker, count)) = fence_marker(line_body) {
            if !in_fence {
                in_fence = true;
                fence_char = marker;
                fence_len = count;
            } else if marker == fence_char && count >= fence_len {
                in_fence = false;
            }
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        }

        if in_fence {
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        }

        let Some(candidate) = strip_directive_indent(line_body) else {
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        };

        if !candidate.starts_with(":::") {
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        }

        let rest = candidate[3..].trim();
        if rest.is_empty() {
            if aside_open {
                output.push_str("</Aside>");
                output.push_str(line_ending);
                aside_open = false;
            } else {
                output.push_str(line_body);
                output.push_str(line_ending);
            }
            continue;
        }

        let Some((kind, title, inline_body, inline_close)) = parse_directive_line(rest) else {
            output.push_str(line_body);
            output.push_str(line_ending);
            continue;
        };

        if inline_close {
            output.push_str("<Aside type=\"");
            output.push_str(&kind);
            output.push('"');
            if let Some(title) = title {
                output.push_str(" title=\"");
                output.push_str(&escape_attr(&title));
                output.push('"');
            }
            output.push('>');
            if let Some(inline_body) = inline_body {
                output.push_str(&inline_body);
            }
            output.push_str("</Aside>");
            output.push_str(line_ending);
        } else {
            if aside_open {
                output.push_str("</Aside>");
                output.push_str(line_ending);
            }

            output.push_str("<Aside type=\"");
            output.push_str(&kind);
            output.push('"');
            if let Some(title) = title {
                output.push_str(" title=\"");
                output.push_str(&escape_attr(&title));
                output.push('"');
            }
            output.push('>');
            output.push_str(line_ending);
            aside_open = true;
        }
    }

    if aside_open {
        output.push_str("</Aside>\n");
    }

    output
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix('\n') {
        if let Some(body) = body.strip_suffix('\r') {
            return (body, "\r\n");
        }
        return (body, "\n");
    }
    (line, "")
}

fn fence_marker(line: &str) -> Option<(u8, usize)> {
    let bytes = line.as_bytes();
    let mut pos = 0usize;
    let mut spaces = 0usize;
    while pos < bytes.len() {
        match bytes[pos] {
            b' ' => {
                spaces += 1;
                if spaces > 3 {
                    return None;
                }
                pos += 1;
            }
            b'\t' => {
                pos += 1;
                break;
            }
            _ => break,
        }
    }

    let marker = *bytes.get(pos)?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let mut count = 0usize;
    while pos < bytes.len() && bytes[pos] == marker {
        count += 1;
        pos += 1;
    }
    if count >= 3 {
        Some((marker, count))
    } else {
        None
    }
}

fn strip_directive_indent(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut pos = 0usize;
    let mut spaces = 0usize;
    while pos < bytes.len() {
        match bytes[pos] {
            b' ' => {
                spaces += 1;
                if spaces > 3 {
                    return None;
                }
                pos += 1;
            }
            b'\t' => {
                pos += 1;
                break;
            }
            _ => break,
        }
    }
    Some(&line[pos..])
}

fn parse_directive_line(rest: &str) -> Option<(String, Option<String>, Option<String>, bool)> {
    let mut rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    let mut inline_close = false;
    let trimmed = rest.trim_end();
    if trimmed.ends_with(":::") {
        inline_close = true;
        rest = trimmed[..trimmed.len() - 3].trim_end();
    } else {
        rest = trimmed;
    }

    if rest.is_empty() {
        return None;
    }

    let mut end = rest.len();
    for (idx, ch) in rest.char_indices() {
        if ch.is_whitespace() || ch == '[' {
            end = idx;
            break;
        }
    }
    let kind = rest[..end].trim();
    let kind = kind.to_ascii_lowercase();
    if !matches!(kind.as_str(), "tip" | "note" | "caution" | "danger" | "info") {
        return None;
    }

    let mut title = None;
    let mut remaining = rest[end..].trim();
    if remaining.starts_with('[') {
        if let Some(close) = remaining.find(']') {
            let inner = remaining[1..close].trim();
            if !inner.is_empty() {
                title = Some(inner.to_string());
            }
            remaining = remaining[close + 1..].trim();
        }
    }

    let mut inline_body = None;
    if inline_close {
        if !remaining.is_empty() {
            inline_body = Some(remaining.to_string());
        }
    } else if title.is_none() && !remaining.is_empty() {
        title = Some(remaining.to_string());
    } else if title.is_some() && !remaining.is_empty() {
        inline_body = Some(remaining.to_string());
    }

    Some((kind, title, inline_body, inline_close))
}

fn normalize_steps_html(input: &str) -> String {
    let rendered = render_fragment_markdown(input);
    let mut output = String::new();
    let mut remainder = rendered.as_str();

    while let Some((pre, open_tag, inner, post)) = split_first_ordered_list(remainder) {
        if output.is_empty() {
            output.push_str(open_tag);
            output.push_str(inner);
            output.push_str("</ol>");
        } else {
            insert_before_last_ol_close(&mut output, inner);
        }
        if !pre.trim().is_empty() {
            insert_into_last_list_item(&mut output, pre);
        }
        remainder = post;
    }

    if output.is_empty() {
        return input.to_string();
    }

    if !remainder.trim().is_empty() {
        insert_into_last_list_item(&mut output, remainder);
    }

    output
}

fn normalize_file_tree_html(input: &str) -> String {
    let rendered = render_fragment_markdown(input);
    extract_first_unordered_list(&rendered)
}

fn collect_aside_replacements(input: &str) -> Vec<AsideReplacement> {
    collect_tag_replacements_with_open_multi(input, &["Aside", "aside"], normalize_aside_html)
}

fn collect_checklist_replacements(input: &str) -> Vec<ChecklistReplacement> {
    collect_tag_replacements_with_open_multi(input, &["Checklist", "checklist"], normalize_checklist_html)
}

fn normalize_checklist_html(open_tag: &str, inner: &str) -> ChecklistReplacement {
    let attrs = AttributeReader::new(open_tag);
    let key = attrs.value("key").unwrap_or_else(|| "default".to_string());
    let mut inner_html = String::new();
    inner_html.push_str("<div class=\"checklist\">");
    inner_html.push_str(&render_fragment_markdown(inner));
    inner_html.push_str("</div>");
    ChecklistReplacement { key, inner_html }
}

fn normalize_aside_html(open_tag: &str, inner: &str) -> AsideReplacement {
    let attrs = AttributeReader::new(open_tag);
    let aside_type = attrs
        .value("type")
        .unwrap_or_else(|| "note".to_string());
    let title = attrs
        .value("title")
        .unwrap_or_else(|| capitalize_word(&aside_type));
    let class_name = format!("starlight-aside starlight-aside--{}", aside_type);
    let label = escape_attr(&title);
    let mut inner_html = String::new();
    inner_html.push_str("<p class=\"starlight-aside__title\" aria-hidden=\"true\">");
    inner_html.push_str(&escape_html(&title));
    inner_html.push_str("</p><div class=\"starlight-aside__content\">");
    inner_html.push_str(&render_fragment_markdown(inner));
    inner_html.push_str("</div>");
    AsideReplacement {
        class_name,
        label,
        inner_html,
    }
}

fn normalize_tabs_html(input: &str) -> String {
    let fragments = collect_fragments(input);
    if fragments.is_empty() {
        return render_fragment_markdown(input);
    }
    let mut output = String::new();
    for (slot, fragment) in fragments {
        output.push_str("<div class=\"starlight-tabs__panel\"");
        if let Some(slot) = slot {
            output.push_str(" data-tab=\"");
            output.push_str(&escape_attr(&slot));
            output.push_str("\"");
        }
        output.push_str(">");
        output.push_str(&render_fragment_markdown(&fragment));
        output.push_str("</div>");
    }
    output
}

fn collect_fragments(input: &str) -> Vec<(Option<String>, String)> {
    let mut fragments = Vec::new();
    let scanner = TagScanner::new(input);
    let open_upper = b"<Fragment";
    let open_lower = b"<fragment";
    let close_upper = b"</Fragment";
    let close_lower = b"</fragment";
    let mut pos = 0usize;

    while pos < scanner.bytes.len() {
        if scanner.matches_tag_at(pos, open_upper) || scanner.matches_tag_at(pos, open_lower) {
            if let Some(open_end) = scanner.find_tag_end(pos) {
                let open_tag = &input[pos..=open_end];
                let slot = AttributeReader::new(open_tag).value("slot");
                if let Some((close_start, close_end)) =
                    scanner.find_matching_close_multi(open_end + 1, &[close_upper, close_lower])
                {
                    let inner = &input[open_end + 1..close_start];
                    fragments.push((slot, inner.to_string()));
                    pos = close_end;
                    continue;
                }
            }
        }
        pos += 1;
    }

    fragments
}


fn collect_tag_replacements_with_open_multi<T, F>(
    input: &str,
    tags: &[&str],
    normalize: F,
) -> Vec<T>
where
    F: Fn(&str, &str) -> T,
{
    let mut replacements = Vec::new();
    let scanner = TagScanner::new(input);
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = tags
        .iter()
        .map(|tag| (format!("<{}", tag).into_bytes(), format!("</{}", tag).into_bytes()))
        .collect();
    let mut pos = 0usize;
    let mut depth = 0usize;
    let mut inner_start = 0usize;
    let mut open_tag = String::new();
    let mut active_idx: Option<usize> = None;

    while pos < scanner.bytes.len() {
        if let Some((idx, open_end)) = scanner.find_open_tag_multi(pos, &pairs) {
            depth += 1;
            if depth == 1 {
                inner_start = open_end + 1;
                open_tag = input[pos..=open_end].to_string();
                active_idx = Some(idx);
            }
            pos = open_end + 1;
            continue;
        }
        if let Some(idx) = active_idx {
            if scanner.matches_tag_at(pos, &pairs[idx].1) {
                if let Some(close_end) = scanner.find_tag_end(pos) {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 && inner_start <= pos {
                        let inner = &input[inner_start..pos];
                        replacements.push(normalize(&open_tag, inner));
                        open_tag.clear();
                        active_idx = None;
                    }
                    pos = close_end + 1;
                    continue;
                }
            }
        }
        pos += 1;
    }

    replacements
}

fn render_fragment_markdown(input: &str) -> String {
    let options = build_mdast_html_options();
    let dedented = dedent_one_level(input);
    to_html_with_options(&dedented, &options).unwrap_or_else(|_| input.to_string())
}

fn build_mdast_html_options() -> Options {
    let mut parse = ParseOptions::gfm();
    parse.constructs.frontmatter = true;
    parse.constructs.html_flow = true;
    parse.constructs.html_text = true;
    parse.constructs.mdx_esm = false;
    parse.constructs.mdx_expression_flow = false;
    parse.constructs.mdx_expression_text = false;
    parse.constructs.mdx_jsx_flow = false;
    parse.constructs.mdx_jsx_text = false;

    let mut compile = CompileOptions::gfm();
    compile.allow_dangerous_html = true;
    compile.allow_dangerous_protocol = true;

    Options { parse, compile }
}

fn should_inject_starlight_css(input: &str) -> bool {
    input.contains("sl-steps")
        || input.contains("starlight-aside")
        || input.contains("starlight-tabs")
        || input.contains("starlight-file-tree")
        || input.contains("astro-code")
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

fn rewrite_tabs_only(input: &str) -> Result<String, MarkflowError> {
    let tabs = collect_tag_replacements_multi(input, &["Tabs", "tabs"], normalize_tabs_html);
    let astro_jsx_tabs =
        collect_tag_replacements_multi(input, &["AstroJSXTabs", "astrojsxtabs"], normalize_tabs_html);
    let package_tabs = collect_tag_replacements_multi(
        input,
        &["PackageManagerTabs", "packagemanagertabs"],
        normalize_tabs_html,
    );

    if tabs.is_empty() && astro_jsx_tabs.is_empty() && package_tabs.is_empty() {
        return Ok(input.to_string());
    }

    let tabs_queue = Rc::new(RefCell::new(VecDeque::from(tabs)));
    let astro_jsx_tabs_queue = Rc::new(RefCell::new(VecDeque::from(astro_jsx_tabs)));
    let package_tabs_queue = Rc::new(RefCell::new(VecDeque::from(package_tabs)));

    rewrite_str(
        input,
        RewriteStrSettings {
            element_content_handlers: vec![
                {
                    let tabs_queue = tabs_queue.clone();
                    element!("Tabs, tabs", move |el| {
                        if let Some(next) = tabs_queue.borrow_mut().pop_front() {
                            let _ = el.set_tag_name("div");
                            let _ = el.set_attribute("class", "starlight-tabs");
                            el.set_inner_content(&next, ContentType::Html);
                        }
                        Ok(())
                    })
                },
                {
                    let astro_jsx_tabs_queue = astro_jsx_tabs_queue.clone();
                    element!("AstroJSXTabs, astrojsxtabs", move |el| {
                        if let Some(next) = astro_jsx_tabs_queue.borrow_mut().pop_front() {
                            let _ = el.set_tag_name("div");
                            let _ = el.set_attribute("class", "starlight-tabs");
                            el.set_inner_content(&next, ContentType::Html);
                        }
                        Ok(())
                    })
                },
                {
                    let package_tabs_queue = package_tabs_queue.clone();
                    element!("PackageManagerTabs, packagemanagertabs", move |el| {
                        if let Some(next) = package_tabs_queue.borrow_mut().pop_front() {
                            let _ = el.set_tag_name("div");
                            let _ = el.set_attribute("class", "starlight-tabs");
                            el.set_inner_content(&next, ContentType::Html);
                        }
                        Ok(())
                    })
                },
            ],
            ..RewriteStrSettings::new()
        },
    )
    .map_err(|err| MarkflowError::MarkdownAdapter(err.to_string()))
}

fn unescape_target_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    let targets = [
        "Steps",
        "FileTree",
        "Tabs",
        "PackageManagerTabs",
        "AstroJSXTabs",
        "Aside",
        "Fragment",
    ];
    while let Some(idx) = rest.find("&lt;") {
        output.push_str(&rest[..idx]);
        rest = &rest[idx + 4..];
        if let Some(end) = rest.find("&gt;") {
            let tag = &rest[..end];
            let trimmed = tag.trim_start_matches('/').trim();
            let is_target = targets
                .iter()
                .any(|target| trimmed.eq_ignore_ascii_case(target));
            if is_target {
                output.push('<');
                output.push_str(tag);
                output.push('>');
            } else {
                output.push_str("&lt;");
                output.push_str(tag);
                output.push_str("&gt;");
            }
            rest = &rest[end + 4..];
        } else {
            output.push_str("&lt;");
            output.push_str(rest);
            rest = "";
        }
    }
    output.push_str(rest);
    output
}

fn parse_attribute_value(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let idx = tag.find(&needle)?;
    let mut rest = &tag[idx + needle.len()..];
    rest = rest.trim_start();
    if rest.is_empty() {
        return None;
    }
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        let quote = first;
        rest = &rest[quote.len_utf8()..];
        let end = rest.find(quote)?;
        return Some(rest[..end].to_string());
    }
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '>')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

fn capitalize_word(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_uppercase());
    out.push_str(chars.as_str());
    out
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn split_first_ordered_list(input: &str) -> Option<(&str, &str, &str, &str)> {
    let scanner = TagScanner::new(input);
    let open_start = input.find("<ol")?;
    let open_end = scanner.find_tag_end(open_start)?;
    let (close_start, close_end) = find_matching_ol_close(input, open_end + 1)?;
    let pre = &input[..open_start];
    let open_tag = &input[open_start..=open_end];
    let inner = &input[open_end + 1..close_start];
    let post = &input[close_end..];
    Some((pre, open_tag, inner, post))
}

fn find_matching_ol_close(input: &str, mut pos: usize) -> Option<(usize, usize)> {
    let scanner = TagScanner::new(input);
    let mut depth = 1usize;
    while pos < scanner.bytes.len() {
        if scanner.matches_tag_at(pos, b"<ol") {
            if let Some(end) = scanner.find_tag_end(pos) {
                depth += 1;
                pos = end + 1;
                continue;
            }
        }
        if scanner.matches_tag_at(pos, b"</ol") {
            if let Some(end) = scanner.find_tag_end(pos) {
                depth -= 1;
                let close_end = end + 1;
                if depth == 0 {
                    return Some((pos, close_end));
                }
                pos = close_end;
                continue;
            }
        }
        pos += 1;
    }
    None
}

fn insert_before_last_ol_close(output: &mut String, fragment: &str) {
    if let Some(idx) = output.rfind("</ol>") {
        output.insert_str(idx, fragment);
    } else {
        output.push_str(fragment);
    }
}

fn insert_into_last_list_item(output: &mut String, fragment: &str) {
    if let Some(idx) = output.rfind("</li>") {
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
        if let Some(open_pos) = find_ul_open(bytes, pos) {
            if open_pos == pos {
                depth += 1;
                pos += 3;
                continue;
            }
        }
        if let Some((close_pos, close_len)) = find_ul_close(bytes, pos) {
            if close_pos == pos {
                if depth > 0 {
                    depth -= 1;
                }
                pos += close_len;
                if depth == 0 {
                    let end = pos;
                    return input[start..end].to_string();
                }
                continue;
            }
        }
        pos += 1;
    }

    input.to_string()
}

fn extract_ordered_list_inner(input: &str) -> Option<String> {
    let (_pre, _open, inner, _post) = split_first_ordered_list(input)?;
    Some(inner.to_string())
}

fn append_class(existing: &str, class_name: &str) -> String {
    let mut classes: Vec<&str> = existing.split_whitespace().collect();
    if !classes.iter().any(|name| *name == class_name) {
        classes.push(class_name);
    }
    classes.join(" ")
}

fn ensure_pre_class(input: &str) -> String {
    let scanner = TagScanner::new(input);
    let mut output = String::with_capacity(input.len());
    let mut pos = 0usize;

    while let Some(rel_start) = input[pos..].find("<pre") {
        let start = pos + rel_start;
        output.push_str(&input[pos..start]);
        if let Some(end) = scanner.find_tag_end(start) {
            let tag = &input[start..=end];
            let updated = ensure_tag_class(tag, "astro-code");
            output.push_str(&updated);
            pos = end + 1;
        } else {
            output.push_str(&input[start..]);
            return output;
        }
    }

    output.push_str(&input[pos..]);
    output
}

fn ensure_tag_class(tag: &str, class_name: &str) -> String {
    if tag.contains("class=\"") {
        if tag.contains(class_name) {
            return tag.to_string();
        }
        if let Some(start) = tag.find("class=\"") {
            let value_start = start + "class=\"".len();
            if let Some(end) = tag[value_start..].find('"') {
                let end = value_start + end;
                let mut out = String::new();
                out.push_str(&tag[..end]);
                out.push(' ');
                out.push_str(class_name);
                out.push_str(&tag[end..]);
                return out;
            }
        }
    } else if tag.contains("class='") {
        if tag.contains(class_name) {
            return tag.to_string();
        }
        if let Some(start) = tag.find("class='") {
            let value_start = start + "class='".len();
            if let Some(end) = tag[value_start..].find('\'') {
                let end = value_start + end;
                let mut out = String::new();
                out.push_str(&tag[..end]);
                out.push(' ');
                out.push_str(class_name);
                out.push_str(&tag[end..]);
                return out;
            }
        }
    }

    if let Some(idx) = tag.rfind('>') {
        let mut out = String::new();
        out.push_str(&tag[..idx]);
        out.push_str(" class=\"");
        out.push_str(class_name);
        out.push_str("\"");
        out.push_str(&tag[idx..]);
        return out;
    }

    tag.to_string()
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
        if bytes[pos] == b'<' && bytes[pos + 1] == b'/' && bytes[pos + 2] == b'u' && bytes[pos + 3] == b'l' {
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
