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
    collapse_multiline_wrapper_tags, normalize_list_jsx_components, normalize_mdx_jsx_indentation,
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
    false
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

    // 4. Normalize list-embedded JSX components (tab components in lists)
    let normalized = normalize_list_jsx_components(&normalized);

    // 5. Mask raw <script>/<style> blocks only when raw HTML passthrough is disabled.
    let (parsed_input, raw_masks) = if options.allow_raw_html() {
        (normalized.clone(), Vec::new())
    } else {
        mask_raw_html_blocks(&normalized)
    };

    // 6. Parse markdown to MDAST with enhanced options
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

    let tree = markdown::to_mdast(&parsed_input, &parse_options)
        .map_err(|e| format!("Markdown parse error: {}", e))?;

    // 7. Traverse the AST and render to blocks
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 8. Finish and get blocks, then unmask raw HTML that was temporarily hidden
    let mut result = ctx.finish();
    unmask_raw_html_blocks(&mut result.blocks, &raw_masks);

    // 9. Apply smartypants if enabled
    if options.enable_smartypants {
        for block in &mut result.blocks {
            if let RenderBlock::Html { content } = block {
                *content = apply_smartypants(content);
            }
        }
    }

    Ok(result)
}

/// A raw HTML block (script/style) that was temporarily masked during parsing.
#[derive(Debug, Clone)]
struct RawHtmlMask {
    marker: String,
    html: String,
}

/// Replace `<script>` / `<style>` blocks with stable markers before parsing so they
/// don't get rejected by the HTML parser when `html_flow` is disabled.
fn mask_raw_html_blocks(input: &str) -> (String, Vec<RawHtmlMask>) {
    let mut output = String::with_capacity(input.len());
    let mut masks = Vec::new();
    let mut cursor = 0;

    while let Some((fence_start, fence_delim)) = find_fence_start(&input[cursor..]) {
        // absolute position of fence start
        let abs_start = cursor + fence_start;

        // Mask any raw HTML that appears before the fence
        let plain = &input[cursor..abs_start];
        mask_in_plain_text(plain, &mut output, &mut masks);

        // Find fence end
        let fence_body_start = abs_start;
        let after_start = abs_start + fence_delim.len();
        if let Some(end_rel) = find_fence_end(&input[after_start..], &fence_delim) {
            let abs_end = after_start + end_rel;
            output.push_str(&input[fence_body_start..abs_end]);
            cursor = abs_end;
        } else {
            // No closing fence; push remainder and finish
            output.push_str(&input[fence_body_start..]);
            cursor = input.len();
            break;
        }
    }

    // Mask any trailing plain text
    if cursor < input.len() {
        let plain = &input[cursor..];
        mask_in_plain_text(plain, &mut output, &mut masks);
    }

    (output, masks)
}

/// Restore masked raw HTML markers back into rendered HTML/slot strings.
fn unmask_raw_html_blocks(blocks: &mut [RenderBlock], masks: &[RawHtmlMask]) {
    if masks.is_empty() {
        return;
    }

    for block in blocks {
        match block {
            RenderBlock::Html { content } => {
                for mask in masks {
                    if content.contains(&mask.marker) {
                        *content = content.replace(&mask.marker, &mask.html);
                    }
                }
            }
            RenderBlock::Component { slot_html, .. } => {
                for mask in masks {
                    if slot_html.contains(&mask.marker) {
                        *slot_html = slot_html.replace(&mask.marker, &mask.html);
                    }
                }
            }
        }
    }
}

/// Simple helper to locate the next <script>...</script> or <style>...</style> block.
struct TagMatch<'a> {
    start: usize,
    end: usize,
    block: &'a str,
}

fn find_next_tag(input: &str) -> Option<TagMatch<'_>> {
    let lower = input.to_ascii_lowercase();
    let script_pos = lower.find("<script");
    let style_pos = lower.find("<style");

    let (start, kind) = match (script_pos, style_pos) {
        (Some(s), Some(t)) => {
            if s < t {
                (s, "script")
            } else {
                (t, "style")
            }
        }
        (Some(s), None) => (s, "script"),
        (None, Some(t)) => (t, "style"),
        (None, None) => return None,
    };

    let closing = format!("</{}>", kind);
    let lower_tail = &lower[start..];
    let close_rel = lower_tail.find(&closing)?;
    let end = start + close_rel + closing.len();
    Some(TagMatch {
        start,
        end,
        block: &input[start..end],
    })
}

/// Mask script/style tags in a chunk of plain (non-code-fence) text.
fn mask_in_plain_text(segment: &str, out: &mut String, masks: &mut Vec<RawHtmlMask>) {
    let mut rest = segment;
    while let Some(pos) = find_next_tag(rest) {
        out.push_str(&rest[..pos.start]);
        let marker = format!("MARKFLOWRAWBLOCK{}MARK", masks.len());
        out.push_str(&marker);
        masks.push(RawHtmlMask {
            marker,
            html: pos.block.to_string(),
        });
        rest = &rest[pos.end..];
    }
    out.push_str(rest);
}

/// Locate the next code fence start (``` or ~~~) returning its start offset and delimiter.
fn find_fence_start(input: &str) -> Option<(usize, String)> {
    let mut offset = 0;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let delim: String = trimmed
                .chars()
                .take_while(|c| *c == '`' || *c == '~')
                .collect();
            return Some((offset, delim));
        }
        offset += line.len();
    }
    None
}

/// Locate the matching closing fence after a start; returns relative end index just after fence line.
fn find_fence_end(input: &str, delim: &str) -> Option<usize> {
    let mut offset = 0;
    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with(delim) {
            return Some(offset + line.len());
        }
        offset += line.len();
    }
    None
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
        assert_eq!(result.blocks.len(), 1);

        match &result.blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains("<script>alert('xss')</script>"));
                assert!(content.contains("&amp; symbols."));
                assert!(content.starts_with("<p>Text with "));
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
        assert!(tree.children().is_some());
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
            assert!(slot_html.contains("<Aside"));
            assert!(
                slot_html.contains("type={\"tip\"}")
                    || slot_html.contains("type=&#123;\"tip\"&#125;")
            );
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

    #[test]
    fn test_fragment_slot_escapes_braces_in_code_block() {
        let input = r#"<UIFrameworkTabs>
<Fragment slot="react">
```ts title="src/lib/auth-client.ts"
import { createAuthClient } from 'better-auth/react';

export const authClient = createAuthClient();
```
</Fragment>
</UIFrameworkTabs>"#;

        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);
        assert!(jsx.contains("createAuthClient"));
        assert!(jsx.contains("&#123;") && jsx.contains("&#125;"));
        assert!(!jsx.contains("{ createAuthClient }"));
    }

    #[test]
    fn test_inline_code_with_jsx_like_text_is_escaped() {
        let input = "`<PreactBanner client:load />` and `<SvelteCounter client:visible />`";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);

        assert!(jsx.contains("&lt;PreactBanner client:load /&gt;"));
        assert!(jsx.contains("&lt;SvelteCounter client:visible /&gt;"));
        assert!(!jsx.contains("<PreactBanner"));
        assert!(!jsx.contains("SvelteCounter client:visible />"));
    }

    #[test]
    fn test_spoiler_with_inline_code_does_not_emit_jsx_components() {
        let input = "<Spoiler>`<PreactBanner client:load />` und `<SvelteCounter client:visible />`</Spoiler>";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);

        assert!(jsx.contains("&lt;PreactBanner client:load /&gt;"));
        assert!(jsx.contains("&lt;SvelteCounter client:visible /&gt;"));
        assert!(!jsx.contains("<PreactBanner"));
        assert!(!jsx.contains("<SvelteCounter"));
    }

    #[test]
    fn test_spoiler_wrapped_in_raw_p_still_escapes_children() {
        let input = "<p><Spoiler>`<PreactBanner client:load />`</Spoiler></p>";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);
        assert!(jsx.contains("&lt;PreactBanner client:load /&gt;"));
        assert!(!jsx.contains("<PreactBanner"));
    }

    #[test]
    fn test_typescript_type_annotation_in_inline_code_escaped() {
        // TypeScript type annotations like Record<string, unknown> should be escaped
        let input = "**Type:** `Record<string, unknown>`";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);

        // The < and > should be escaped to &lt; and &gt;
        assert!(
            jsx.contains("Record&lt;string, unknown&gt;"),
            "Expected escaped TypeScript type annotation, got: {}",
            jsx
        );
        // There should be no literal < or > in the code content
        assert!(
            !jsx.contains("<string"),
            "Should not contain literal <string, got: {}",
            jsx
        );
    }

    #[test]
    fn test_complex_typescript_type_in_inline_code() {
        // More complex TypeScript type from astro docs
        let input = "**Type:** `(appId: string, callback: (data: Record<string, never>) => void) => void`";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);

        // All < and > should be escaped
        assert!(
            jsx.contains("Record&lt;string, never&gt;"),
            "Expected escaped Record type, got: {}",
            jsx
        );
        assert!(
            jsx.contains("=&gt; void"),
            "Expected escaped arrow function, got: {}",
            jsx
        );
        // No literal < or > in code
        assert!(
            !jsx.contains("<string") && !jsx.contains("> void"),
            "Should not contain literal angle brackets in type, got: {}",
            jsx
        );
    }

    #[test]
    fn test_html_numeric_entity_escaping() {
        // HTML numeric entities like &#123; should have their & escaped to prevent JSX issues
        let input = "&#123; test &#125;";
        let options = Options {
            enable_directives: true,
            allow_raw_html: true,
            ..Default::default()
        };

        let result = to_blocks(input, &options).unwrap();
        let jsx =
            crate::codegen::blocks_to_jsx_string(&result.blocks, None::<fn(&str) -> Option<_>>);

        // The & should be escaped so the entity doesn't get decoded by the browser
        // but also doesn't cause JSX parsing issues
        // Expected: &amp;#123; or &#123; preserved as text (not decoded to literal {)
        println!("JSX output: {}", jsx);

        // Check that there are no literal braces (which would be problematic in JSX)
        assert!(
            !jsx.contains("{ test }"),
            "HTML entities should not be decoded to literal braces, got: {}",
            jsx
        );
    }
}
