/**
 * @file Transforms compiled blocks into JSX code
 * @module transforms/blocks-to-jsx
 */

/**
 * Converts blocks array from Rust compiler into JSX code with component imports and exports
 * @param {Array<{type: string, content?: string, name?: string, props?: object, slotHtml?: string}>} blocks - Array of blocks from compiler
 * @param {object} frontmatter - Frontmatter object to export
 * @param {Array} headings - Headings array to export
 * @returns {string} Complete JSX module code with imports, exports, and default component
 */
export function blocksToJsx(blocks, frontmatter = {}, headings = []) {
  const fragments = [];
  const componentImports = new Set();

  for (const block of blocks) {
    if (block.type === "html") {
      fragments.push(block.content);
    } else if (block.type === "component") {
      componentImports.add(block.name);
      const propsStr = block.props
        ? Object.entries(block.props)
            .map(([key, value]) => {
              if (typeof value === "string") {
                return `${key}="${value.replace(/"/g, '\\"')}"`;
              }
              return `${key}={${JSON.stringify(value)}}`;
            })
            .join(" ")
        : "";
      const openTag = propsStr
        ? `<${block.name} ${propsStr}>`
        : `<${block.name}>`;
      fragments.push(`${openTag}${block.slotHtml || ""}</${block.name}>`);
    }
  }

  const componentImportLines = Array.from(componentImports)
    .map(
      (name) =>
        `import ${name} from '@astrojs/starlight/components/${name}.astro';`,
    )
    .join("\n");

  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);
  const jsxContent = fragments.join("\n");

  return `${componentImportLines}
export const frontmatter = ${frontmatterJson};
export function getHeadings() { return ${headingsJson}; }
export default function MarkflowContent() {
  return (
    <>
${jsxContent}
    </>
  );
}
`;
}
