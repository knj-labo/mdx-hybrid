//! Streaming transform stage utilities (event adapters).
//!
//! - `hoist_adapter`: lifts top-level ESM import/export statements from the event stream.
//! - `directive_adapter`: rewrites `:::` directives into configured block tags during streaming.
//! - `code_fence`: tracks fenced blocks to avoid hoisting/rewrites inside them.

/// Code fence state tracking utilities.
pub mod code_fence;
/// Directive-to-Aside (or custom) rewriting adapter.
pub mod directive_adapter;
/// Directive mapping traits and default implementations.
pub mod directives;
/// ESM import/export hoisting adapter.
pub mod hoist_adapter;
/// Smart punctuation transformations (quotes, dashes, ellipsis).
pub mod smartypants;
/// Astro docs component rewrite helpers.
pub mod components;

pub use directive_adapter::DirectiveAdapter;
pub use hoist_adapter::HoistAdapter;
