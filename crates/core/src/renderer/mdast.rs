use markdown::mdast::Node;
use serde::Serialize;
use std::collections::HashMap;

use crate::transform::directives::{parse_opening_directive, is_directive_closer};

/// Represents a rendering block to be passed to Astro.
///
/// Each block is either plain HTML content or a component invocation
/// with props and slot content.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RenderBlock {
    /// Plain HTML content to be rendered with Astro's Fragment.
    Html {
        /// The HTML content string.
        content: String,
    },

    /// An Astro component to be dynamically rendered.
    Component {
        /// Component name (e.g., "note", "card").
        name: String,
        /// Component props as key-value pairs.
        props: HashMap<String, String>,
        /// HTML content for the component's default slot.
        slot_html: String,
    },
}

/// Manages the current rendering state with block-based architecture.
///
/// This struct tracks the rendering context as we traverse the markdown AST,
/// maintaining a list of completed blocks and a current HTML buffer for
/// content that hasn't been finalized into a block yet.
pub struct Context<'a> {
    /// Completed rendering blocks (HTML or Component).
    pub blocks: Vec<RenderBlock>,

    /// Current HTML buffer (not yet finalized into a block).
    pub current_html: String,

    stack: Vec<Scope>,
    #[allow(dead_code)]
    options: &'a Options,

    /// Stack of active directive metadata (for nested directives).
    directive_stack: Vec<DirectiveMeta>,
}

/// Metadata extracted from a directive placeholder tag.
#[derive(Debug, Clone)]
struct DirectiveMeta {
    name: String,
    title: Option<String>,
    #[allow(dead_code)]
    attrs: Option<String>,
}

/// Represents the type of scope currently being rendered.
///
/// Used in the Context stack to track which HTML element we are currently
/// inside of (e.g., inside a paragraph, inside a list, inside an Aside component).
#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    /// Document root - not inside any specific block element.
    Root,
    /// Inside a paragraph element (`<p>`).
    Paragraph,
    /// Inside a list element (`<ul>` or `<ol>`).
    List,
    /// Inside an Aside component with associated metadata.
    Aside(AsideMeta),
    /// Inside a Card component with associated metadata.
    Card(CardMeta),
}

/// Metadata for Aside components.
///
/// Stores the type of aside (e.g., "note", "warning", "tip") and an optional title.
#[derive(Debug, Clone, PartialEq)]
pub struct AsideMeta {
    /// The kind of aside (e.g., "note", "warning", "caution").
    pub kind: String,
    /// Optional title to display in the aside header.
    pub title: Option<String>,
}

/// Metadata for Card components.
///
/// Stores the card's title and an optional icon identifier.
#[derive(Debug, Clone, PartialEq)]
pub struct CardMeta {
    /// The title to display in the card header.
    pub title: String,
    /// Optional icon identifier for the card.
    pub icon: Option<String>,
}

impl<'a> Context<'a> {
    pub fn new(options: &'a Options) -> Self {
        Self {
            blocks: Vec::new(),
            current_html: String::with_capacity(4096),
            stack: vec![Scope::Root],
            options,
            directive_stack: Vec::new(),
        }
    }

    /// Writes a raw string to the current HTML buffer without escaping (for safe HTML tags).
    pub fn push_raw(&mut self, s: &str) {
        self.current_html.push_str(s);
    }

    /// Writes text content to the buffer with HTML escaping (public API).
    pub fn push_text(&mut self, s: &str) {
        self.push_escaped(s);
    }

    /// Writes HTML-escaped text to the current HTML buffer (internal use).
    ///
    /// Escapes `<`, `>`, and `&` characters for safe text node rendering.
    fn push_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.current_html.push_str("&lt;"),
                '>' => self.current_html.push_str("&gt;"),
                '&' => self.current_html.push_str("&amp;"),
                _ => self.current_html.push(c),
            }
        }
    }

    /// Writes HTML-escaped attribute value to the current HTML buffer (internal use).
    ///
    /// Escapes `<`, `>`, `&`, and `"` for safe attribute rendering.
    fn push_attr_value(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.current_html.push_str("&lt;"),
                '>' => self.current_html.push_str("&gt;"),
                '&' => self.current_html.push_str("&amp;"),
                '"' => self.current_html.push_str("&quot;"),
                _ => self.current_html.push(c),
            }
        }
    }

    /// Returns a reference to the current scope at the top of the stack.
    pub fn current_scope(&self) -> &Scope {
        self.stack.last().unwrap_or(&Scope::Root)
    }

    /// Enters a new scope by pushing it onto the stack.
    pub fn enter(&mut self, scope: Scope) {
        self.stack.push(scope);
    }

    /// Exits the current scope by popping from the stack.
    pub fn exit(&mut self) -> Option<Scope> {
        self.stack.pop()
    }

    /// Finalizes the current HTML buffer into an HTML block.
    ///
    /// This moves any pending HTML content from `current_html` into the `blocks` list.
    /// If the current HTML buffer is empty, this is a no-op.
    pub fn flush_html(&mut self) {
        if !self.current_html.is_empty() {
            let content = std::mem::take(&mut self.current_html);
            self.blocks.push(RenderBlock::Html { content });
        }
    }

    /// Adds a component block to the rendering output.
    ///
    /// This first flushes any pending HTML, then adds a Component block.
    pub fn push_component(&mut self, name: &str, props: HashMap<String, String>, slot_html: String) {
        self.flush_html();
        self.blocks.push(RenderBlock::Component {
            name: name.to_string(),
            props,
            slot_html,
        });
    }

    /// Renders child nodes to an HTML string (for component slots).
    ///
    /// Creates a temporary context to render the children, then combines
    /// all resulting blocks into a single HTML string.
    pub fn render_children_to_string(&self, children: &[Node]) -> String {
        let mut child_ctx = Context::new(self.options);

        for child in children {
            render_node(child, &mut child_ctx);
        }
        child_ctx.flush_html();

        // Combine all blocks into a single HTML string
        let mut result = String::new();
        for block in child_ctx.blocks {
            match block {
                RenderBlock::Html { content } => result.push_str(&content),
                RenderBlock::Component { name, slot_html, .. } => {
                    // Nested components are rendered as custom elements (fallback)
                    use std::fmt::Write;
                    let _ = write!(result, r#"<starlight-{} data-component>{}</starlight-{}>"#, name, slot_html, name);
                }
            }
        }
        result
    }

    /// Consumes the context and returns the list of rendering blocks.
    pub fn finish(mut self) -> Vec<RenderBlock> {
        self.flush_html();
        self.blocks
    }
}

/// Rendering options for the mdast v2 renderer.
pub struct Options {
    /// Whether to inject Starlight CSS when components are used.
    pub inject_starlight_css: bool,
    /// Whether directive processing is enabled.
    pub enable_directives: bool,
}

/// Preprocesses input markdown to convert directive syntax into placeholder HTML comments.
///
/// This allows markdown-rs to preserve directive structure even though it doesn't
/// natively support `::: note` syntax. Using HTML comments ensures markdown between
/// the markers is still parsed correctly.
///
/// # Examples
///
/// ```
/// Input:  ":::note[Title]\nContent\n:::"
/// Output: "<!--mf-directive-start data-name=\"note\" data-title=\"Title\"-->\nContent\n<!--mf-directive-end-->"
/// ```
fn preprocess_directives(input: &str) -> String {
    use crate::transform::code_fence::{FenceState, advance_fence_state};
    use std::fmt::Write;

    let mut fence_state = FenceState::default();
    let mut output = String::with_capacity(input.len());
    let mut directive_stack: Vec<String> = Vec::new(); // Track directive names for proper closing

    for line in input.lines() {
        let fence_outcome = advance_fence_state(line, fence_state);
        fence_state = fence_outcome.next_state;

        // Inside code fence - passthrough without processing
        if fence_outcome.skip_imports {
            writeln!(output, "{}", line).ok();
            continue;
        }

        // Check for directive opening
        if let Some(opening) = parse_opening_directive(line) {
            directive_stack.push(opening.name.clone());

            // Convert to HTML comment placeholder
            write!(output, "<!--mf-directive-start data-name=\"{}\"", opening.name).ok();

            if let Some(title) = &opening.bracket_title {
                // Escape quotes in title
                let escaped_title = title.replace('"', "&quot;");
                write!(output, " data-title=\"{}\"", escaped_title).ok();
            }

            if !opening.raw_attrs.is_empty() {
                write!(output, " data-attrs=\"{}\"", opening.raw_attrs.replace('"', "&quot;")).ok();
            }

            writeln!(output, "-->").ok();
            continue;
        }

        // Check for directive closer
        if is_directive_closer(line) && !directive_stack.is_empty() {
            directive_stack.pop();
            writeln!(output, "<!--mf-directive-end-->").ok();
            continue;
        }

        // Regular line - passthrough
        writeln!(output, "{}", line).ok();
    }

    // Close any unclosed directives
    while directive_stack.pop().is_some() {
        writeln!(output, "<!--mf-directive-end-->").ok();
    }

    output
}

/// Parses a directive placeholder HTML comment to extract metadata.
///
/// Returns Some(DirectiveMeta) if the HTML is an opening <!--mf-directive-start--> comment,
/// or None if it's a closing comment or not a directive placeholder.
fn parse_directive_placeholder(html: &str) -> Option<DirectiveMeta> {
    let trimmed = html.trim();

    // Check for closing comment
    if trimmed == "<!--mf-directive-end-->" {
        return None;
    }

    // Check for opening comment
    if !trimmed.starts_with("<!--mf-directive-start ") {
        return None;
    }

    // Extract data-name attribute
    let name = extract_attr(trimmed, "data-name")?;

    // Extract optional data-title attribute
    let title = extract_attr(trimmed, "data-title");

    // Extract optional data-attrs attribute
    let attrs = extract_attr(trimmed, "data-attrs");

    Some(DirectiveMeta { name, title, attrs })
}

/// Helper to extract an attribute value from an HTML tag string.
fn extract_attr(html: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    let start = html.find(&pattern)? + pattern.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];

    // Unescape HTML entities
    let unescaped = value.replace("&quot;", "\"");

    Some(unescaped)
}

/// Recursively renders an AST node to HTML, updating the context state.
fn render_node(node: &Node, ctx: &mut Context) {
    match node {
        // Root node - transparent container, just process children
        Node::Root(root) => {
            for child in &root.children {
                render_node(child, ctx);
            }
        }

        // Text node - escape and write to buffer
        Node::Text(text) => {
            ctx.push_text(&text.value);
        }

        // Paragraph node - wrap children in <p> tags
        Node::Paragraph(para) => {
            ctx.push_raw("<p>");
            ctx.enter(Scope::Paragraph);

            for child in &para.children {
                render_node(child, ctx);
            }

            ctx.exit();
            ctx.push_raw("</p>");
        }

        // Link node - render as <a> with escaped href
        Node::Link(link) => {
            ctx.push_raw(r#"<a href=""#);
            ctx.push_attr_value(&link.url);
            ctx.push_raw(r#"">"#);

            for child in &link.children {
                render_node(child, ctx);
            }

            ctx.push_raw("</a>");
        }

        // Strong node - render as <strong>
        Node::Strong(strong) => {
            ctx.push_raw("<strong>");
            for child in &strong.children {
                render_node(child, ctx);
            }
            ctx.push_raw("</strong>");
        }

        // Emphasis node - render as <em>
        Node::Emphasis(emphasis) => {
            ctx.push_raw("<em>");
            for child in &emphasis.children {
                render_node(child, ctx);
            }
            ctx.push_raw("</em>");
        }

        // InlineCode node - render as <code>
        Node::InlineCode(code) => {
            ctx.push_raw("<code>");
            ctx.push_text(&code.value);
            ctx.push_raw("</code>");
        }

        // HTML node - check for directive placeholders
        Node::Html(html) => {
            let html_str = html.value.as_ref();

            // Check if this is a directive placeholder
            if let Some(meta) = parse_directive_placeholder(html_str) {
                // Opening directive comment - push to stack
                ctx.directive_stack.push(meta);
                return; // Don't output the placeholder comment
            }

            // Check for closing directive comment
            if html_str.trim() == "<!--mf-directive-end-->" {
                // Pop directive metadata from stack
                if let Some(meta) = ctx.directive_stack.pop() {
                    // Extract slot HTML from current buffer
                    let slot_html = std::mem::take(&mut ctx.current_html);

                    // Build props from directive metadata
                    let mut props = HashMap::new();
                    if let Some(title) = meta.title {
                        props.insert("title".to_string(), title);
                    }
                    // TODO: Parse attrs string if needed

                    // Emit Component block
                    ctx.push_component(&meta.name, props, slot_html);
                }
                return; // Don't output the placeholder comment
            }

            // Regular HTML - pass through
            ctx.push_raw(html_str);
        }

        // Unhandled node types - log warning
        _ => {
            eprintln!("Warning: Unhandled node type: {:?}", node);
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
/// * `Ok(Vec<RenderBlock>)` - List of rendering blocks (HTML or Component)
/// * `Err(String)` - Error message if parsing fails
///
/// # Examples
///
/// ```
/// use markflow_core::renderer::mdast::{to_blocks, Options};
///
/// let input = "Hello, [world](https://example.com)!";
/// let options = Options {
///     inject_starlight_css: false,
///     enable_directives: false,
/// };
/// let blocks = to_blocks(input, &options).unwrap();
/// ```
pub fn to_blocks(input: &str, options: &Options) -> Result<Vec<RenderBlock>, String> {
    // 1. Preprocess directives if enabled
    let preprocessed = if options.enable_directives {
        preprocess_directives(input)
    } else {
        input.to_string()
    };

    // 2. Parse markdown to MDAST with enhanced options
    let parse_options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            // NOTE: Directive syntax (:::note, etc.) is preprocessed into <mf-directive> tags
            // before parsing, then detected as Node::Html during rendering.
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

    let tree = markdown::to_mdast(&preprocessed, &parse_options)
        .map_err(|e| format!("Markdown parse error: {}", e))?;

    // 2. Traverse the AST and render to blocks
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 3. Finish and return blocks
    Ok(ctx.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests basic text rendering without any markdown formatting.
    #[test]
    fn test_simple_text() {
        let input = "Hello, world!";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains("Hello, world!"));
            }
            _ => panic!("Expected HTML block"),
        }
    }

    /// Tests paragraph rendering with proper `<p>` tags.
    #[test]
    fn test_paragraph() {
        let input = "This is a paragraph.";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RenderBlock::Html { content } => {
                assert_eq!(content, "<p>This is a paragraph.</p>");
            }
            _ => panic!("Expected HTML block"),
        }
    }

    /// Tests link rendering with proper `<a>` tags and attribute escaping.
    #[test]
    fn test_link() {
        let input = "[Rust](https://www.rust-lang.org/)";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains(r#"<a href="https://www.rust-lang.org/""#));
                assert!(content.contains("Rust</a>"));
            }
            _ => panic!("Expected HTML block"),
        }
    }

    /// Tests directive syntax conversion to Component blocks.
    #[test]
    fn test_directive_to_component() {
        let input = ":::note[My Title]\nThis is **important** content.\n:::";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();

        // Should produce exactly 1 Component block
        assert_eq!(blocks.len(), 1, "Expected 1 block, got {}", blocks.len());

        match &blocks[0] {
            RenderBlock::Component { name, props, slot_html } => {
                assert_eq!(name, "note");
                assert_eq!(props.get("title"), Some(&"My Title".to_string()));
                assert!(slot_html.contains("<p>This is <strong>important</strong> content.</p>"));
            }
            _ => panic!("Expected Component block, got {:?}", blocks[0]),
        }
    }

    /// Tests directive without title.
    #[test]
    fn test_directive_without_title() {
        let input = ":::tip\nHelpful advice here.\n:::";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.len(), 1);

        match &blocks[0] {
            RenderBlock::Component { name, props, slot_html } => {
                assert_eq!(name, "tip");
                assert!(props.get("title").is_none());
                assert!(slot_html.contains("Helpful advice"));
            }
            _ => panic!("Expected Component block"),
        }
    }

    /// Tests that directives are disabled when enable_directives is false.
    #[test]
    fn test_directive_disabled() {
        let input = ":::note\nContent\n:::";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();

        // Should parse as regular paragraph text
        match &blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains(":::note"));
            }
            _ => panic!("Expected HTML block when directives disabled"),
        }
    }

    /// Debug test to inspect how directive syntax is parsed.
    ///
    /// Note: markdown-rs does not natively support directives,
    /// so this will show the raw AST structure for investigation.
    /// Run with `cargo test -- --nocapture` to see output.
    #[test]
    fn debug_directive_ast() {
        let input = ":::note[My Title]\nContent\n:::";

        // Use GFM parse options (same as to_blocks)
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

        // Parse and dump AST structure
        let tree = markdown::to_mdast(input, &parse_options).unwrap();

        println!("\n=== AST DEBUG START ===");
        println!("{:#?}", tree);
        println!("=== AST DEBUG END ===\n");

        // Also test multi-paragraph directive
        let input2 = ":::note\nFirst paragraph.\n\nSecond paragraph.\n:::";
        let tree2 = markdown::to_mdast(input2, &parse_options).unwrap();

        println!("\n=== MULTI-PARAGRAPH AST DEBUG START ===");
        println!("{:#?}", tree2);
        println!("=== MULTI-PARAGRAPH AST DEBUG END ===\n");
    }
}
