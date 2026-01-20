#![deny(missing_docs)]
//! Markflow core: MDAST-based markdown rendering with code generation utilities.

/// Code generation utilities for WASM and NAPI bindings.
pub mod codegen;
/// Error types and utilities.
pub mod error;
/// YAML frontmatter extraction helpers.
pub mod frontmatter;
/// Parsing layer (markdown-rs adapter + config).
pub mod parser;
/// Component registry for managing component and directive mappings.
pub mod registry;
/// Rendering layer (MDAST-based block renderer).
pub mod renderer;
/// Slug generation utilities.
pub mod slug;
/// Transform utilities (code fences, directives, smartypants).
pub mod transform;

pub use error::MarkflowError;
pub use frontmatter::{FrontmatterError, FrontmatterExtraction, extract_frontmatter};
pub use parser::{ParseConfig, ParseConstructs};
#[allow(deprecated)]
pub use registry::{
    ComponentDef, ComponentDefinition, DirectiveMapping, PropSource, RegistryConfig,
    SlotNormalization,
};
pub use renderer::{BlocksResult, HeadingEntry, MdastOptions, PropValue, RenderBlock, to_blocks};
pub use slug::{Slugger, slugify};
pub use transform::{code_fence, directives};
