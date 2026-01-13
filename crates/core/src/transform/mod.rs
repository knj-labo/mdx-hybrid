//! Transform stage utilities.
//!
//! - `ast_components`: AST-level component transformers (Tabs, Steps, etc.) applied before rendering.
//! - `hoist_adapter`: lifts top-level ESM import/export statements from the event stream.
//! - `directive_adapter`: rewrites `:::` directives into configured block tags during streaming.
//! - `code_fence`: tracks fenced blocks to avoid hoisting/rewrites inside them.

/// AST-level component transformers for Starlight compatibility.
pub mod ast_components;
/// Code fence state tracking utilities.
pub mod code_fence;
/// Astro docs component rewrite helpers (lol_html-based, for streaming).
pub mod components;
/// Directive-to-Aside (or custom) rewriting adapter.
pub mod directive_adapter;
/// Directive mapping traits and default implementations.
pub mod directives;
/// ESM import/export hoisting adapter.
pub mod hoist_adapter;
/// Smart punctuation transformations (quotes, dashes, ellipsis).
pub mod smartypants;

pub use ast_components::apply_component_transformers;
pub use directive_adapter::DirectiveAdapter;
pub use hoist_adapter::HoistAdapter;
