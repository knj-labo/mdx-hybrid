/**
 * ExpressiveCode library preset for component registry.
 * @module registry/presets/expressive-code
 */

export const expressiveCodeLibrary = {
  id: 'expressive-code',
  name: 'Astro ExpressiveCode',
  defaultModulePath: 'astro-expressive-code/components',
  components: [
    { name: 'Code', modulePath: 'astro-expressive-code/components', exportType: 'named' },
  ],
  directiveMappings: [],
};
