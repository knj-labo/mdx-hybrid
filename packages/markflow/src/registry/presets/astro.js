export const astroLibrary = {
  id: 'astro',
  name: 'Astro Core',
  defaultModulePath: 'astro/components',
  components: [
    { name: 'Code', modulePath: 'astro/components', exportType: 'named' },
    { name: 'Prism', modulePath: 'astro/components', exportType: 'named' },
  ],
  directiveMappings: [],
};
