import { markflowPlugin } from 'vite-plugin-markflow';

/**
 * Astro integration for Markflow.
 * @param {import('./index').MarkflowOptions} [options]
 * @returns {import('astro').AstroIntegration}
 */
export default function markflow(options = {}) {
  return {
    name: 'astro-markflow',
    hooks: {
      'astro:config:setup': ({ updateConfig }) => {
        updateConfig({
          vite: {
            plugins: [markflowPlugin(options)],
          },
        });
      },
    },
  };
}
