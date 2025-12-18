use std::borrow::Cow;
use std::convert::TryFrom;

/// A Markdown event emitted by the Markflow pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// Start of a tagged element.
    Start(Tag<'a>),
    /// End of a tagged element.
    End(TagEnd),
    /// Text node.
    Text(Cow<'a, str>),
    /// Inline code span.
    Code(Cow<'a, str>),
    /// HTML chunk (block-level).
    Html(Cow<'a, str>),
    /// Inline HTML snippet.
    InlineHtml(Cow<'a, str>),
    /// Inline JSX snippet (preserved as-is).
    JsxInline(Cow<'a, str>),
    /// Flow JSX block (preserved as-is).
    JsxFlow(Cow<'a, str>),
    /// Inline math.
    InlineMath(Cow<'a, str>),
    /// Display math.
    DisplayMath(Cow<'a, str>),
    /// Footnote reference.
    FootnoteReference(Cow<'a, str>),
    /// Task list checkbox marker.
    TaskListMarker(bool),
    /// Horizontal rule.
    Rule,
    /// Hard line break.
    HardBreak,
    /// Soft line break.
    SoftBreak,
}

/// Tags for container elements.
#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    Paragraph,
    Heading {
        level: HeadingLevel,
        id: Option<Cow<'a, str>>,
        classes: Vec<Cow<'a, str>>,
        attrs: Vec<(Cow<'a, str>, Option<Cow<'a, str>>)>,
    },
    BlockQuote,
    CodeBlock(CodeBlockKind<'a>),
    List(Option<u64>),
    Item,
    FootnoteDefinition(Cow<'a, str>),
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link {
        link_type: LinkType,
        dest_url: Cow<'a, str>,
        title: Cow<'a, str>,
        id: Cow<'a, str>,
    },
    Image {
        link_type: LinkType,
        dest_url: Cow<'a, str>,
        title: Cow<'a, str>,
        id: Cow<'a, str>,
    },
}

/// Tag terminators.
#[derive(Debug, Clone, PartialEq)]
pub enum TagEnd {
    Paragraph,
    Heading(HeadingLevel),
    BlockQuote,
    CodeBlock,
    List(bool),
    Item,
    FootnoteDefinition,
    Table,
    TableHead,
    TableRow,
    TableCell,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    Image,
}

/// Heading depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeadingLevel {
    H1 = 1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

/// Code block metadata.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeBlockKind<'a> {
    Indented,
    Fenced(Cow<'a, str>),
}

/// Table alignment metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

/// Link kinds used throughout the pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinkType {
    Inline,
    Reference,
    ReferenceUnknown,
    Collapsed,
    CollapsedUnknown,
    Shortcut,
    ShortcutUnknown,
    Autolink,
    Email,
}

impl<'a> Tag<'a> {
    /// Converts a tag into its closing counterpart.
    pub fn to_end(&self) -> TagEnd {
        match self {
            Tag::Paragraph => TagEnd::Paragraph,
            Tag::Heading { level, .. } => TagEnd::Heading(*level),
            Tag::BlockQuote => TagEnd::BlockQuote,
            Tag::CodeBlock(_) => TagEnd::CodeBlock,
            Tag::List(start) => TagEnd::List(start.is_some()),
            Tag::Item => TagEnd::Item,
            Tag::FootnoteDefinition(_) => TagEnd::FootnoteDefinition,
            Tag::Table(_) => TagEnd::Table,
            Tag::TableHead => TagEnd::TableHead,
            Tag::TableRow => TagEnd::TableRow,
            Tag::TableCell => TagEnd::TableCell,
            Tag::Emphasis => TagEnd::Emphasis,
            Tag::Strong => TagEnd::Strong,
            Tag::Strikethrough => TagEnd::Strikethrough,
            Tag::Link { .. } => TagEnd::Link,
            Tag::Image { .. } => TagEnd::Image,
        }
    }
}

impl TryFrom<usize> for HeadingLevel {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(HeadingLevel::H1),
            2 => Ok(HeadingLevel::H2),
            3 => Ok(HeadingLevel::H3),
            4 => Ok(HeadingLevel::H4),
            5 => Ok(HeadingLevel::H5),
            6 => Ok(HeadingLevel::H6),
            _ => Err(()),
        }
    }
}
