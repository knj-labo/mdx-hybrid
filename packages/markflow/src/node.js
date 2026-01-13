import { createCompiler } from 'markflow-napi';

/** @type {import('markflow-napi').MarkflowCompiler | null} */
let compiler = null;

/**
 * Compile MDX source to Astro-compatible module.
 * @param {string} source - MDX source code
 * @param {import('./index').CompileOptions} [options] - Compile options
 * @returns {Promise<import('./index').CompileResult>}
 */
export async function compile(source, options = {}) {
  if (!compiler) {
    compiler = createCompiler({});
  }

  const result = compiler.compile(source, options.filepath ?? 'input.mdx', {
    file: options.filepath,
    url: options.url,
  });

  return {
    code: result.code,
    frontmatter: JSON.parse(result.frontmatterJson),
    headings: result.headings,
    hasUserDefaultExport: false, // TODO: expose from NAPI
  };
}
