//! Streaming transform stage utilities (event adapters).
//!
//! - `hoist_adapter`: lifts top-level ESM import/export statements from the event stream.
//! - `directive_adapter`: rewrites `:::` directives into `<Aside>` blocks during streaming.

/// Directive-to-Aside rewriting adapter.
pub mod directive_adapter;
/// ESM import/export hoisting adapter.
pub mod hoist_adapter;

pub use directive_adapter::DirectiveAdapter;
pub use hoist_adapter::HoistAdapter;
