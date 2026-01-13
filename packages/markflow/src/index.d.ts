/**
 * Heading metadata extracted from the document.
 */
export interface HeadingEntry {
  /** Heading depth (1-6). */
  depth: number;
  /** Slugified identifier. */
  slug: string;
  /** Visible heading text. */
  text: string;
}

/**
 * Options for the compile function.
 */
export interface CompileOptions {
  /** File path for error messages and `export const file`. */
  filepath?: string;
  /** Route URL associated with the file. */
  url?: string;
}

/**
 * Result of compiling MDX to an Astro-compatible module.
 */
export interface CompileResult {
  /** Generated JavaScript/JSX module code. */
  code: string;
  /** Parsed frontmatter object. */
  frontmatter: Record<string, unknown>;
  /** Extracted heading metadata. */
  headings: HeadingEntry[];
  /** Whether the user provided their own export default. */
  hasUserDefaultExport: boolean;
}

/**
 * Compile MDX source to an Astro-compatible JavaScript module.
 *
 * This function extracts frontmatter, hoists imports/exports, parses markdown
 * to JSX, and generates a complete module with createComponent wrapper.
 *
 * The backend (NAPI or WASM) is automatically selected based on the runtime
 * environment:
 * - Node.js: Uses markflow-napi (native bindings)
 * - Browser/Edge: Uses markflow-wasm (WebAssembly)
 *
 * @param source - The MDX source code
 * @param options - Optional compile options
 * @returns Promise resolving to the compile result
 *
 * @example
 * ```javascript
 * import { compile } from 'markflow';
 *
 * const result = await compile(`---
 * title: Hello
 * ---
 *
 * # Welcome
 *
 * This is **bold** text.
 * `, { filepath: 'page.mdx' });
 *
 * console.log(result.code);
 * console.log(result.headings);
 * console.log(result.frontmatter);
 * ```
 */
export function compile(
  source: string,
  options?: CompileOptions
): Promise<CompileResult>;
