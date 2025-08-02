import { compile } from '@mdx-hybrid/core'
import type { CompileOptions } from '@mdx-hybrid/core'
import type { AstroIntegration } from 'astro'
import { VFile } from 'vfile'

export interface AstroMDXHybridOptions extends Omit<CompileOptions, 'development'> {
  /**
   * Include files to process
   * @default /\.mdx$/
   */
  include?: RegExp | RegExp[]

  /**
   * Exclude files from processing
   */
  exclude?: RegExp | RegExp[]
}

export function mdxHybrid(options: AstroMDXHybridOptions = {}): AstroIntegration {
  const { include = /\.mdx$/, exclude, ...compileOptions } = options

  return {
    name: '@mdx-hybrid/astro',
    hooks: {
      'astro:config:setup': ({ config, command, addPageExtension, updateConfig }) => {
        // Add .mdx as a valid page extension
        addPageExtension('.mdx')

        // Add vite plugin for MDX processing
        updateConfig({
          vite: {
            plugins: [
              {
                name: 'mdx-hybrid:astro',
                enforce: 'pre',

                async transform(code: string, id: string) {
                  // Check if file should be processed
                  if (!shouldProcess(id, include, exclude)) {
                    return null
                  }

                  try {
                    // Compile MDX with mdx-hybrid
                    const compiled = await compile(code, {
                      ...compileOptions,
                      development: command === 'dev',
                      jsx: true,
                      jsxRuntime: 'automatic',
                      jsxImportSource: 'astro',
                      // Astro uses its own provider for components
                      providerImportSource: '@mdx-js/react',
                      // Output format for Astro
                      outputFormat: 'function-body',
                    })

                    // Create Astro-compatible module
                    const moduleCode = createAstroModule(compiled.code, id)

                    return {
                      code: moduleCode,
                      map: compiled.map,
                    }
                  } catch (error) {
                    const message = error instanceof Error ? error.message : String(error)
                    this.error(`MDX compilation failed for ${id}: ${message}`)
                  }
                },

                configureServer(server) {
                  // Watch MDX files for changes
                  server.watcher.add('**/*.mdx')
                },
              },
            ],
          },
        })
      },
    },
  }
}

function shouldProcess(
  id: string,
  include: RegExp | RegExp[],
  exclude?: RegExp | RegExp[]
): boolean {
  // Normalize to arrays
  const includePatterns = Array.isArray(include) ? include : [include]
  const excludePatterns = exclude ? (Array.isArray(exclude) ? exclude : [exclude]) : []

  // Check excludes first
  if (excludePatterns.some((pattern) => pattern.test(id))) {
    return false
  }

  // Check includes
  return includePatterns.some((pattern) => pattern.test(id))
}

function createAstroModule(compiledCode: string, filePath: string): string {
  return `
import { Fragment, jsx as h } from 'astro/jsx-runtime';
import { createComponent, renderComponent, render } from 'astro/runtime/server/index.js';

${compiledCode}

const Content = createComponent(async (result, props, slots) => {
  const content = await render\`\${renderComponent(result, 'MDXContent', MDXContent, props, slots)}\`;
  return content;
});

export default Content;
export { MDXContent };

// Export frontmatter if available
export const frontmatter = typeof _frontmatter !== 'undefined' ? _frontmatter : {};

// Export metadata for Astro
export const url = typeof _url !== 'undefined' ? _url : undefined;
export const file = ${JSON.stringify(filePath)};
`
}

// Re-export types
export type { CompileOptions } from '@mdx-hybrid/core'
