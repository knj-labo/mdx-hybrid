//! MDAST-based Markdown to HTML renderer.
//!
//! This module provides a markdown renderer using the markdown-rs AST (MDAST).
//! It converts markdown input to a list of rendering blocks (HTML or Component)
//! suitable for Astro/Starlight integration.
//!
//! # Module Structure
//!
//! - `types` - Type definitions (PropValue, RenderBlock, HeadingEntry, etc.)
//! - `context` - Rendering context for tracking state during traversal
//! - `render` - AST node rendering functions
//! - `directives` - Directive syntax preprocessing

mod context;
mod directives;
pub mod render;
mod types;

pub use context::Context;
pub use types::{AsideMeta, BlocksResult, CardMeta, HeadingEntry, PropValue, RenderBlock, Scope};

use crate::transform::jsx_normalize::{
    collapse_multiline_wrapper_tags, normalize_mdx_jsx_indentation,
};
use crate::transform::smartypants::apply_smartypants;
use render::render_node;

/// Rendering options for the mdast renderer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Options {
    /// Whether directive processing is enabled.
    #[serde(default)]
    pub enable_directives: bool,
    /// Whether to apply smart punctuation transformations.
    #[serde(default)]
    pub enable_smartypants: bool,
    /// Whether to add loading="lazy" to images.
    #[serde(default)]
    pub enable_lazy_images: bool,
    /// Whether to allow raw HTML (<script>, <style>, etc.) to pass through.
    /// When enabled, markdown-rs parses these as HTML nodes instead of MDX JSX,
    /// avoiding parse errors on trusted docs content that mixes raw tags.
    #[serde(default = "default_allow_raw_html")]
    pub allow_raw_html: bool,
}

impl Options {
    /// Returns whether lazy image loading is enabled.
    pub fn lazy_images(&self) -> bool {
        self.enable_lazy_images
    }

    /// Returns whether raw HTML passthrough is enabled.
    pub fn allow_raw_html(&self) -> bool {
        self.allow_raw_html
    }
}

fn default_allow_raw_html() -> bool {
    true
}

impl Default for Options {
    fn default() -> Self {
        Self {
            enable_directives: false,
            enable_smartypants: false,
            enable_lazy_images: false,
            allow_raw_html: default_allow_raw_html(),
        }
    }
}

/// Converts Markdown input to rendering blocks (entry point).
///
/// # Arguments
///
/// * `input` - The markdown text to convert
/// * `options` - Rendering options (CSS injection, directives, etc.)
///
/// # Returns
///
/// * `Ok(BlocksResult)` - Rendering blocks with heading metadata
/// * `Err(String)` - Error message if parsing fails
///
/// # Examples
///
/// ```
/// use markflow_core::renderer::mdast::{to_blocks, Options};
///
/// let input = "Hello, [world](https://example.com)!";
/// let options = Options {
///     enable_directives: false,
///     ..Default::default()
/// };
/// let blocks = to_blocks(input, &options).unwrap();
/// ```
pub fn to_blocks(input: &str, options: &Options) -> Result<BlocksResult, String> {
    // 1. Preprocess directives if enabled
    let preprocessed = if options.enable_directives {
        directives::preprocess_directives(input)
    } else {
        input.to_string()
    };

    // 2. Collapse multiline wrapper tags to prevent tag mismatch errors
    let collapsed = collapse_multiline_wrapper_tags(&preprocessed);

    // 3. Normalize JSX indentation to prevent content from being treated as code blocks
    let normalized = normalize_mdx_jsx_indentation(&collapsed);

    // 4. Parse markdown to MDAST with enhanced options
    let parse_options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            // MDX: JSX support for <Component>...</Component>
            mdx_jsx_flow: true,
            mdx_jsx_text: true,
            // HTML: allow raw tags when configured (trusted docs content)
            html_flow: options.allow_raw_html(),
            html_text: options.allow_raw_html(),
            // Enable frontmatter (--- ... ---)
            frontmatter: true,
            // GitHub Flavored Markdown features
            gfm_autolink_literal: true,
            gfm_strikethrough: true,
            gfm_table: true,
            gfm_task_list_item: true,
            ..markdown::Constructs::default()
        },
        ..markdown::ParseOptions::default()
    };

    let tree = markdown::to_mdast(&normalized, &parse_options)
        .map_err(|e| format!("Markdown parse error: {}", e))?;

    // 4. Traverse the AST and render to blocks
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 5. Finish and get blocks
    let mut result = ctx.finish();

    // 6. Apply smartypants if enabled
    if options.enable_smartypants {
        for block in &mut result.blocks {
            if let RenderBlock::Html { content } = block {
                *content = apply_smartypants(content);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let input = "Hello, world!";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);
        match &blocks.blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains("Hello, world!"));
            }
            _ => panic!("Expected HTML block"),
        }
    }

    #[test]
    fn test_paragraph() {
        let input = "This is a paragraph.";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);
        match &blocks.blocks[0] {
            RenderBlock::Html { content } => {
                assert_eq!(content, "<p>This is a paragraph.</p>");
            }
            _ => panic!("Expected HTML block"),
        }
    }

    #[test]
    fn test_link() {
        let input = "[Rust](https://www.rust-lang.org/)";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);
        match &blocks.blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains(r#"<a href="https://www.rust-lang.org/""#));
                assert!(content.contains("Rust</a>"));
            }
            _ => panic!("Expected HTML block"),
        }
    }

    #[test]
    fn test_directive_to_component() {
        let input = ":::note[My Title]\nThis is **important** content.\n:::";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        match &blocks.blocks[0] {
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                assert_eq!(name, "Aside");
                assert_eq!(props.get("type"), Some(&PropValue::literal("note")));
                assert_eq!(props.get("title"), Some(&PropValue::literal("My Title")));
                assert!(slot_html.contains("<p>This is <strong>important</strong> content.</p>"));
            }
            _ => panic!("Expected Component block"),
        }
    }

    #[test]
    fn test_directive_without_title() {
        let input = ":::tip\nHelpful advice here.\n:::";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        match &blocks.blocks[0] {
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                assert_eq!(name, "Aside");
                assert_eq!(props.get("type"), Some(&PropValue::literal("tip")));
                assert!(props.get("title").is_none());
                assert!(slot_html.contains("Helpful advice"));
            }
            _ => panic!("Expected Component block"),
        }
    }

    #[test]
    fn test_directive_disabled() {
        let input = ":::note\nContent\n:::";
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        match &blocks.blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains(":::note"));
            }
            _ => panic!("Expected HTML block when directives disabled"),
        }
    }

    #[test]
    fn test_standard_markdown_elements() {
        let input = r#"# Heading 1
## Heading 2

- List Item 1
- List Item 2

> Blockquote

```rust
fn main() {}
```

![Alt text](image.png "Title")

---
"#;
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };
        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<h1 id=") && content.contains(">Heading 1</h1>"));
            assert!(content.contains("<h2 id=") && content.contains(">Heading 2</h2>"));
            assert!(content.contains("<ul>"));
            assert!(content.contains("<li>"));
            assert!(content.contains("<blockquote>"));
            assert!(content.contains(r#"<code class="language-rust">"#));
            assert!(content.contains("fn main() &#123;&#125;"));
            assert!(content.contains(r#"<img src="image.png""#));
            assert!(content.contains("<hr />"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_task_list() {
        let input = "- [ ] Unchecked task\n- [x] Checked task\n";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };
        let blocks = to_blocks(input, &options).unwrap();

        assert_eq!(blocks.blocks.len(), 1);
        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains(r#"class="task-list-item""#));
            assert!(content.contains(r#"type="checkbox""#));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_ordered_list() {
        let input = "1. First\n2. Second\n3. Third\n";
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };
        let blocks = to_blocks(input, &options).unwrap();

        assert_eq!(blocks.blocks.len(), 1);
        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<ol>"));
            assert!(content.contains("<li>"));
            assert!(content.contains("</ol>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_xss_text_escaping() {
        let input = "Text with <script>alert('xss')</script> and & symbols.";
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 3);

        match &result.blocks[0] {
            RenderBlock::Html { content } => {
                assert_eq!(content, "<p>Text with ");
            }
            other => panic!("Expected HTML block, got {:?}", other),
        }

        match &result.blocks[1] {
            RenderBlock::Component {
                name, slot_html, ..
            } => {
                assert_eq!(name, "script");
                assert_eq!(slot_html, "alert('xss')");
            }
            other => panic!("Expected Component block, got {:?}", other),
        }

        match &result.blocks[2] {
            RenderBlock::Html { content } => {
                assert_eq!(content, " and &amp; symbols.</p>");
            }
            other => panic!("Expected HTML block, got {:?}", other),
        }
    }

    #[test]
    fn test_xss_attribute_escaping() {
        let input = r#"[Link](http://example.com "Title with <script> and & and ' quotes")"#;
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("&lt;script&gt;"));
            assert!(content.contains("&amp;"));
            assert!(content.contains("&#39;"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_xss_image_attributes() {
        let input = r#"![Alt with ' and "](image.png "Title with &")"#;
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("&#39;"));
            assert!(content.contains("&quot;"));
            assert!(content.contains("&amp;"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn debug_directive_ast() {
        let input = ":::note[My Title]\nContent\n:::";

        let parse_options = markdown::ParseOptions {
            constructs: markdown::Constructs {
                frontmatter: true,
                gfm_autolink_literal: true,
                gfm_strikethrough: true,
                gfm_table: true,
                gfm_task_list_item: true,
                ..markdown::Constructs::default()
            },
            ..markdown::ParseOptions::default()
        };

        let tree = markdown::to_mdast(input, &parse_options).unwrap();
        println!("\n=== AST DEBUG START ===");
        println!("{:#?}", tree);
        println!("=== AST DEBUG END ===\n");
    }

    #[test]
    fn test_strikethrough() {
        let input = "This is ~~deleted~~ text.";
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<del>deleted</del>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_table() {
        let input = r#"| Name | Age | City |
| :--- | :---: | ---: |
| Alice | 30 | Tokyo |
| Bob | 25 | NYC |"#;
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<table>"));
            assert!(content.contains("<thead>"));
            assert!(content.contains("<tbody>"));
            assert!(content.contains("align=\"left\""));
            assert!(content.contains("align=\"center\""));
            assert!(content.contains("align=\"right\""));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_table_with_formatting() {
        let input = r#"| Feature | Status |
| --- | --- |
| **Bold** | ✓ |
| [Link](https://example.com) | ✓ |
| `code` | ✓ |"#;
        let options = Options {
            enable_directives: false,
            ..Default::default()
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<strong>Bold</strong>"));
            assert!(content.contains(r#"<a href="https://example.com">"#));
            assert!(content.contains("<code>code</code>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_code_block_preserves_newlines() {
        let input = r#"```ts
line1
line2
line3
```"#;
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 1);

        if let RenderBlock::Html { content } = &result.blocks[0] {
            assert!(content.contains("line1&#10;line2&#10;line3"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_code_block_inside_jsx_preserves_newlines() {
        let input = r#"<Steps>
```ts
line1
line2
line3
```
</Steps>"#;
        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();

        let component = result
            .blocks
            .iter()
            .find(|b| matches!(b, RenderBlock::Component { name, .. } if name == "Steps"));
        assert!(component.is_some());

        if let RenderBlock::Component { slot_html, .. } = component.unwrap() {
            assert!(slot_html.contains("line1&#10;line2&#10;line3"));
        }
    }

    #[test]
    fn test_indented_directive_inside_steps() {
        let input = r#"<Steps>

1. First step

2. Second step

    <PackageManagerTabs>
    content
    </PackageManagerTabs>

    :::tip
    Some tip content
    :::

3. Third step

</Steps>"#;

        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let steps_component = result
            .blocks
            .iter()
            .find(|b| matches!(b, RenderBlock::Component { name, .. } if name == "Steps"));
        assert!(steps_component.is_some());

        if let RenderBlock::Component { slot_html, .. } = steps_component.unwrap() {
            assert!(!slot_html.contains("<tip>"));
            assert!(slot_html.contains("<Aside") && slot_html.contains("type={\"tip\"}"));
            assert!(slot_html.contains("Some tip content"));
        }
    }

    #[test]
    fn test_jsx_component_inside_table_cell() {
        let input = r#"| Header |
| --- |
| <Box>content</Box> |"#;

        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 1);

        if let RenderBlock::Html { content } = &result.blocks[0] {
            assert!(content.contains("<td><Box>content</Box></td>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_mixed_text_and_component_inside_table_cell() {
        let input = r#"| Col |
| --- |
| before <Aside type="note">tip</Aside> after |"#;

        let options = Options {
            enable_directives: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 1);

        if let RenderBlock::Html { content } = &result.blocks[0] {
            assert!(content.contains("<td>before <Aside type={\"note\"}>tip</Aside> after</td>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    #[test]
    fn test_raw_html_passthrough() {
        let input = r#"
# Title

<script is:inline>
  const value = "hello {world}";
  console.log(value);
</script>

<style>
  body { color: red; }
</style>
"#;

        let options = Options {
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 1);

        if let RenderBlock::Html { content } = &result.blocks[0] {
            assert!(content.contains("<script is:inline>"));
            assert!(content.contains("console.log(value);"));
            assert!(content.contains("</script>"));
            assert!(content.contains("<style>"));
            assert!(content.contains("body { color: red; }"));
            assert!(content.contains("</style>"));
            assert!(!content.contains("&lt;script")); // ensure not escaped
        } else {
            panic!("Expected HTML block");
        }
    }
}
