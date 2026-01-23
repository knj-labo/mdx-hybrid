/**
 * Shared module export generation for Astro MDX compatibility.
 * @module utils/module-exports
 */

export interface ModuleExportOptions {
  frontmatter: Record<string, unknown>;
  headings: Array<{ depth: number; slug: string; text: string }>;
  componentName?: string; // default: 'MarkflowContent'
  skipDefaultExport?: boolean; // skip default export when already provided by another tool (e.g., @mdx-js/mdx)
}

/**
 * Generates consistent module exports for Astro MDX compatibility.
 * Ensures Content, frontmatter, getHeadings, and default export are always present.
 *
 * @param opts - Options for export generation
 * @returns Generated export statements
 */
export function generateModuleExports(opts: ModuleExportOptions): string {
  const name = opts.componentName ?? 'MarkflowContent';
  const frontmatterJson = JSON.stringify(opts.frontmatter);
  const headingsJson = JSON.stringify(opts.headings);

  const lines = [
    `export const frontmatter = ${frontmatterJson};`,
    `export function getHeadings() { return ${headingsJson}; }`,
    `export const Content = ${name};`,
  ];

  if (!opts.skipDefaultExport) {
    lines.push(`export default ${name};`);
  }

  return lines.join('\n');
}
