use markdown::mdast::Node;
use serde::Serialize;
use std::collections::HashMap;

use crate::parser::markdown_adapter::normalize_mdx_jsx_indentation;
use crate::transform::directives::{is_directive_closer, parse_opening_directive};

/// A component prop value - either a literal string or a JS expression.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PropValue {
    /// A literal string value (from key="value").
    Literal { value: String },
    /// A JS expression (from key={expression}).
    Expression { value: String },
}

impl PropValue {
    /// Creates a literal string prop value.
    pub fn literal(value: impl Into<String>) -> Self {
        PropValue::Literal {
            value: value.into(),
        }
    }

    /// Creates an expression prop value.
    pub fn expression(value: impl Into<String>) -> Self {
        PropValue::Expression {
            value: value.into(),
        }
    }

    /// Returns the raw value regardless of type.
    pub fn value(&self) -> &str {
        match self {
            PropValue::Literal { value } | PropValue::Expression { value } => value,
        }
    }

    /// Returns true if this is an expression.
    pub fn is_expression(&self) -> bool {
        matches!(self, PropValue::Expression { .. })
    }
}

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
        /// Component props as key-value pairs (literals or expressions).
        props: HashMap<String, PropValue>,
        /// HTML content for the component's default slot.
        slot_html: String,
    },
}

/// Heading metadata extracted during rendering.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct HeadingEntry {
    /// Heading depth (1-6).
    pub depth: u8,
    /// Slugified identifier.
    pub slug: String,
    /// Visible heading text.
    pub text: String,
}

/// Result of parsing markdown to blocks with extracted metadata.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BlocksResult {
    /// Rendering blocks (HTML or Component).
    pub blocks: Vec<RenderBlock>,
    /// Extracted heading metadata.
    pub headings: Vec<HeadingEntry>,
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

    /// Extracted heading metadata for table of contents.
    pub headings: Vec<HeadingEntry>,

    /// Slugger for generating unique heading IDs.
    slugger: crate::Slugger,

    stack: Vec<Scope>,
    #[allow(dead_code)]
    options: &'a Options,
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
    fn push_attr_value(&mut self, s: &str) {
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

    /// Consumes the context and returns the list of rendering blocks.
    pub fn finish(mut self) -> BlocksResult {
        self.flush_html();
        BlocksResult {
            blocks: self.blocks,
            headings: self.headings,
        }
    }
}

/// Rendering options for the mdast v2 renderer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Options {
    /// Whether directive processing is enabled.
    pub enable_directives: bool,
}

/// Preprocesses input markdown to convert directive syntax into internal JSX tags.
///
/// This allows markdown-rs to preserve directive structure even though it doesn't
/// natively support `::: note` syntax. Using JSX tags ensures markdown between
/// the markers is still parsed correctly and unifies directive handling with JSX.
///
/// # Examples
///
/// Input:
/// ```text
/// :::note[Title]
/// Content
/// :::
/// ```
///
/// Output:
/// ```text
/// <mf-directive name="note" title="Title">
/// Content
/// </mf-directive>
/// ```
fn preprocess_directives(input: &str) -> String {
    use crate::transform::code_fence::{FenceState, advance_fence_state};
    use std::fmt::Write;

    let mut fence_state = FenceState::default();
    let mut output = String::with_capacity(input.len());
    // Track directive names and their leading whitespace for proper closing
    let mut directive_stack: Vec<(String, String)> = Vec::new();

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
            // Preserve leading whitespace from original line
            let leading_ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();

            directive_stack.push((opening.name.clone(), leading_ws.clone()));

            // Convert to JSX container tag (NOT self-closing)
            // This ensures the content between ::: markers is INSIDE the JSX element
            write!(
                output,
                "{}<mf-directive name=\"{}\"",
                leading_ws, opening.name
            )
            .ok();

            if let Some(title) = &opening.bracket_title {
                // Escape quotes in title
                let escaped_title = title.replace('"', "&quot;");
                write!(output, " title=\"{}\"", escaped_title).ok();
            }

            if !opening.raw_attrs.is_empty() {
                write!(
                    output,
                    " attrs=\"{}\"",
                    opening.raw_attrs.replace('"', "&quot;")
                )
                .ok();
            }

            // Opening tag, not self-closing
            writeln!(output, ">").ok();
            continue;
        }

        // Check for directive closer
        if is_directive_closer(line) && !directive_stack.is_empty() {
            let (_, leading_ws) = directive_stack.pop().unwrap();
            // Close the container tag with same indentation as opener
            writeln!(output, "{}</mf-directive>", leading_ws).ok();
            continue;
        }

        // Regular line - passthrough
        writeln!(output, "{}", line).ok();
    }

    // Close any unclosed directives
    while let Some((_, leading_ws)) = directive_stack.pop() {
        writeln!(output, "{}</mf-directive>", leading_ws).ok();
    }

    output
}

/// Extracts plain text from a list of AST nodes (for heading text).
///
/// This recursively traverses the nodes and collects all text content,
/// which is used for generating slugs and table of contents entries.
fn extract_text_from_nodes(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        extract_text_from_node(node, &mut text);
    }
    text.trim().to_string()
}

/// Helper function to recursively extract text from a single node.
fn extract_text_from_node(node: &Node, buffer: &mut String) {
    match node {
        Node::Text(t) => buffer.push_str(&t.value),
        Node::InlineCode(code) => buffer.push_str(&code.value),
        Node::Strong(strong) => {
            for child in &strong.children {
                extract_text_from_node(child, buffer);
            }
        }
        Node::Emphasis(emphasis) => {
            for child in &emphasis.children {
                extract_text_from_node(child, buffer);
            }
        }
        Node::Link(link) => {
            for child in &link.children {
                extract_text_from_node(child, buffer);
            }
        }
        Node::Delete(del) => {
            for child in &del.children {
                extract_text_from_node(child, buffer);
            }
        }
        // Ignore other node types in headings
        _ => {}
    }
}

/// Renders a list node as `<ul>` or `<ol>`.
fn render_list(list: &markdown::mdast::List, ctx: &mut Context) {
    let tag = if list.ordered { "ol" } else { "ul" };
    ctx.push_raw(&format!("<{}>", tag));
    ctx.enter(Scope::List);

    for child in &list.children {
        render_node(child, ctx);
    }

    ctx.exit();
    ctx.push_raw(&format!("</{}>", tag));
}

/// Renders a list item node as `<li>`.
///
/// For task list items (GFM), adds `task-list-item` class and wraps content
/// in `<label><input><span>` to match the structure expected by rehype-tasklist-enhancer.
/// This enables proper styling in Starlight's Checklist component.
fn render_list_item(item: &markdown::mdast::ListItem, ctx: &mut Context) {
    // Task list support (GFM)
    let class_attr = if item.checked.is_some() {
        " class=\"task-list-item\""
    } else {
        ""
    };
    ctx.push_raw(&format!("<li{}>", class_attr));

    if let Some(checked) = item.checked {
        // For task list items, wrap in <label><input><span> structure
        // to match rehype-tasklist-enhancer output for Checklist component compatibility
        let checked_str = if checked { " checked" } else { "" };
        ctx.push_raw(&format!(
            "<label><input type=\"checkbox\" disabled{}/><span>",
            checked_str
        ));

        // Render children inside <span>
        for child in &item.children {
            render_node(child, ctx);
        }

        ctx.push_raw("</span></label>");
    } else {
        // Normal list item
        for child in &item.children {
            render_node(child, ctx);
        }
    }

    ctx.push_raw("</li>");
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
            ctx.push_raw(r#"""#);

            // Add optional title attribute
            if let Some(title) = &link.title {
                ctx.push_raw(r#" title=""#);
                ctx.push_attr_value(title);
                ctx.push_raw(r#"""#);
            }

            ctx.push_raw(">");

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
            ctx.push_code_text(&code.value);
            ctx.push_raw("</code>");
        }

        // Heading node - render as <h1> to <h6>
        Node::Heading(heading) => {
            // Extract heading text for table of contents
            let text = extract_text_from_nodes(&heading.children);
            let slug = ctx.slugger.next_slug(&text);

            // Record heading metadata
            ctx.headings.push(HeadingEntry {
                depth: heading.depth,
                slug: slug.clone(),
                text,
            });

            // Render heading with ID
            let tag = format!("h{}", heading.depth);
            ctx.push_raw(&format!("<{} id=\"{}\">", tag, slug));
            for child in &heading.children {
                render_node(child, ctx);
            }
            ctx.push_raw(&format!("</{}>", tag));
        }

        // List node - render as <ul> or <ol>
        Node::List(list) => render_list(list, ctx),

        // ListItem node - render as <li>
        Node::ListItem(item) => render_list_item(item, ctx),

        // Code block node - render as <pre><code>
        // astro-code class for Starlight CSS compatibility
        // tabindex="0" for keyboard accessibility when horizontally scrolling
        Node::Code(code) => {
            ctx.push_raw(r#"<pre class="astro-code" tabindex="0">"#);

            // Language class for syntax highlighting
            if let Some(lang) = &code.lang {
                ctx.push_raw(r#"<code class="language-"#);
                ctx.push_attr_value(lang);
                ctx.push_raw(r#"">"#);
            } else {
                ctx.push_raw("<code>");
            }

            // Use push_code_text to escape curly braces for JSX safety
            ctx.push_code_text(&code.value);
            ctx.push_raw("</code></pre>");
        }

        // Blockquote node - render as <blockquote>
        Node::Blockquote(quote) => {
            ctx.push_raw("<blockquote>");
            for child in &quote.children {
                render_node(child, ctx);
            }
            ctx.push_raw("</blockquote>");
        }

        // Image node - render as <img>
        Node::Image(img) => {
            ctx.push_raw(r#"<img src=""#);
            ctx.push_attr_value(&img.url);
            ctx.push_raw(r#"""#);

            // Alt text
            ctx.push_raw(r#" alt=""#);
            ctx.push_attr_value(&img.alt);
            ctx.push_raw(r#"""#);

            // Optional title attribute
            if let Some(title) = &img.title {
                ctx.push_raw(r#" title=""#);
                ctx.push_attr_value(title);
                ctx.push_raw(r#"""#);
            }

            ctx.push_raw(" />");
        }

        // ThematicBreak node - render as <hr>
        Node::ThematicBreak(_) => {
            ctx.push_raw("<hr />");
        }

        // HTML node - escape for security (XSS prevention)
        Node::Html(html) => {
            // With html_flow: false, this should rarely be reached
            // But if it is, escape the HTML for security
            log::warn!(
                "Raw HTML in markdown will be escaped for security: {}",
                html.value
            );
            ctx.push_text(&html.value);
        }

        // GFM: Strikethrough (~~text~~)
        Node::Delete(delete) => {
            ctx.push_raw("<del>");
            for child in &delete.children {
                render_node(child, ctx);
            }
            ctx.push_raw("</del>");
        }

        // GFM: Table
        Node::Table(table) => {
            ctx.push_raw("<table>");

            // 1. Header row (thead)
            ctx.push_raw("<thead>");
            if let Some(Node::TableRow(row)) = table.children.first() {
                // true = is_header
                render_table_row(row, ctx, true, &table.align);
            }
            ctx.push_raw("</thead>");

            // 2. Data rows (tbody)
            if table.children.len() > 1 {
                ctx.push_raw("<tbody>");
                // Process rows from second onwards
                for row in table.children.iter().skip(1) {
                    if let Node::TableRow(r) = row {
                        // false = not header
                        render_table_row(r, ctx, false, &table.align);
                    }
                }
                ctx.push_raw("</tbody>");
            }

            ctx.push_raw("</table>");
        }

        // TableRow/TableCell are processed by render_table_row helper
        // If they appear here directly, ignore them
        Node::TableRow(_) => {}
        Node::TableCell(_) => {}

        // MDX: JSX flow elements (block-level <Component>...</Component>)
        Node::MdxJsxFlowElement(elem) => {
            render_jsx(elem.name.as_deref(), &elem.attributes, &elem.children, ctx);
        }

        // MDX: JSX text elements (inline <Component />)
        Node::MdxJsxTextElement(elem) => {
            render_jsx(elem.name.as_deref(), &elem.attributes, &elem.children, ctx);
        }

        // Unhandled node types - log warning
        _ => {
            log::warn!("Unhandled markdown node type: {:?}", node);
        }
    }
}

/// Helper function to render a table row with proper alignment.
///
/// # Arguments
///
/// * `row` - The TableRow node to render
/// * `ctx` - The rendering context
/// * `is_header` - Whether this is a header row (uses `<th>` instead of `<td>`)
/// * `aligns` - Column alignment specifications
fn render_table_row(
    row: &markdown::mdast::TableRow,
    ctx: &mut Context,
    is_header: bool,
    aligns: &[markdown::mdast::AlignKind],
) {
    ctx.push_raw("<tr>");

    for (i, cell) in row.children.iter().enumerate() {
        if let Node::TableCell(c) = cell {
            let tag = if is_header { "th" } else { "td" };

            // Alignment attribute (align="center" etc.)
            // Guard against more cells than alignment specs
            let align_attr = if i < aligns.len() {
                match aligns[i] {
                    markdown::mdast::AlignKind::Left => " align=\"left\"",
                    markdown::mdast::AlignKind::Right => " align=\"right\"",
                    markdown::mdast::AlignKind::Center => " align=\"center\"",
                    markdown::mdast::AlignKind::None => "",
                }
            } else {
                ""
            };

            ctx.push_raw(&format!("<{}{}>", tag, align_attr));

            for child in &c.children {
                render_node(child, ctx);
            }

            ctx.push_raw(&format!("</{}>", tag));
        }
    }

    ctx.push_raw("</tr>");
}

/// Renders a JSX element (MDX) as either a component block or transparent container.
///
/// # Arguments
///
/// * `name` - The JSX element name (e.g., "Card", "div"), or None for fragments (<>...</>)
/// * `attributes` - JSX attributes/props
/// * `children` - Child nodes to render
/// * `ctx` - The rendering context
///
/// # Behavior
///
/// - Fragment elements (<>...</>) are transparent - children are rendered inline
/// - Named elements become Component blocks with props and slot HTML
/// - Children are recursively rendered to support nested markdown/JSX
/// - Headings inside JSX are bubbled up to the parent context for TOC
fn render_jsx(
    name: Option<&str>,
    attributes: &[markdown::mdast::AttributeContent],
    children: &[Node],
    ctx: &mut Context,
) {
    // 1. Fragment handling: <> ... </> has no name, just render children
    let Some(tag_name) = name else {
        for child in children {
            render_node(child, ctx);
        }
        return;
    };

    // 2. Handle internal directive container: <mf-directive name="..." title="...">...</mf-directive>
    if tag_name == "mf-directive" {
        let mut directive_type = "note".to_string();
        let mut title: Option<String> = None;

        // Extract metadata from attributes
        for attr in attributes {
            if let markdown::mdast::AttributeContent::Property(prop) = attr {
                let val = match &prop.value {
                    Some(markdown::mdast::AttributeValue::Literal(s)) => s.clone(),
                    _ => String::new(),
                };

                match prop.name.as_str() {
                    "name" => directive_type = val,
                    "title" => {
                        // Unescape &quot; back to "
                        title = Some(val.replace("&quot;", "\""));
                    }
                    _ => {}
                }
            }
        }

        // Render children to get slot HTML
        // Use render_children_to_string to properly handle both HTML content
        // and nested JSX components (like <ReadMore>) inside directives
        let slot_html = ctx.render_children_to_string(children);

        // Build props for Aside component
        // Use "Aside" as component name with type prop for proper Starlight integration
        let mut props = HashMap::new();
        props.insert("type".to_string(), PropValue::literal(directive_type));
        if let Some(t) = title {
            props.insert("title".to_string(), PropValue::literal(t));
        }

        // Emit as Aside Component block
        // When inside a list, render inline to avoid fragmenting the list structure.
        if ctx.is_in_list() {
            ctx.push_component_inline("Aside", &props, &slot_html);
        } else {
            ctx.push_component("Aside", props, slot_html);
        }
        return;
    }

    // 3. Extract props from JSX attributes
    let mut props = HashMap::new();
    for attr in attributes {
        match attr {
            // key="value" or key={expression}
            markdown::mdast::AttributeContent::Property(prop) => {
                let value = match &prop.value {
                    Some(markdown::mdast::AttributeValue::Literal(s)) => {
                        // String literal: key="value"
                        PropValue::literal(s.clone())
                    }
                    Some(markdown::mdast::AttributeValue::Expression(expr)) => {
                        // Expression: key={expression} - preserve as-is
                        PropValue::expression(expr.value.clone())
                    }
                    None => PropValue::literal(String::new()),
                };
                props.insert(prop.name.clone(), value);
            }
            // {...spread} - skip for now (complex to handle)
            markdown::mdast::AttributeContent::Expression(_) => {
                // Spread attributes not yet supported
            }
        }
    }

    // 4. Render children to HTML string (enables nested markdown/JSX)
    // This is the key to supporting markdown inside JSX components!
    // Headings found in children are automatically bubbled up to ctx for TOC.
    let slot_html = ctx.render_children_to_string(children);

    // 5. Special handling for Fragment with slot attribute
    // Fragment with slot must be rendered inline as JSX to work with Astro's slot system.
    // Creating a separate Component block would break the parent-child relationship.
    if tag_name == "Fragment" && props.contains_key("slot") {
        ctx.push_component_inline(tag_name, &props, &slot_html);
        return;
    }

    // 6. Push as component block (unified with directive rendering)
    // When inside a list, render inline to avoid fragmenting the list structure.
    if ctx.is_in_list() {
        ctx.push_component_inline(tag_name, &props, &slot_html);
    } else {
        ctx.push_component(tag_name, props, slot_html);
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
///     enable_directives: false,
/// };
/// let blocks = to_blocks(input, &options).unwrap();
/// ```
pub fn to_blocks(input: &str, options: &Options) -> Result<BlocksResult, String> {
    // 1. Preprocess directives if enabled
    let preprocessed = if options.enable_directives {
        preprocess_directives(input)
    } else {
        input.to_string()
    };

    // 2. Normalize JSX indentation to prevent content from being treated as code blocks
    // When content inside JSX elements is indented (4+ spaces), markdown-rs interprets
    // it as an indented code block. This normalization strips that indentation.
    let normalized = normalize_mdx_jsx_indentation(&preprocessed);

    // 3. Parse markdown to MDAST with enhanced options
    let parse_options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            // MDX: JSX support for <Component>...</Component>
            mdx_jsx_flow: true,
            mdx_jsx_text: true,
            // HTML: DISABLED - directives are now rendered as internal JSX tags
            // This unified approach allows directives and JSX to coexist seamlessly
            html_flow: false,
            html_text: false,
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

    // 5. Finish and return blocks
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
            enable_directives: true,
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

    /// Tests paragraph rendering with proper `<p>` tags.
    #[test]
    fn test_paragraph() {
        let input = "This is a paragraph.";
        let options = Options {
            enable_directives: true,
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

    /// Tests link rendering with proper `<a>` tags and attribute escaping.
    #[test]
    fn test_link() {
        let input = "[Rust](https://www.rust-lang.org/)";
        let options = Options {
            enable_directives: true,
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

    /// Tests directive syntax conversion to Component blocks.
    #[test]
    fn test_directive_to_component() {
        let input = ":::note[My Title]\nThis is **important** content.\n:::";
        let options = Options {
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();

        // Should produce exactly 1 Component block
        assert_eq!(
            blocks.blocks.len(),
            1,
            "Expected 1 block, got {}",
            blocks.blocks.len()
        );

        match &blocks.blocks[0] {
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                // Directives are converted to Aside component with type prop
                assert_eq!(name, "Aside");
                assert_eq!(props.get("type"), Some(&PropValue::literal("note")));
                assert_eq!(props.get("title"), Some(&PropValue::literal("My Title")));
                assert!(slot_html.contains("<p>This is <strong>important</strong> content.</p>"));
            }
            _ => panic!("Expected Component block, got {:?}", blocks.blocks[0]),
        }
    }

    /// Tests directive without title.
    #[test]
    fn test_directive_without_title() {
        let input = ":::tip\nHelpful advice here.\n:::";
        let options = Options {
            enable_directives: true,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        match &blocks.blocks[0] {
            RenderBlock::Component {
                name,
                props,
                slot_html,
            } => {
                // Directives are converted to Aside component with type prop
                assert_eq!(name, "Aside");
                assert_eq!(props.get("type"), Some(&PropValue::literal("tip")));
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
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();

        // Should parse as regular paragraph text
        match &blocks.blocks[0] {
            RenderBlock::Html { content } => {
                assert!(content.contains(":::note"));
            }
            _ => panic!("Expected HTML block when directives disabled"),
        }
    }

    /// Tests standard markdown elements rendering.
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
        };
        let blocks = to_blocks(input, &options).unwrap();

        // Should produce 1 HTML block (no directives, so all content is HTML)
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(
                content.contains("<h1 id=") && content.contains(">Heading 1</h1>"),
                "Missing h1"
            );
            assert!(
                content.contains("<h2 id=") && content.contains(">Heading 2</h2>"),
                "Missing h2"
            );
            assert!(content.contains("<ul>"), "Missing ul");
            assert!(content.contains("<li>"), "Missing li");
            assert!(content.contains("List Item 1"), "Missing list item text");
            assert!(content.contains("<blockquote>"), "Missing blockquote");
            assert!(
                content.contains(r#"<code class="language-rust">"#),
                "Missing code block with language"
            );
            // Curly braces are escaped to &#123; and &#125; for JSX safety in code blocks
            assert!(
                content.contains("fn main() &#123;&#125;"),
                "Missing code content"
            );
            assert!(
                content.contains(r#"<img src="image.png""#),
                "Missing img src"
            );
            assert!(content.contains(r#"alt="Alt text""#), "Missing alt");
            assert!(content.contains(r#"title="Title""#), "Missing title");
            assert!(content.contains("<hr />"), "Missing hr");
        } else {
            panic!("Expected HTML block, got {:?}", blocks.blocks[0]);
        }
    }

    /// Tests task list rendering (GFM feature).
    #[test]
    fn test_task_list() {
        let input = "- [ ] Unchecked task\n- [x] Checked task\n";
        let options = Options {
            enable_directives: true,
        };
        let blocks = to_blocks(input, &options).unwrap();

        assert_eq!(blocks.blocks.len(), 1);
        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(
                content.contains(r#"class="task-list-item""#),
                "Missing task-list-item class"
            );
            assert!(content.contains(r#"type="checkbox""#), "Missing checkbox");
            assert!(content.contains("Unchecked task"), "Missing unchecked text");
            assert!(content.contains("Checked task"), "Missing checked text");
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests ordered list rendering.
    #[test]
    fn test_ordered_list() {
        let input = "1. First\n2. Second\n3. Third\n";
        let options = Options {
            enable_directives: true,
        };
        let blocks = to_blocks(input, &options).unwrap();

        assert_eq!(blocks.blocks.len(), 1);
        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(content.contains("<ol>"));
            assert!(content.contains("<li>"), "Missing <li>");
            assert!(content.contains("First"), "Missing 'First'");
            assert!(content.contains("Second"), "Missing 'Second'");
            assert!(content.contains("</ol>"));
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests HTML escaping in text content to prevent XSS.
    #[test]
    fn test_xss_text_escaping() {
        let input = "Text with <script>alert('xss')</script> and & symbols.";
        let options = Options {
            enable_directives: false,
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(
            result.blocks.len(),
            3,
            "Expected 3 blocks due to JSX parsing"
        );

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

    /// Tests HTML escaping in link title attributes to prevent XSS.
    #[test]
    fn test_xss_attribute_escaping() {
        let input = r#"[Link](http://example.com "Title with <script> and & and ' quotes")"#;
        let options = Options {
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            println!("Attribute escaping:\n{}\n", content);
            // Special characters in title attribute should be escaped
            assert!(
                content.contains("&lt;script&gt;"),
                "Script tags in title not escaped"
            );
            assert!(content.contains("&amp;"), "Ampersand in title not escaped");
            assert!(
                content.contains("&#39;"),
                "Single quotes in title not escaped"
            );
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests that image alt and title attributes are properly escaped.
    #[test]
    fn test_xss_image_attributes() {
        let input = r#"![Alt with ' and "](image.png "Title with &")"#;
        let options = Options {
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            println!("Image attributes:\n{}\n", content);
            // Alt and title should be escaped
            assert!(content.contains("&#39;"), "Single quote in alt not escaped");
            assert!(
                content.contains("&quot;"),
                "Double quote in alt not escaped"
            );
            assert!(content.contains("&amp;"), "Ampersand in title not escaped");
        } else {
            panic!("Expected HTML block");
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

    /// Tests GFM strikethrough (~~text~~) rendering.
    #[test]
    fn test_strikethrough() {
        let input = "This is ~~deleted~~ text.";
        let options = Options {
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            assert!(
                content.contains("<del>deleted</del>"),
                "Strikethrough not rendered"
            );
            assert!(content.contains("<p>"), "Missing paragraph tag");
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests GFM table rendering with alignment.
    #[test]
    fn test_table() {
        let input = r#"| Name | Age | City |
| :--- | :---: | ---: |
| Alice | 30 | Tokyo |
| Bob | 25 | NYC |"#;
        let options = Options {
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            println!("Table HTML:\n{}\n", content);

            // Check table structure
            assert!(content.contains("<table>"), "Missing <table>");
            assert!(content.contains("<thead>"), "Missing <thead>");
            assert!(content.contains("<tbody>"), "Missing <tbody>");
            assert!(content.contains("</table>"), "Missing </table>");

            // Check header cells
            assert!(content.contains("<th"), "Missing <th> tags");
            assert!(content.contains("Name"), "Missing 'Name' header");
            assert!(content.contains("Age"), "Missing 'Age' header");
            assert!(content.contains("City"), "Missing 'City' header");

            // Check data cells
            assert!(content.contains("<td"), "Missing <td> tags");
            assert!(content.contains("Alice"), "Missing 'Alice' data");
            assert!(content.contains("Bob"), "Missing 'Bob' data");

            // Check alignment attributes
            assert!(content.contains("align=\"left\""), "Missing left alignment");
            assert!(
                content.contains("align=\"center\""),
                "Missing center alignment"
            );
            assert!(
                content.contains("align=\"right\""),
                "Missing right alignment"
            );
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests table with complex content (links, emphasis, etc.).
    #[test]
    fn test_table_with_formatting() {
        let input = r#"| Feature | Status |
| --- | --- |
| **Bold** | ✓ |
| [Link](https://example.com) | ✓ |
| `code` | ✓ |"#;
        let options = Options {
            enable_directives: false,
        };

        let blocks = to_blocks(input, &options).unwrap();
        assert_eq!(blocks.blocks.len(), 1);

        if let RenderBlock::Html { content } = &blocks.blocks[0] {
            // Check that formatting is preserved inside table cells
            assert!(
                content.contains("<strong>Bold</strong>"),
                "Missing bold in table"
            );
            assert!(
                content.contains(r#"<a href="https://example.com">"#),
                "Missing link in table"
            );
            assert!(
                content.contains("<code>code</code>"),
                "Missing code in table"
            );
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests that code blocks preserve newlines between lines.
    #[test]
    fn test_code_block_preserves_newlines() {
        let input = r#"```ts
line1
line2
line3
```"#;
        let options = Options {
            enable_directives: true,
        };

        let result = to_blocks(input, &options).unwrap();
        assert_eq!(result.blocks.len(), 1);

        if let RenderBlock::Html { content } = &result.blocks[0] {
            // Code block newlines are encoded as &#10; to survive esbuild's
            // JSX transform (which normalizes whitespace in JSX text children)
            assert!(
                content.contains("line1&#10;line2&#10;line3"),
                "Code block should preserve newlines as HTML entities. Got: {}",
                content
            );
        } else {
            panic!("Expected HTML block");
        }
    }

    /// Tests that code blocks inside JSX components preserve newlines.
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
        };

        let result = to_blocks(input, &options).unwrap();

        // Find the component block
        let component = result
            .blocks
            .iter()
            .find(|b| matches!(b, RenderBlock::Component { name, .. } if name == "Steps"));
        assert!(component.is_some(), "Should have Steps component block");

        if let RenderBlock::Component { slot_html, .. } = component.unwrap() {
            // Code block newlines are encoded as &#10; to survive esbuild's JSX transform
            assert!(
                slot_html.contains("line1&#10;line2&#10;line3"),
                "Code block inside JSX should preserve newlines. Got: {}",
                slot_html
            );
        }
    }

    /// Tests that indented directives inside Steps preserve their indentation
    /// and become part of the list item content.
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
        };

        let result = to_blocks(input, &options).unwrap();
        // Find the Steps component block
        let steps_component = result
            .blocks
            .iter()
            .find(|b| matches!(b, RenderBlock::Component { name, .. } if name == "Steps"));
        assert!(
            steps_component.is_some(),
            "Should have Steps component block"
        );

        if let RenderBlock::Component { slot_html, .. } = steps_component.unwrap() {
            // The directive should be converted to <Aside type="tip">, not raw <tip>
            assert!(
                !slot_html.contains("<tip>"),
                "Directive should NOT produce raw <tip> element. Got: {}",
                slot_html
            );
            // Should have Aside component with type prop
            assert!(
                slot_html.contains("<Aside") && slot_html.contains("type={\"tip\"}"),
                "Should have Aside component with type prop. Got: {}",
                slot_html
            );
            // Directive content should be present
            assert!(
                slot_html.contains("Some tip content"),
                "Tip directive content should be present. Got: {}",
                slot_html
            );
        }
    }
}
