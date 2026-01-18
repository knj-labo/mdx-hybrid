/**
 * Registry type definitions
 * @module registry/types
 */

/**
 * Defines a component that can be injected into MDX content.
 */
export interface ComponentDefinition {
  /** Component name (e.g., "Aside", "Code") */
  name: string;
  /** Module path to import from (e.g., "@astrojs/starlight/components") */
  modulePath: string;
  /** Export type: "named" for named exports, "default" for default exports */
  exportType: 'named' | 'default';
}

/**
 * Prop injection source specification.
 */
export interface PropSource {
  /** Source of the prop value */
  source: 'directive_name' | 'bracket_title' | 'literal';
  /** Literal value when source is 'literal' */
  value?: string;
}

/**
 * Maps a directive (e.g., :::note) to a component.
 */
export interface DirectiveMapping {
  /** Directive name (e.g., "note", "tip") */
  directive: string;
  /** Target component name (e.g., "Aside") */
  component: string;
  /** Props to inject into the component */
  injectProps?: Record<string, PropSource>;
}

/**
 * A component library preset containing components and directive mappings.
 */
export interface ComponentLibrary {
  /** Unique identifier for the library */
  id: string;
  /** Human-readable name */
  name: string;
  /** Default module path for components */
  defaultModulePath: string;
  /** Components provided by this library */
  components: ComponentDefinition[];
  /** Directive mappings for this library */
  directiveMappings?: DirectiveMapping[];
}

/**
 * Runtime registry for component lookup and resolution.
 */
export interface Registry {
  /** Get a component definition by name */
  getComponent(name: string): ComponentDefinition | undefined;
  /** Get a directive mapping by directive name */
  getDirectiveMapping(directive: string): DirectiveMapping | undefined;
  /** Get all registered components */
  getAllComponents(): ComponentDefinition[];
  /** Get all supported directive names */
  getSupportedDirectives(): string[];
  /** Get all components that belong to a specific module */
  getComponentsByModule(modulePath: string): ComponentDefinition[];
  /** Check if a component exists in the registry */
  hasComponent(name: string): boolean;
  /** Get the import path for a component */
  getImportPath(name: string): string | undefined;
  /** Convert registry to Rust-compatible configuration format */
  toRustConfig(): { components: ComponentDefinition[]; directiveMappings: DirectiveMapping[] };
}

/**
 * Error found during validation.
 */
export interface ValidationError {
  /** Error type: 'component' or 'directive' */
  type: 'component' | 'directive';
  /** Name of the invalid item */
  name: string;
  /** Human-readable error message */
  message: string;
}

/**
 * Result of validation.
 */
export interface ValidationResult {
  /** Whether the registry/library is valid */
  valid: boolean;
  /** List of validation errors */
  errors: ValidationError[];
}
