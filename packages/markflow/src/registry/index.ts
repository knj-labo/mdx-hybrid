import type { ComponentLibrary, ComponentDefinition, DirectiveMapping } from './types';

export class Registry {
  private components: Map<string, ComponentDefinition> = new Map();
  private directives: Map<string, DirectiveMapping> = new Map();

  constructor(libraries: ComponentLibrary[]) {
    for (const lib of libraries) {
      for (const comp of lib.components) {
        this.components.set(comp.name, comp);
      }
      for (const dir of lib.directiveMappings ?? []) {
        this.directives.set(dir.directive, dir);
      }
    }
  }

  getComponent(name: string): ComponentDefinition | undefined {
    return this.components.get(name);
  }

  getDirectiveMapping(directive: string): DirectiveMapping | undefined {
    return this.directives.get(directive);
  }

  getAllComponents(): ComponentDefinition[] {
    return Array.from(this.components.values());
  }

  getSupportedDirectives(): string[] {
    return Array.from(this.directives.keys());
  }

  toRustConfig() {
    return {
      components: this.getAllComponents(),
      directiveMappings: Array.from(this.directives.values()),
    };
  }
}

export function createRegistry(libraries: ComponentLibrary[]): Registry {
  return new Registry(libraries);
}

export * from './types';
export { starlightLibrary } from './presets/starlight';
export { astroLibrary } from './presets/astro';
