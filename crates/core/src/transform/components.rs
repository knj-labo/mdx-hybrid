//! Rewrites Astro docs components (Aside, Steps, Tabs, FileTree) into plain HTML structures.

use lol_html::{element, ElementContentHandlers};
use std::borrow::Cow;
use lol_html::Selector;

/// Returns lol_html handlers for rewriting Astro docs components into plain HTML.
pub fn component_handlers(
) -> Vec<(Cow<'static, Selector>, ElementContentHandlers<'static>)> {
    vec![
        steps_handler(),
        step_handler(),
        tabs_handler(),
        tab_handler(),
        filetree_handler(),
        file_handler(),
    ]
}

fn steps_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("Steps", |el| {
        el.set_tag_name("ol")?;
        el.set_attribute("class", "steps")?;
        Ok(())
    }).into()
}

fn step_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("Step", |el| {
        el.set_tag_name("li")?;
        el.set_attribute("class", "steps__item")?;
        Ok(())
    }).into()
}

fn tabs_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("Tabs", |el| {
        el.set_tag_name("div")?;
        el.set_attribute("class", "tabs")?;
        el.set_attribute("role", "tablist")?;
        Ok(())
    }).into()
}

fn tab_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("Tab", |el| {
        let title = el.get_attribute("title").unwrap_or_default();
        el.set_tag_name("div")?;
        el.set_attribute("class", "tab")?;
        el.set_attribute("role", "tabpanel")?;
        if !title.is_empty() {
            let heading = format!("<div class=\"tab__title\">{}</div>", title);
            let _ = el.prepend(&heading, lol_html::html_content::ContentType::Html);
            let _ = el.remove_attribute("title");
        }
        Ok(())
    }).into()
}

fn filetree_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("FileTree", |el| {
        el.set_tag_name("ul")?;
        el.set_attribute("class", "filetree")?;
        Ok(())
    }).into()
}

fn file_handler() -> (Cow<'static, Selector>, ElementContentHandlers<'static>) {
    element!("File", |el| {
        el.set_tag_name("li")?;
        el.set_attribute("class", "filetree__item")?;
        Ok(())
    }).into()
}
