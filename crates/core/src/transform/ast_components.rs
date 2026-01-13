//! AST-level component transformers for Starlight compatibility.
//!
//! These transformers operate on the mdast AST before rendering to HTML,
//! enabling structural changes that streaming HTML rewriters cannot achieve
//! (e.g., extracting labels from children and reorganizing into tablist/tabpanel structure).

use markdown::mdast::{AttributeContent, AttributeValue, MdxJsxFlowElement, Node};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter for generating unique tab panel IDs.
static TAB_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Applies component transformers to the AST recursively.
///
/// Call this after `markdown::to_mdast()` and before rendering to HTML.
/// Transformers are applied bottom-up (children first, then parent),
/// allowing nested components to be transformed before their containers.
pub fn apply_component_transformers(node: &mut Node) {
    // First transform children recursively (bottom-up traversal)
    match node {
        Node::Root(root) => {
            for child in &mut root.children {
                apply_component_transformers(child);
            }
        }
        Node::MdxJsxFlowElement(elem) => {
            for child in &mut elem.children {
                apply_component_transformers(child);
            }
        }
        Node::MdxJsxTextElement(elem) => {
            for child in &mut elem.children {
                apply_component_transformers(child);
            }
        }
        Node::Paragraph(p) => {
            for child in &mut p.children {
                apply_component_transformers(child);
            }
        }
        Node::Blockquote(b) => {
            for child in &mut b.children {
                apply_component_transformers(child);
            }
        }
        Node::List(l) => {
            for child in &mut l.children {
                apply_component_transformers(child);
            }
        }
        Node::ListItem(li) => {
            for child in &mut li.children {
                apply_component_transformers(child);
            }
        }
        _ => {}
    }

    // Then apply transformations to this node
    if let Node::MdxJsxFlowElement(elem) = node {
        if let Some(name) = &elem.name {
            match name.as_str() {
                "Tabs" => transform_tabs(node),
                // Future: "Steps", "FileTree", etc.
                _ => {}
            }
        }
    }
}

/// Extracts a string attribute value from JSX attributes.
fn get_attribute(attributes: &[AttributeContent], name: &str) -> Option<String> {
    for attr in attributes {
        if let AttributeContent::Property(prop) = attr {
            if prop.name == name {
                return match &prop.value {
                    Some(AttributeValue::Literal(s)) => Some(s.clone()),
                    _ => None,
                };
            }
        }
    }
    None
}

/// Information extracted from a TabItem element.
struct TabItemInfo {
    /// The label displayed on the tab button.
    label: String,
    /// Optional icon identifier (reserved for future use).
    #[allow(dead_code)]
    icon: Option<String>,
    /// The children nodes of the TabItem (content to display in panel).
    children: Vec<Node>,
}

/// Extracts TabItem information from a JSX element.
/// Supports both `<TabItem label="...">` (Starlight) and `<Tab title="...">` (legacy) formats.
fn extract_tab_item(elem: &MdxJsxFlowElement) -> Option<TabItemInfo> {
    // Try Starlight format first: <TabItem label="...">
    let label = get_attribute(&elem.attributes, "label")
        // Fall back to legacy format: <Tab title="...">
        .or_else(|| get_attribute(&elem.attributes, "title"))?;
    let icon = get_attribute(&elem.attributes, "icon");
    Some(TabItemInfo {
        label,
        icon,
        children: elem.children.clone(),
    })
}

/// Transforms a `<Tabs>` element into Starlight-compatible structure.
///
/// Input:
/// ```jsx
/// <Tabs>
///   <TabItem label="A">Content A</TabItem>
///   <TabItem label="B">Content B</TabItem>
/// </Tabs>
/// ```
///
/// Output (as HTML node):
/// ```html
/// <starlight-tabs>
///   <div class="tablist-wrapper not-content">
///     <ul role="tablist">
///       <li role="presentation" class="tab">
///         <a role="tab" href="#..." id="..." aria-selected="true" tabindex="0">A</a>
///       </li>
///       ...
///     </ul>
///   </div>
///   <section role="tabpanel" id="..." aria-labelledby="...">Content A</section>
///   ...
/// </starlight-tabs>
/// ```
fn transform_tabs(node: &mut Node) {
    let Node::MdxJsxFlowElement(tabs_elem) = node else {
        return;
    };

    // Extract TabItem/Tab children
    // Supports both <TabItem> (Starlight) and <Tab> (legacy) formats
    let mut tab_items: Vec<TabItemInfo> = Vec::new();
    for child in &tabs_elem.children {
        if let Node::MdxJsxFlowElement(child_elem) = child {
            let is_tab_item = matches!(
                child_elem.name.as_deref(),
                Some("TabItem") | Some("Tab")
            );
            if is_tab_item {
                if let Some(info) = extract_tab_item(child_elem) {
                    tab_items.push(info);
                }
            }
        }
    }

    if tab_items.is_empty() {
        return;
    }

    // Generate unique base ID for this tabs instance
    let base_id = TAB_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Build the transformed structure
    // We'll construct this as a new MdxJsxFlowElement with children,
    // which will then be rendered properly by the mdast renderer.

    // Generate tab buttons HTML
    let mut tablist_html = String::new();
    tablist_html.push_str("<div class=\"tablist-wrapper not-content\"><ul role=\"tablist\">");

    for (i, item) in tab_items.iter().enumerate() {
        let tab_id = format!("tab-{}-{}", base_id, i);
        let panel_id = format!("tab-panel-{}-{}", base_id, i);
        let selected = if i == 0 { "true" } else { "false" };
        let tabindex = if i == 0 { "0" } else { "-1" };

        tablist_html.push_str(&format!(
            "<li role=\"presentation\" class=\"tab\"><a role=\"tab\" href=\"#{}\" id=\"{}\" aria-selected=\"{}\" tabindex=\"{}\">{}</a></li>",
            panel_id,
            tab_id,
            selected,
            tabindex,
            html_escape(&item.label)
        ));
    }
    tablist_html.push_str("</ul></div>");

    // Build new children: tablist HTML + tab panels
    let mut new_children: Vec<Node> = Vec::new();

    // Add tablist as HTML node
    new_children.push(Node::Html(markdown::mdast::Html {
        value: tablist_html,
        position: None,
    }));

    // Add tab panels
    for (i, item) in tab_items.into_iter().enumerate() {
        let tab_id = format!("tab-{}-{}", base_id, i);
        let panel_id = format!("tab-panel-{}-{}", base_id, i);
        let hidden = if i == 0 { "" } else { " hidden" };

        // Create panel opening HTML
        let panel_open = format!(
            "<section role=\"tabpanel\" id=\"{}\" aria-labelledby=\"{}\"{}>",
            panel_id, tab_id, hidden
        );

        // Create panel closing HTML
        let panel_close = "</section>".to_string();

        // Add opening tag as HTML
        new_children.push(Node::Html(markdown::mdast::Html {
            value: panel_open,
            position: None,
        }));

        // Add panel content (the TabItem's children)
        new_children.extend(item.children);

        // Add closing tag as HTML
        new_children.push(Node::Html(markdown::mdast::Html {
            value: panel_close,
            position: None,
        }));
    }

    // Replace the Tabs element with starlight-tabs custom element
    tabs_elem.name = Some("starlight-tabs".to_string());
    tabs_elem.attributes.clear();
    tabs_elem.children = new_children;
}

/// Simple HTML escaping for text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_attribute() {
        use markdown::mdast::{AttributeContent, AttributeValue, MdxJsxAttribute};

        let attrs = vec![AttributeContent::Property(MdxJsxAttribute {
            name: "label".to_string(),
            value: Some(AttributeValue::Literal("Test Label".to_string())),
        })];

        assert_eq!(get_attribute(&attrs, "label"), Some("Test Label".to_string()));
        assert_eq!(get_attribute(&attrs, "missing"), None);
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("Hello"), "Hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }
}
