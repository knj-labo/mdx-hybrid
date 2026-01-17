//! Registry type definitions for component and directive mappings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the component registry passed from JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryConfig {
    /// Available components.
    pub components: Vec<ComponentDef>,
    /// Directive to component mappings.
    pub directive_mappings: Vec<DirectiveMapping>,
}

/// A single component definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDef {
    /// Component name (e.g., "Aside", "Tabs").
    pub name: String,
    /// Module path for import (e.g., "@astrojs/starlight/components").
    pub module_path: String,
    /// Export type: "named" or "default".
    pub export_type: String,
}

/// Mapping from a directive name to a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveMapping {
    /// Directive name (e.g., "note", "tip").
    pub directive: String,
    /// Target component name (e.g., "Aside").
    pub component: String,
    /// Optional props to inject when mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_props: Option<HashMap<String, PropSource>>,
}

/// Source for an injected prop value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropSource {
    /// Source type: "directive_name", "bracket_title", or "literal".
    pub source: String,
    /// Literal value when source is "literal".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl RegistryConfig {
    /// Check if a directive name is supported.
    pub fn is_supported_directive(&self, name: &str) -> bool {
        self.directive_mappings.iter().any(|m| m.directive == name)
    }

    /// Get the component name for a directive.
    pub fn get_directive_component(&self, directive: &str) -> Option<&str> {
        self.directive_mappings
            .iter()
            .find(|m| m.directive == directive)
            .map(|m| m.component.as_str())
    }

    /// Get the module path for a component.
    pub fn get_component_module(&self, name: &str) -> Option<&str> {
        self.components
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.module_path.as_str())
    }

    /// Get the full directive mapping for a directive name.
    pub fn get_directive_mapping(&self, directive: &str) -> Option<&DirectiveMapping> {
        self.directive_mappings
            .iter()
            .find(|m| m.directive == directive)
    }
}
