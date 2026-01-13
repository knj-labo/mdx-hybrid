/** @type {typeof import('markflow-wasm') | null} */
let wasmModule = null;

/** @type {boolean} */
let initialized = false;

async function loadWasm() {
  if (!wasmModule) {
    wasmModule = await import('markflow-wasm');
  }
  if (!initialized) {
    // Initialize WASM module
    await wasmModule.default();
    initialized = true;
  }
  return wasmModule;
}

/**
 * Compile MDX source to Astro-compatible module.
 * @param {string} source - MDX source code
 * @param {import('./index').CompileOptions} [options] - Compile options
 * @returns {Promise<import('./index').CompileResult>}
 */
export async function compile(source, options = {}) {
  const wasm = await loadWasm();
  const result = wasm.compile(source, options.filepath ?? 'input.mdx');

  return {
    code: result.code,
    frontmatter: JSON.parse(result.frontmatter_json),
    headings: result.headings,
    hasUserDefaultExport: result.has_user_default_export,
  };
}
