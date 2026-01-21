//! Code generation utilities for WASM and NAPI bindings.
//!
//! This module provides shared functionality for generating JavaScript/JSX code
//! from parsed markdown content, eliminating duplication between binding layers.

use crate::registry::defaults::default_starlight_registry;
use crate::renderer::mdast::RenderBlock;
use crate::{PropValue, RegistryConfig};
use std::fmt::Write as FmtWrite;

/// Converts a Rust string to a JavaScript string literal.
///
/// Uses JSON serialization to properly escape special characters.
///
/// # Examples
///
/// ```
/// use markflow_core::codegen::js_string_literal;
///
/// assert_eq!(js_string_literal("hello"), "\"hello\"");
/// assert_eq!(js_string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
/// ```
pub fn js_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

/// Escapes a string for use inside a JavaScript string literal (without surrounding quotes).
///
/// Uses JSON serialization to properly escape special characters, then strips the quotes.
///
/// # Examples
///
/// ```
/// use markflow_core::codegen::escape_js_string_value;
///
/// assert_eq!(escape_js_string_value("hello"), "hello");
/// assert_eq!(escape_js_string_value("say \"hi\""), "say \\\"hi\\\"");
/// assert_eq!(escape_js_string_value("line1\nline2"), "line1\\nline2");
/// assert_eq!(escape_js_string_value("back\\slash"), "back\\\\slash");
/// ```
pub fn escape_js_string_value(value: &str) -> String {
    // Use serde_json for proper escaping, then strip the surrounding quotes
    let json = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string());
    // Remove leading and trailing quotes added by serde_json
    json[1..json.len() - 1].to_string()
}

/// Result of directive mapping.
pub struct DirectiveMappingResult {
    /// The component tag name to use (e.g., "Aside" instead of "note").
    pub tag_name: String,
    /// Optional additional prop to add (e.g., `type="note"`).
    pub type_prop: Option<String>,
}

/// Converts RenderBlocks to a JSX string.
///
/// # Arguments
///
/// * `blocks` - The render blocks to convert
/// * `directive_mapper` - Optional closure that maps directive names to component names
///   and optionally adds a type prop. If None, directive names are used as-is.
///
/// # Example
///
/// ```
/// use markflow_core::codegen::{blocks_to_jsx_string, DirectiveMappingResult};
/// use markflow_core::RenderBlock;
///
/// let blocks = vec![RenderBlock::Html {
///     content: "<p>Hello</p>".to_string(),
/// }];
///
/// // Without directive mapping
/// let jsx = blocks_to_jsx_string(&blocks, None::<fn(&str) -> Option<DirectiveMappingResult>>);
/// assert_eq!(jsx, "<p>Hello</p>");
/// ```
pub fn blocks_to_jsx_string<F>(blocks: &[RenderBlock], directive_mapper: Option<F>) -> String
where
    F: Fn(&str) -> Option<DirectiveMappingResult>,
{
    blocks_to_jsx_string_with_registry(blocks, directive_mapper, None)
}

/// Converts RenderBlocks to a JSX string with registry-based slot normalization.
///
/// # Arguments
///
/// * `blocks` - The render blocks to convert
/// * `directive_mapper` - Optional closure that maps directive names to component names
/// * `registry` - Optional registry for slot normalization rules. If None, uses default Starlight registry.
pub fn blocks_to_jsx_string_with_registry<F>(
    blocks: &[RenderBlock],
    directive_mapper: Option<F>,
    registry: Option<&RegistryConfig>,
) -> String
where
    F: Fn(&str) -> Option<DirectiveMappingResult>,
{
    // Use provided registry or default to Starlight
    let default_registry = default_starlight_registry();
    let registry = registry.unwrap_or(&default_registry);

    let mut result = String::new();
    for block in blocks {
        match block {
            RenderBlock::Html { content } => {
                let escaped = sanitize_html_block_for_jsx(content);
                result.push_str(&escaped);
            }
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                // Apply slot normalization based on registry configuration
                let slot_html = normalize_slot_by_registry(name, slot_html, registry);
                // Note: Braces in text content are already escaped by push_text() and
                // render_children_to_string(). We must NOT escape here because it would
                // break JSX expressions in nested component props like <Since v={"1.0"}>

                // Apply directive mapping if provided
                let (tag_name, type_prop) = if let Some(ref mapper) = directive_mapper {
                    if let Some(mapping) = mapper(name) {
                        (mapping.tag_name, mapping.type_prop)
                    } else {
                        (name.clone(), None)
                    }
                } else {
                    (name.clone(), None)
                };

                result.push('<');
                result.push_str(&tag_name);

                // Add type prop if mapping provided one
                if let Some(type_value) = type_prop {
                    result.push_str(" type=\"");
                    result.push_str(&type_value);
                    result.push('"');
                }

                // Render props as {...{key: "value" | expression, ...}}
                if !props.is_empty() {
                    result.push_str(" {...{");
                    let mut first = true;
                    for (key, prop_value) in props {
                        if !first {
                            result.push_str(", ");
                        }
                        first = false;
                        // Escape the key for JavaScript
                        result.push('"');
                        result.push_str(&key.replace('"', "\\\""));
                        result.push_str("\": ");

                        // Render value based on type
                        match prop_value {
                            PropValue::Literal { value } => {
                                // String literal: wrap in quotes
                                result.push('"');
                                result.push_str(&escape_js_string_value(value));
                                result.push('"');
                            }
                            PropValue::Expression { value } => {
                                // JS expression: output raw (no quotes)
                                result.push_str(value);
                            }
                        }
                    }
                    result.push_str("}}");
                }

                result.push('>');
                result.push_str(&slot_html);
                result.push_str("</");
                result.push_str(&tag_name);
                result.push('>');
            }
        }
    }
    result
}

/// Escapes JSX-sensitive characters inside raw HTML blocks.
///
/// - Escapes `{` and `}` globally so they are not parsed as JSX expressions.
/// - For backtick-delimited code spans (`, ```, etc.), wraps the contents in `<code>`
///   and escapes `<`, `>`, `&`, and braces so code examples remain literal.
/// - For `<script>` / `<style>` blocks we only escape braces to avoid breaking
///   the embedded code while still keeping JSX safe.
fn sanitize_html_block_for_jsx(content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    let is_script_or_style =
        lower.contains("<script") || lower.contains("<style") || lower.contains("</script>");

    // For script/style we must keep the exact JS/CSS text (including braces) because
    // escaping them to entities breaks the embedded code. These tags are wrapped in JSX
    // as children text, which is allowed even when containing braces.
    if is_script_or_style {
        return content.to_string();
    }

    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            // Count how many backticks start the span (supports ``` as well).
            let mut tick_count = 1;
            while let Some('`') = chars.peek() {
                tick_count += 1;
                chars.next();
            }

            let mut code = String::new();
            while let Some(next) = chars.next() {
                if next == '`' {
                    let mut end_ticks = 1;
                    while let Some('`') = chars.peek() {
                        end_ticks += 1;
                        chars.next();
                    }
                    if end_ticks == tick_count {
                        break;
                    } else {
                        code.push_str(&"`".repeat(end_ticks));
                        continue;
                    }
                } else {
                    code.push(next);
                }
            }

            // Escape code span contents
            out.push_str("<code>");
            for c in code.chars() {
                match c {
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    '&' => out.push_str("&amp;"),
                    '{' => out.push_str("&#123;"),
                    '}' => out.push_str("&#125;"),
                    _ => out.push(c),
                }
            }
            out.push_str("</code>");
        } else {
            match ch {
                '{' => out.push_str("&#123;"),
                '}' => out.push_str("&#125;"),
                _ => out.push(ch),
            }
        }
    }

    out
}

/// Applies slot normalization based on registry configuration.
///
/// This function looks up the component in the registry's slot_normalizations
/// and applies the appropriate transformation strategy.
fn normalize_slot_by_registry(
    component: &str,
    slot_html: &str,
    registry: &RegistryConfig,
) -> String {
    if let Some(normalization) = registry.get_slot_normalization(component) {
        match normalization.strategy.as_str() {
            "wrap_in_ol" => normalize_wrap_in_ol(slot_html),
            "wrap_in_ul" => normalize_wrap_in_ul(slot_html, normalization.wrapper_class.as_deref()),
            _ => slot_html.to_string(),
        }
    } else {
        slot_html.to_string()
    }
}

/// Normalizes slot content by wrapping in a single `<ol>` element.
///
/// This is used for components like Steps that require ordered list structure.
fn normalize_wrap_in_ol(slot_html: &str) -> String {
    let trimmed = slot_html.trim();

    // If it is already a single <ol> ... </ol> with no siblings, keep it.
    if trimmed.starts_with("<ol") && trimmed.ends_with("</ol>") {
        let first_ol = trimmed.find("<ol").unwrap_or(0);
        let last_close = trimmed.rfind("</ol>").unwrap_or(trimmed.len());
        let has_extra_ol = trimmed[first_ol + 3..last_close].contains("<ol");
        let trailing = trimmed[last_close + 5..].trim(); // 5 = len("</ol>")
        let leading = trimmed[..first_ol].trim();
        if !has_extra_ol && leading.is_empty() && trailing.is_empty() {
            return slot_html.to_string();
        }
    }

    // Otherwise, merge everything into a single ordered list.
    fn push_other_as_li(buf: &mut String, fragment: &str) {
        let frag = fragment.trim();
        if frag.is_empty() {
            return;
        }
        buf.push_str("<li>");
        buf.push_str(frag);
        buf.push_str("</li>");
    }

    let mut items = String::new();
    let mut rest = trimmed;

    while let Some(start) = rest.find("<ol") {
        let before = &rest[..start];
        push_other_as_li(&mut items, before);

        let after_ol = &rest[start..];
        if let Some(end_idx) = after_ol.find("</ol>") {
            let body_start = after_ol
                .find('>')
                .map(|i| i + 1)
                .unwrap_or_else(|| "<ol".len());
            let body = &after_ol[body_start..end_idx];
            items.push_str(body); // keep inner <li> list items as-is
            rest = &after_ol[end_idx + "</ol>".len()..];
        } else {
            // Malformed; wrap remainder
            push_other_as_li(&mut items, after_ol);
            rest = "";
            break;
        }
    }

    push_other_as_li(&mut items, rest);

    format!("<ol>{}</ol>", items)
}

/// Normalizes slot content by wrapping in a single `<ul>` element.
///
/// This is used for components like FileTree that require unordered list structure.
fn normalize_wrap_in_ul(slot_html: &str, wrapper_class: Option<&str>) -> String {
    let trimmed = slot_html.trim();
    let has_li = trimmed.contains("<li");

    let class_attr = wrapper_class
        .map(|c| format!(" class=\"{}\"", c))
        .unwrap_or_default();

    if trimmed.is_empty() {
        return format!("<ul{}><li></li></ul>", class_attr);
    }

    // If already wrapped in <ul>, check if we need to add li
    if trimmed.starts_with("<ul") && trimmed.ends_with("</ul>") {
        if has_li {
            // Already properly wrapped with li items, return unchanged
            // (Starlight FileTree component handles styling internally)
            return slot_html.to_string();
        }
        // Add empty li
        return format!(
            "<ul{}>{}<li></li></ul>",
            class_attr,
            &trimmed[trimmed.find('>').map(|i| i + 1).unwrap_or(3)..trimmed.len() - 5]
        );
    }

    if has_li {
        format!("<ul{}>{}</ul>", class_attr, slot_html)
    } else {
        format!("<ul{}><li>{}</li></ul>", class_attr, slot_html)
    }
}

/// Options for Astro module generation.
#[derive(Debug, Clone, Default)]
pub struct AstroModuleOptions<'a> {
    /// The JSX content to embed in the component.
    pub jsx: &'a str,
    /// Hoisted import statements.
    pub hoisted_imports: &'a [String],
    /// Serialized frontmatter as JSON.
    pub frontmatter_json: &'a str,
    /// Serialized headings as JSON.
    pub headings_json: &'a str,
    /// File path for the module.
    pub filepath: &'a str,
    /// URL for the module (None means `undefined`).
    pub url: Option<&'a str>,
    /// Layout import path (e.g., "../layouts/Base.astro").
    pub layout_import: Option<&'a str>,
    /// Whether the user provided their own `export default`.
    pub has_user_default_export: bool,
}

/// Generates an Astro-compatible JavaScript module from the given options.
///
/// This produces a complete module with:
/// - Runtime imports (Fragment, jsx, createComponent, renderJSX)
/// - Hoisted user imports
/// - Frontmatter, file, url, headings exports
/// - MarkflowContent component
/// - MDX component markers for Astro Content Collections
/// - Default export (unless user provided one)
pub fn generate_astro_module(options: &AstroModuleOptions<'_>) -> String {
    let mut code = String::new();

    // Runtime imports
    let _ = writeln!(
        code,
        "import {{ Fragment, jsx as __jsx }} from 'astro/jsx-runtime';"
    );
    let _ = writeln!(code, "const _Fragment = Fragment;");
    let _ = writeln!(
        code,
        "const _jsx = (type, props, ...children) => {{\n  const resolved = props ?? {{}};\n  if (children.length > 0) {{\n    resolved.children = children.length === 1 ? children[0] : children;\n  }}\n  return __jsx(type, resolved, resolved.key);\n}};"
    );
    let _ = writeln!(
        code,
        "import {{ createComponent, renderJSX }} from 'astro/runtime/server/index.js';"
    );

    // Layout import if specified
    if let Some(layout) = options.layout_import {
        let _ = writeln!(code, "import Layout from {};", js_string_literal(layout));
    }

    // Hoisted imports
    for import in options.hoisted_imports {
        let _ = writeln!(code, "{}", import);
    }

    // Exports
    let _ = writeln!(
        code,
        "export const frontmatter = {};",
        options.frontmatter_json
    );
    let _ = writeln!(
        code,
        "export const file = {};",
        js_string_literal(options.filepath)
    );
    let url_literal = options
        .url
        .map(js_string_literal)
        .unwrap_or_else(|| "undefined".to_string());
    let _ = writeln!(code, "export const url = {};", url_literal);
    let _ = writeln!(code, "export const headings = {};", options.headings_json);
    let _ = writeln!(code, "export function getHeadings() {{");
    let _ = writeln!(code, "  return {};", options.headings_json);
    let _ = writeln!(code, "}}");

    // MarkflowContent component
    let _ = writeln!(code, "// function MarkflowContent");
    let _ = writeln!(
        code,
        "const MarkflowContent = createComponent((result, props) => {{"
    );
    let _ = writeln!(code, "  return renderJSX(result, (");
    let _ = writeln!(code, "    <>");
    code.push_str(options.jsx);
    if !options.jsx.ends_with('\n') {
        code.push('\n');
    }
    let _ = writeln!(code, "    </>");
    let _ = writeln!(code, "  ));");
    let _ = writeln!(code, "}}, file);");

    let _ = writeln!(code, "export const Content = MarkflowContent;");

    // Add MDX component markers for Astro Content Collections
    let _ = writeln!(code, "Content[Symbol.for('mdx-component')] = true;");
    let _ = writeln!(
        code,
        "Content[Symbol.for('astro.needsHeadRendering')] = !Boolean(frontmatter.layout);"
    );
    let _ = writeln!(
        code,
        "Content.moduleId = {};",
        js_string_literal(options.filepath)
    );

    // Export default (conditional)
    if !options.has_user_default_export {
        if options.layout_import.is_some() {
            let _ = writeln!(
                code,
                "export default createComponent((result, props) => renderJSX(result, _jsx(Layout, {{...props, frontmatter: frontmatter, children: _jsx(MarkflowContent, {{...props}})}})), file);"
            );
        } else {
            let _ = writeln!(code, "export default MarkflowContent;");
        }
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_js_string_literal() {
        assert_eq!(js_string_literal("hello"), "\"hello\"");
        assert_eq!(js_string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(js_string_literal("line1\nline2"), "\"line1\\nline2\"");
        assert_eq!(js_string_literal("back\\slash"), "\"back\\\\slash\"");
    }

    #[test]
    fn test_escape_js_string_value() {
        // Basic escaping
        assert_eq!(escape_js_string_value("hello"), "hello");
        assert_eq!(escape_js_string_value("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_js_string_value("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_js_string_value("back\\slash"), "back\\\\slash");

        // CJK characters
        assert_eq!(escape_js_string_value("中文"), "中文");
        assert_eq!(escape_js_string_value("中文\"引用\""), "中文\\\"引用\\\"");
        assert_eq!(escape_js_string_value("한글"), "한글");

        // Special characters
        assert_eq!(escape_js_string_value("tab\there"), "tab\\there");
        assert_eq!(escape_js_string_value("return\rhere"), "return\\rhere");
    }

    #[test]
    fn test_blocks_to_jsx_string_cjk_props() {
        let mut props = HashMap::new();
        props.insert(
            "title".to_string(),
            PropValue::Literal {
                value: "中文\"引用\"标题".to_string(),
            },
        );
        let blocks = vec![RenderBlock::Component {
            name: "Card".to_string(),
            props,
            slot_html: "<p>Content</p>".to_string(),
        }];
        let jsx = blocks_to_jsx_string(&blocks, None::<fn(&str) -> Option<DirectiveMappingResult>>);
        assert_eq!(
            jsx,
            "<Card {...{\"title\": \"中文\\\"引用\\\"标题\"}}><p>Content</p></Card>"
        );
    }

    #[test]
    fn test_blocks_to_jsx_string_html_only() {
        let blocks = vec![RenderBlock::Html {
            content: "<p>Hello</p>".to_string(),
        }];
        let jsx = blocks_to_jsx_string(&blocks, None::<fn(&str) -> Option<DirectiveMappingResult>>);
        assert_eq!(jsx, "<p>Hello</p>");
    }

    #[test]
    fn test_blocks_to_jsx_string_component() {
        let mut props = HashMap::new();
        props.insert(
            "title".to_string(),
            PropValue::Literal {
                value: "Hello".to_string(),
            },
        );
        let blocks = vec![RenderBlock::Component {
            name: "Card".to_string(),
            props,
            slot_html: "<p>Content</p>".to_string(),
        }];
        let jsx = blocks_to_jsx_string(&blocks, None::<fn(&str) -> Option<DirectiveMappingResult>>);
        assert_eq!(
            jsx,
            "<Card {...{\"title\": \"Hello\"}}><p>Content</p></Card>"
        );
    }

    #[test]
    fn test_blocks_to_jsx_string_with_directive_mapper() {
        let mut props = HashMap::new();
        props.insert(
            "title".to_string(),
            PropValue::Literal {
                value: "Important".to_string(),
            },
        );
        let blocks = vec![RenderBlock::Component {
            name: "note".to_string(),
            props,
            slot_html: "<p>Content</p>".to_string(),
        }];

        let mapper = |name: &str| -> Option<DirectiveMappingResult> {
            match name {
                "note" | "tip" | "caution" | "danger" => Some(DirectiveMappingResult {
                    tag_name: "Aside".to_string(),
                    type_prop: Some(name.to_string()),
                }),
                _ => None,
            }
        };

        let jsx = blocks_to_jsx_string(&blocks, Some(mapper));
        assert_eq!(
            jsx,
            "<Aside type=\"note\" {...{\"title\": \"Important\"}}><p>Content</p></Aside>"
        );
    }

    #[test]
    fn test_generate_astro_module_basic() {
        let options = AstroModuleOptions {
            jsx: "<p>Hello</p>",
            hoisted_imports: &[],
            frontmatter_json: "{}",
            headings_json: "[]",
            filepath: "/test.md",
            url: None,
            layout_import: None,
            has_user_default_export: false,
        };

        let code = generate_astro_module(&options);

        assert!(code.contains("import { Fragment, jsx as __jsx } from 'astro/jsx-runtime';"));
        assert!(code.contains("export const frontmatter = {};"));
        assert!(code.contains("export const file = \"/test.md\";"));
        assert!(code.contains("export const url = undefined;"));
        assert!(code.contains("<p>Hello</p>"));
        assert!(code.contains("export default MarkflowContent;"));
    }

    #[test]
    fn test_generate_astro_module_with_layout() {
        let options = AstroModuleOptions {
            jsx: "<p>Hello</p>",
            hoisted_imports: &[],
            frontmatter_json: "{}",
            headings_json: "[]",
            filepath: "/test.md",
            url: None,
            layout_import: Some("../layouts/Base.astro"),
            has_user_default_export: false,
        };

        let code = generate_astro_module(&options);

        assert!(code.contains("import Layout from \"../layouts/Base.astro\";"));
        assert!(code.contains("_jsx(Layout,"));
    }

    #[test]
    fn test_generate_astro_module_no_default_export() {
        let options = AstroModuleOptions {
            jsx: "<p>Hello</p>",
            hoisted_imports: &[],
            frontmatter_json: "{}",
            headings_json: "[]",
            filepath: "/test.md",
            url: None,
            layout_import: None,
            has_user_default_export: true,
        };

        let code = generate_astro_module(&options);

        assert!(!code.contains("export default MarkflowContent;"));
    }
}
