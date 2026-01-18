/**
 * @file Transforms compiled blocks into JSX code
 * @module transforms/blocks-to-jsx
 */

/**
 * Escapes a string value for use in JSX prop.
 * Uses JSON.stringify for proper JS string escaping.
 * @param {string} value - The string to escape
 * @returns {string} Escaped string suitable for JSX
 */
function escapeJsString(value) {
  // Use JSON.stringify which handles all JS escaping, then remove the outer quotes
  return JSON.stringify(String(value)).slice(1, -1);
}

/**
 * Converts blocks array from Rust compiler into JSX code with component imports and exports
 * @param {Array<{type: string, content?: string, name?: string, props?: object, slotHtml?: string}>} blocks - Array of blocks from compiler
 * @param {object} frontmatter - Frontmatter object to export
 * @param {Array} headings - Headings array to export
 * @param {import('markflow/registry').ComponentRegistry} [registry] - Component registry for import resolution
 * @returns {string} Complete JSX module code with imports, exports, and default component
 */
export function blocksToJsx(blocks, frontmatter = {}, headings = [], registry = null) {
  const fragments = [];
  const componentImports = new Map(); // component name -> { modulePath, exportType }

  // Get supported directives from registry if available
  const supportedDirectives = registry?.getSupportedDirectives() ?? [];

  for (const block of blocks) {
    if (block.type === "html") {
      fragments.push(block.content);
    } else if (block.type === "component") {
      // Handle directive components using registry
      const isDirective = supportedDirectives.includes(block.name);
      let componentName = block.name;
      let effectiveProps = block.props;

      if (isDirective && registry) {
        const mapping = registry.getDirectiveMapping(block.name);
        if (mapping) {
          componentName = mapping.component;
          // Apply injected props from mapping
          if (mapping.injectProps) {
            const injectedProps = {};
            for (const [propKey, propSource] of Object.entries(mapping.injectProps)) {
              if (propSource.source === 'directive_name') {
                injectedProps[propKey] = { type: "literal", value: block.name };
              } else if (propSource.source === 'literal' && propSource.value) {
                injectedProps[propKey] = { type: "literal", value: propSource.value };
              }
            }
            effectiveProps = { ...block.props, ...injectedProps };
          }
        }
      }

      // Skip Fragment - it's a built-in Astro component
      if (componentName !== "Fragment") {
        const componentDef = registry?.getComponent(componentName);
        const modulePath = componentDef?.modulePath ?? '@astrojs/starlight/components';
        const exportType = componentDef?.exportType ?? 'default';
        componentImports.set(componentName, { modulePath, exportType });
      }

      const propsStr = effectiveProps
        ? Object.entries(effectiveProps)
            .map(([key, value]) => {
              // Handle PropValue enum from Rust: { type: "literal"|"expression", value: string }
              if (typeof value === "object" && value !== null && "type" in value && "value" in value) {
                if (value.type === "literal") {
                  return `${key}="${escapeJsString(value.value)}"`;
                } else if (value.type === "expression") {
                  return `${key}={${value.value}}`;
                }
              }
              if (typeof value === "string") {
                return `${key}="${escapeJsString(value)}"`;
              }
              return `${key}={${JSON.stringify(value)}}`;
            })
            .join(" ")
        : "";
      const openTag = propsStr
        ? `<${componentName} ${propsStr}>`
        : `<${componentName}>`;
      fragments.push(`${openTag}${block.slotHtml || ""}</${componentName}>`);
    }
  }

  // Generate imports grouped by module path
  const importsByModule = new Map();
  for (const [name, { modulePath, exportType }] of componentImports) {
    if (!importsByModule.has(modulePath)) {
      importsByModule.set(modulePath, { named: [], default: [] });
    }
    if (exportType === 'named') {
      importsByModule.get(modulePath).named.push(name);
    } else {
      importsByModule.get(modulePath).default.push(name);
    }
  }

  const componentImportLines = Array.from(importsByModule.entries())
    .map(([modulePath, { named, default: defaults }]) => {
      const lines = [];
      if (named.length > 0) {
        lines.push(`import { ${named.join(', ')} } from '${modulePath}';`);
      }
      for (const name of defaults) {
        lines.push(`import ${name} from '${modulePath}/${name}.astro';`);
      }
      return lines.join('\n');
    })
    .filter(Boolean)
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
