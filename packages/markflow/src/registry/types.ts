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
