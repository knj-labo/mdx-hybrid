/**
 * Creates a component registry from one or more library presets.
 * The registry provides lookup and utility methods for component resolution.
 *
 * @param {Array<import('./types.js').ComponentLibrary>} libraries - Array of library presets
 * @returns {import('./types.js').ComponentRegistry} Registry instance
 *
 * @example
 * import { createRegistry, starlightLibrary, astroLibrary } from 'markflow/registry';
 * const registry = createRegistry([starlightLibrary, astroLibrary]);
 * const aside = registry.getComponent('Aside');
 */
export function createRegistry(libraries) {
  const components = new Map();
  const directives = new Map();

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
     * @param {string} name - Component name
     * @returns {import('./types.js').ComponentDefinition|undefined}
     */
    getComponent: (name) => components.get(name),

    /**
     * Get a directive mapping by directive name.
     * @param {string} directive - Directive name (e.g., 'note', 'tip')
     * @returns {import('./types.js').DirectiveMapping|undefined}
     */
    getDirectiveMapping: (directive) => directives.get(directive),

    /**
     * Get all registered components.
     * @returns {Array<import('./types.js').ComponentDefinition>}
     */
    getAllComponents: () => Array.from(components.values()),

    /**
     * Get all supported directive names.
     * @returns {string[]}
     */
    getSupportedDirectives: () => Array.from(directives.keys()),

    /**
     * Get all components that belong to a specific module.
     * @param {string} modulePath - Module path to filter by
     * @returns {Array<import('./types.js').ComponentDefinition>}
     *
     * @example
     * const starlightComps = registry.getComponentsByModule('@astrojs/starlight/components');
     */
    getComponentsByModule: (modulePath) =>
      Array.from(components.values()).filter((c) => c.modulePath === modulePath),

    /**
     * Check if a component exists in the registry.
     * @param {string} name - Component name
     * @returns {boolean}
     */
    hasComponent: (name) => components.has(name),

    /**
     * Get the full import path for a component.
     * For default exports, returns path with component file (e.g., 'module/Component.astro').
     * For named exports, returns the module path.
     * @param {string} name - Component name
     * @returns {string|undefined}
     *
     * @example
     * registry.getImportPath('Aside') // '@astrojs/starlight/components'
     */
    getImportPath: (name) => {
      const comp = components.get(name);
      if (!comp) return undefined;
      return comp.modulePath;
    },

    /**
     * Convert registry to Rust-compatible configuration format.
     * @returns {{ components: Array, directiveMappings: Array }}
     */
    toRustConfig: () => ({
      components: Array.from(components.values()),
      directiveMappings: Array.from(directives.values()),
    }),
  };
}

export { starlightLibrary } from './presets/starlight.js';
export { astroLibrary } from './presets/astro.js';
export { expressiveCodeLibrary } from './presets/expressive-code.js';
export { validateRegistry, validateLibrary } from './validation.js';
