/// Byte offset range within the (normalized) source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a new span from byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}
