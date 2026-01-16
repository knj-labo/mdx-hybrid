//! Rendering context for the mdast renderer.

use super::Options;
use super::types::{BlocksResult, HeadingEntry, PropValue, RenderBlock, Scope};
use markdown::mdast::Node;
use std::collections::HashMap;

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

    /// Extracted heading metadata for table of contents.
    pub headings: Vec<HeadingEntry>,

    /// Slugger for generating unique heading IDs.
    slugger: crate::Slugger,

    stack: Vec<Scope>,
    #[allow(dead_code)]
    options: &'a Options,
}

impl<'a> Context<'a> {
    pub fn new(options: &'a Options) -> Self {
        Self {
            blocks: Vec::new(),
            current_html: String::with_capacity(4096),
            headings: Vec::new(),
            slugger: crate::Slugger::new(),
            stack: vec![Scope::Root],
            options,
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

    /// Writes code content to the buffer with JSX-safe escaping.
    ///
    /// This escapes curly braces in addition to HTML entities to prevent
    /// JSX interpreting `{` and `}` as expression delimiters within code blocks.
    pub fn push_code_text(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.current_html.push_str("&lt;"),
                '>' => self.current_html.push_str("&gt;"),
                '&' => self.current_html.push_str("&amp;"),
                '`' => self.current_html.push_str("&#96;"),
                '{' => self.current_html.push_str("&#123;"),
                '}' => self.current_html.push_str("&#125;"),
                // Encode newlines as HTML entities to prevent esbuild's JSX
                // transform from stripping them (esbuild normalizes whitespace
                // in JSX text children, converting \n to spaces)
                '\n' => self.current_html.push_str("&#10;"),
                _ => self.current_html.push(c),
            }
        }
    }

    /// Writes HTML-escaped text to the current HTML buffer (internal use).
    ///
    /// Escapes `<`, `>`, `&`, and `` ` `` characters for safe text node rendering.
    /// Backticks are escaped to prevent template literal injection in JSX contexts.
    fn push_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.current_html.push_str("&lt;"),
                '>' => self.current_html.push_str("&gt;"),
                '&' => self.current_html.push_str("&amp;"),
                '`' => self.current_html.push_str("&#96;"),
                _ => self.current_html.push(c),
            }
        }
    }

    /// Writes HTML-escaped attribute value to the current HTML buffer (internal use).
    ///
    /// Escapes `<`, `>`, `&`, `"`, and `'` for safe attribute rendering.
    pub fn push_attr_value(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.current_html.push_str("&lt;"),
                '>' => self.current_html.push_str("&gt;"),
                '&' => self.current_html.push_str("&amp;"),
                '"' => self.current_html.push_str("&quot;"),
                '\'' => self.current_html.push_str("&#39;"),
                _ => self.current_html.push(c),
            }
        }
    }

    /// Returns a reference to the current scope at the top of the stack.
    pub fn current_scope(&self) -> &Scope {
        self.stack.last().unwrap_or(&Scope::Root)
    }

    /// Returns true if any scope in the stack is a List.
    ///
    /// Used to determine if JSX components should be rendered inline
    /// to avoid fragmenting list structures.
    pub fn is_in_list(&self) -> bool {
        self.stack.iter().any(|scope| matches!(scope, Scope::List))
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
    pub fn push_component(
        &mut self,
        name: &str,
        props: HashMap<String, PropValue>,
        slot_html: String,
    ) {
        self.flush_html();
        self.blocks.push(RenderBlock::Component {
            name: name.to_string(),
            props,
            slot_html,
        });
    }

    /// Renders a component inline to the HTML buffer as JSX.
    ///
    /// Used when inside a list to avoid fragmenting the list structure.
    /// Instead of creating a separate Component block (which would flush
    /// the HTML buffer), this writes the component directly as JSX syntax.
    pub fn push_component_inline(
        &mut self,
        name: &str,
        props: &HashMap<String, PropValue>,
        slot_html: &str,
    ) {
        self.current_html.push('<');
        self.current_html.push_str(name);

        for (key, prop_value) in props {
            self.current_html.push(' ');
            self.current_html.push_str(key);
            self.current_html.push_str("={");
            match prop_value {
                PropValue::Literal { value } => {
                    self.current_html.push('"');
                    self.current_html.push_str(&value.replace('"', "\\\""));
                    self.current_html.push('"');
                }
                PropValue::Expression { value } => {
                    self.current_html.push_str(value);
                }
            }
            self.current_html.push('}');
        }

        self.current_html.push('>');
        self.current_html.push_str(slot_html);
        self.current_html.push_str("</");
        self.current_html.push_str(name);
        self.current_html.push('>');
    }

    /// Renders child nodes to an HTML string (for component slots).
    ///
    /// Creates a temporary context to render the children, then combines
    /// all resulting blocks into a single HTML string.
    ///
    /// **Important:** This also bubbles up any headings found in the children
    /// to the parent context, ensuring JSX component content appears in the TOC.
    pub fn render_children_to_string(&mut self, children: &[Node]) -> String {
        // Import render_node here to avoid circular dependency at module level
        use super::render::render_node;

        let mut child_ctx = Context::new(self.options);

        for child in children {
            render_node(child, &mut child_ctx);
        }
        child_ctx.flush_html();

        // Bubble up headings from child context to parent (for TOC)
        self.headings.append(&mut child_ctx.headings);

        // Combine all blocks into a single HTML string
        let mut result = String::new();
        for block in child_ctx.blocks {
            match block {
                RenderBlock::Html { content } => result.push_str(&content),
                RenderBlock::Component {
                    name,
                    props,
                    slot_html,
                } => {
                    // Render nested components as JSX with props preserved
                    result.push('<');
                    result.push_str(&name);

                    // Render props as JSX: key={"value"} or key={expression}
                    for (key, prop_value) in &props {
                        result.push(' ');
                        result.push_str(key);
                        result.push_str("={");
                        match prop_value {
                            PropValue::Literal { value } => {
                                result.push('"');
                                result.push_str(&value.replace('"', "\\\""));
                                result.push('"');
                            }
                            PropValue::Expression { value } => {
                                result.push_str(value);
                            }
                        }
                        result.push('}');
                    }

                    result.push('>');
                    result.push_str(&slot_html);
                    result.push_str("</");
                    result.push_str(&name);
                    result.push('>');
                }
            }
        }
        result
    }

    /// Generates a unique slug for a heading.
    pub fn generate_slug(&mut self, text: &str) -> String {
        self.slugger.next_slug(text)
    }

    /// Adds a heading entry to the list of headings.
    pub fn add_heading(&mut self, entry: HeadingEntry) {
        self.headings.push(entry);
    }

    /// Returns whether lazy image loading is enabled.
    pub fn lazy_images_enabled(&self) -> bool {
        self.options.lazy_images()
    }

    /// Consumes the context and returns the list of rendering blocks.
    pub fn finish(mut self) -> BlocksResult {
        self.flush_html();
        BlocksResult {
            blocks: self.blocks,
            headings: self.headings,
        }
    }
}
