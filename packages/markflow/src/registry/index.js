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
    getComponent: (name) => components.get(name),
    getDirectiveMapping: (directive) => directives.get(directive),
    getAllComponents: () => Array.from(components.values()),
    getSupportedDirectives: () => Array.from(directives.keys()),
    toRustConfig: () => ({
      components: Array.from(components.values()),
      directiveMappings: Array.from(directives.values()),
    }),
  };
}

export { starlightLibrary } from './presets/starlight.js';
export { astroLibrary } from './presets/astro.js';
