export interface ComponentDefinition {
  name: string;
  modulePath: string;
  exportType: 'named' | 'default';
}

export interface DirectiveMapping {
  directive: string;
  component: string;
  injectProps?: Record<string, { source: 'directive_name' | 'bracket_title' | 'literal', value?: string }>;
}

export interface ComponentLibrary {
  id: string;
  name: string;
  defaultModulePath: string;
  components: ComponentDefinition[];
  directiveMappings?: DirectiveMapping[];
}

export interface ComponentRegistry {
  libraries: ComponentLibrary[];
}

export interface Registry {
  getComponent(name: string): ComponentDefinition | undefined;
  getDirectiveMapping(directive: string): DirectiveMapping | undefined;
  getAllComponents(): ComponentDefinition[];
  getSupportedDirectives(): string[];
  toRustConfig(): { components: ComponentDefinition[]; directiveMappings: DirectiveMapping[] };
}

export function createRegistry(libraries: ComponentLibrary[]): Registry;

export const starlightLibrary: ComponentLibrary;
export const astroLibrary: ComponentLibrary;
