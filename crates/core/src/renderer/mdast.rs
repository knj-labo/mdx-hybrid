use markdown::mdast::Node;

/// Manages the current rendering state with encapsulated buffer and stack.
///
/// This struct tracks the rendering context as we traverse the markdown AST,
/// maintaining an output buffer and a pushdown automaton stack to handle
/// nested element scoping correctly.
pub struct Context<'a> {
    buf: String,
    stack: Vec<Scope>,
    #[allow(dead_code)]
    options: &'a Options,
    needs_starlight_css: bool,
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
            buf: String::with_capacity(1024 * 16),
            stack: vec![Scope::Root],
            options,
            needs_starlight_css: false,
        }
    }

    /// Writes a raw string to the buffer without escaping (for safe HTML tags).
    pub fn push_raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Writes text content to the buffer with HTML escaping (public API).
    pub fn push_text(&mut self, s: &str) {
        self.push_escaped(s);
    }

    /// Writes HTML-escaped text to the buffer (internal use).
    ///
    /// Escapes `<`, `>`, and `&` characters for safe text node rendering.
    fn push_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.buf.push_str("&lt;"),
                '>' => self.buf.push_str("&gt;"),
                '&' => self.buf.push_str("&amp;"),
                _ => self.buf.push(c),
            }
        }
    }

    /// Writes HTML-escaped attribute value to the buffer (internal use).
    ///
    /// Escapes `<`, `>`, `&`, and `"` for safe attribute rendering.
    fn push_attr_value(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.buf.push_str("&lt;"),
                '>' => self.buf.push_str("&gt;"),
                '&' => self.buf.push_str("&amp;"),
                '"' => self.buf.push_str("&quot;"),
                _ => self.buf.push(c),
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

    /// Marks that a Starlight component has been used (requires CSS injection).
    pub fn mark_starlight_used(&mut self) {
        self.needs_starlight_css = true;
    }

    /// Consumes the context and returns the final HTML string.
    pub fn finish(self) -> String {
        self.buf
    }
}

/// Rendering options for the mdast v2 renderer.
pub struct Options {
    /// Whether to inject Starlight CSS when components are used.
    pub inject_starlight_css: bool,
    /// Whether directive processing is enabled (for future use).
    pub enable_directives: bool,
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

        // Unhandled node types - log warning
        _ => {
            eprintln!("Warning: Unhandled node type: {:?}", node);
        }
    }
}

/// Converts Markdown input to HTML (entry point).
///
/// # Arguments
///
/// * `input` - The markdown text to convert
/// * `options` - Rendering options (CSS injection, directives, etc.)
///
/// # Returns
///
/// * `Ok(String)` - The generated HTML string
/// * `Err(String)` - Error message if parsing fails
///
/// # Examples
///
/// ```
/// use markflow_core::renderer::mdast::{to_html, Options};
///
/// let input = "Hello, **world**!";
/// let options = Options {
///     inject_starlight_css: false,
///     enable_directives: false,
/// };
/// let html = to_html(input, &options).unwrap();
/// ```
pub fn to_html(input: &str, options: &Options) -> Result<String, String> {
    // 1. Parse markdown to MDAST with enhanced options
    let parse_options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            // Enable frontmatter (--- ... ---)
            frontmatter: true,
            // Enable GitHub Flavored Markdown features
            gfm_autolink_literal: true,
            gfm_strikethrough: true,
            gfm_table: true,
            gfm_task_list_item: true,
            ..markdown::Constructs::default()
        },
        ..markdown::ParseOptions::default()
    };

    let tree = markdown::to_mdast(input, &parse_options)
        .map_err(|e| format!("Markdown parse error: {}", e))?;

    // 2. Traverse the AST and render to HTML
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 3. Check CSS flag before consuming context
    let needs_css = ctx.needs_starlight_css;
    let mut result = ctx.finish();

    // 4. Post-process: inject CSS if needed
    if needs_css && options.inject_starlight_css {
        // TODO: Load actual starlight.css file
        let style_tag = "<style is:global>/* Starlight CSS will be injected here */</style>\n";
        result = format!("{}{}", style_tag, result);
    }

    Ok(result)
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

        let result = to_html(input, &options).unwrap();
        assert!(result.contains("Hello, world!"));
    }

    /// Tests paragraph rendering with proper `<p>` tags.
    #[test]
    fn test_paragraph() {
        let input = "This is a paragraph.";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let result = to_html(input, &options).unwrap();
        assert!(result.contains("<p>"));
        assert!(result.contains("</p>"));
        assert!(result.contains("This is a paragraph."));
    }

    /// Tests link rendering with proper `<a>` tags and attribute escaping.
    #[test]
    fn test_link() {
        let input = "[Rust](https://www.rust-lang.org/)";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let result = to_html(input, &options).unwrap();
        assert!(result.contains(r#"<a href="https://www.rust-lang.org/""#));
        assert!(result.contains("Rust</a>"));
    }

    /// Debug test to inspect how directive syntax is parsed.
    ///
    /// Note: markdown-rs does not natively support directives,
    /// so this will show the raw AST structure for investigation.
    /// Run with `cargo test -- --nocapture` to see output.
    #[test]
    fn debug_directive_ast() {
        let input = ":::note[My Title]\nContent\n:::";

        // Use default parse options (directives not natively supported)
        let parse_options = markdown::ParseOptions::default();

        // Parse and dump AST structure
        let tree = markdown::to_mdast(input, &parse_options).unwrap();

        println!("=== AST DEBUG START ===");
        println!("{:#?}", tree);
        println!("=== AST DEBUG END ===");
    }
}
