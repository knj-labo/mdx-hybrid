//! Code generation utilities for WASM and NAPI bindings.
//!
//! This module provides shared functionality for generating JavaScript/JSX code
//! from parsed markdown content, eliminating duplication between binding layers.

use crate::PropValue;
use crate::renderer::mdast::RenderBlock;
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
    let mut result = String::new();
    for block in blocks {
        match block {
            RenderBlock::Html { content } => {
                result.push_str(content);
            }
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                // Normalize Steps slot to single <ol> child (Starlight requirement)
                let slot_html = if name == "Steps" {
                    normalize_steps_slot(slot_html)
                } else {
                    slot_html.clone()
                };

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

fn normalize_steps_slot(slot_html: &str) -> String {
    let trimmed = slot_html.trim();
    if trimmed.starts_with("<ol") && trimmed.ends_with("</ol>") {
        slot_html.to_string()
    } else {
        format!("<ol><li>{}</li></ol>", slot_html)
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
