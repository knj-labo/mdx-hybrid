import type { ComponentLibrary, ComponentDefinition, DirectiveMapping } from './types';

export interface Registry {
  getComponent(name: string): ComponentDefinition | undefined;
  getDirectiveMapping(directive: string): DirectiveMapping | undefined;
  getAllComponents(): ComponentDefinition[];
  getSupportedDirectives(): string[];
  toRustConfig(): { components: ComponentDefinition[]; directiveMappings: DirectiveMapping[] };
}

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
    getComponent: (name: string) => components.get(name),
    getDirectiveMapping: (directive: string) => directives.get(directive),
    getAllComponents: () => Array.from(components.values()),
    getSupportedDirectives: () => Array.from(directives.keys()),
    toRustConfig: () => ({
      components: Array.from(components.values()),
      directiveMappings: Array.from(directives.values()),
    }),
  };
}

export * from './types';
export { starlightLibrary } from './presets/starlight';
export { astroLibrary } from './presets/astro';
