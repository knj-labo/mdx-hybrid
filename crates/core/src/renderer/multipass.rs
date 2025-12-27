#![allow(missing_docs)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<'a> {
    Markdown(&'a str),
    Code(&'a str),
    JsxElement {
        name: &'a str,
        attrs: &'a str,
        children: Vec<Block<'a>>,
        is_self_closing: bool,
    },
}

pub fn scan(input: &str) -> Vec<Block<'_>> {
    if input.is_empty() {
        return Vec::new();
    }

    vec![Block::Markdown(input)]
}

#[cfg(test)]
mod tests {
    use super::{Block, scan};

    #[test]
    fn scan_returns_single_markdown_block() {
        let input = "hello";
        let blocks = scan(input);

        assert_eq!(blocks, vec![Block::Markdown(input)]);
    }

    #[test]
    fn scan_empty_returns_empty() {
        let blocks = scan("");

        assert!(blocks.is_empty());
    }
}
