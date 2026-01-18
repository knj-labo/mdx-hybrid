/**
 * Component registry for Markflow.
 * @module registry
 */

import type {
  ComponentDefinition,
  DirectiveMapping,
  ComponentLibrary,
  Registry,
} from './types.js';

/**
 * Creates a component registry from one or more library presets.
 * The registry provides lookup and utility methods for component resolution.
 *
 * @param libraries - Array of library presets
 * @returns Registry instance
 *
 * @example
 * import { createRegistry, starlightLibrary, astroLibrary } from 'markflow/registry';
 * const registry = createRegistry([starlightLibrary, astroLibrary]);
 * const aside = registry.getComponent('Aside');
 */
export function createRegistry(libraries: ComponentLibrary[]): Registry {
  const components = new Map<string, ComponentDefinition>();
  const directives = new Map<string, DirectiveMapping>();

  for (const lib of libraries) {
    for (const comp of lib.components) {
      components.set(comp.name, comp);
    }
    for (const dir of lib.directiveMappings ?? []) {
      directives.set(dir.directive, dir);
    }
  }

  return {
    /**
     * Get a component definition by name.
     */
    getComponent: (name: string): ComponentDefinition | undefined => components.get(name),

    /**
     * Get a directive mapping by directive name.
     */
    getDirectiveMapping: (directive: string): DirectiveMapping | undefined => directives.get(directive),

    /**
     * Get all registered components.
     */
    getAllComponents: (): ComponentDefinition[] => Array.from(components.values()),

    /**
     * Get all supported directive names.
     */
    getSupportedDirectives: (): string[] => Array.from(directives.keys()),

    /**
     * Get all components that belong to a specific module.
     */
    getComponentsByModule: (modulePath: string): ComponentDefinition[] =>
      Array.from(components.values()).filter((c) => c.modulePath === modulePath),

    /**
     * Check if a component exists in the registry.
     */
    hasComponent: (name: string): boolean => components.has(name),

    /**
     * Get the full import path for a component.
     */
    getImportPath: (name: string): string | undefined => {
      const comp = components.get(name);
      if (!comp) return undefined;
      return comp.modulePath;
    },

    /**
     * Convert registry to Rust-compatible configuration format.
     */
    toRustConfig: () => ({
      components: Array.from(components.values()),
      directiveMappings: Array.from(directives.values()),
    }),
  };
}

// Re-export types
export type {
  ComponentDefinition,
  DirectiveMapping,
  ComponentLibrary,
  Registry,
  ValidationError,
  ValidationResult,
} from './types.js';

// Re-export library presets
export { starlightLibrary } from './presets/starlight.js';
export { astroLibrary } from './presets/astro.js';
export { expressiveCodeLibrary } from './presets/expressive-code.js';

// Re-export validation utilities
export { validateRegistry, validateLibrary } from './validation.js';
