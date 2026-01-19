/**
 * Transforms compiled blocks into JSX code
 * @module transforms/blocks-to-jsx
 */

import type { Registry } from 'markflow/registry';
import type { HeadingEntry } from 'markflow';

/**
 * Prop value from the Rust compiler.
 */
export interface PropValue {
  type: 'literal' | 'expression';
  value: string;
}

/**
 * Block from the Rust compiler.
 */
export interface Block {
  type: 'html' | 'component';
  content?: string;
  name?: string;
  props?: Record<string, PropValue | string | unknown>;
  slotHtml?: string;
}

/**
 * Escapes a string value for use in JSX prop.
 * Uses JSON.stringify for proper JS string escaping.
 */
function escapeJsString(value: string): string {
  // Use JSON.stringify which handles all JS escaping, then remove the outer quotes
  return JSON.stringify(String(value)).slice(1, -1);
}

/**
 * Converts blocks array from Rust compiler into JSX code with component imports and exports.
 *
 * @param blocks - Array of blocks from compiler
 * @param frontmatter - Frontmatter object to export
 * @param headings - Headings array to export
 * @param registry - Component registry for import resolution
 * @returns Complete JSX module code with imports, exports, and default component
 */
export function blocksToJsx(
  blocks: Block[],
  frontmatter: Record<string, unknown> = {},
  headings: HeadingEntry[] = [],
  registry: Registry | null = null
): string {
  const fragments: string[] = [];
  const componentImports = new Map<string, { modulePath: string; exportType: string }>();

  // Get supported directives from registry if available
  const supportedDirectives = registry?.getSupportedDirectives() ?? [];

  for (const block of blocks) {
    if (block.type === 'html') {
      fragments.push(block.content ?? '');
    } else if (block.type === 'component') {
      // Handle directive components using registry
      const isDirective = block.name ? supportedDirectives.includes(block.name) : false;
      let componentName = block.name ?? '';
      let effectiveProps = block.props;
      let effectiveSlot = block.slotHtml ?? '';

      if (isDirective && registry && block.name) {
        const mapping = registry.getDirectiveMapping(block.name);
        if (mapping) {
          componentName = mapping.component;
          // Apply injected props from mapping
          if (mapping.injectProps) {
            const injectedProps: Record<string, PropValue> = {};
            for (const [propKey, propSource] of Object.entries(mapping.injectProps)) {
              if (propSource.source === 'directive_name') {
                injectedProps[propKey] = { type: 'literal', value: block.name };
              } else if (propSource.source === 'literal' && propSource.value) {
                injectedProps[propKey] = { type: 'literal', value: propSource.value };
              }
            }
            effectiveProps = { ...block.props, ...injectedProps };
          }
        }
      }

      // Normalize Steps slot to a single <ol> child (Starlight requirement)
      if (componentName === 'Steps') {
        const trimmed = effectiveSlot.trim();
        if (!(trimmed.startsWith('<ol') && trimmed.endsWith('</ol>'))) {
          effectiveSlot = `<ol><li>${effectiveSlot}</li></ol>`;
        }
      }
      // Normalize FileTree slot to a single <ul> child (Starlight requirement)
      if (componentName === 'FileTree') {
        const trimmed = effectiveSlot.trim();
        const hasLi = /<li[\s>]/i.test(trimmed);
        if (!trimmed) {
          effectiveSlot = '<ul><li></li></ul>';
        } else if (trimmed.startsWith('<ul') && trimmed.endsWith('</ul>')) {
          effectiveSlot = hasLi ? effectiveSlot : trimmed.replace('</ul>', '<li></li></ul>');
        } else {
          effectiveSlot = hasLi
            ? `<ul>${effectiveSlot}</ul>`
            : `<ul><li>${effectiveSlot}</li></ul>`;
        }
      }
      // Escape raw JSX braces inside slot HTML to prevent expression evaluation
      effectiveSlot = effectiveSlot
        .replaceAll('{', '&#123;')
        .replaceAll('}', '&#125;');

      // Skip Fragment - it's a built-in Astro component
      if (componentName !== 'Fragment') {
        const componentDef = registry?.getComponent(componentName);
        const modulePath = componentDef?.modulePath ?? '@astrojs/starlight/components';
        const exportType = componentDef?.exportType ?? 'default';
        componentImports.set(componentName, { modulePath, exportType });
      }

      const propsStr = effectiveProps
        ? Object.entries(effectiveProps)
            .map(([key, value]) => {
              // Handle PropValue enum from Rust: { type: "literal"|"expression", value: string }
              if (typeof value === 'object' && value !== null && 'type' in value && 'value' in value) {
                const propValue = value as PropValue;
                if (propValue.type === 'literal') {
                  return `${key}="${escapeJsString(propValue.value)}"`;
                } else if (propValue.type === 'expression') {
                  return `${key}={${propValue.value}}`;
                }
              }
              if (typeof value === 'string') {
                return `${key}="${escapeJsString(value)}"`;
              }
              return `${key}={${JSON.stringify(value)}}`;
            })
            .join(' ')
        : '';
      const openTag = propsStr
        ? `<${componentName} ${propsStr}>`
        : `<${componentName}>`;
      fragments.push(`${openTag}${effectiveSlot}</${componentName}>`);
    }
  }

  // Generate imports grouped by module path
  const importsByModule = new Map<string, { named: string[]; default: string[] }>();
  for (const [name, { modulePath, exportType }] of componentImports) {
    if (!importsByModule.has(modulePath)) {
      importsByModule.set(modulePath, { named: [], default: [] });
    }
    const entry = importsByModule.get(modulePath)!;
    if (exportType === 'named') {
      entry.named.push(name);
    } else {
      entry.default.push(name);
    }
  }

  const componentImportLines = Array.from(importsByModule.entries())
    .map(([modulePath, { named, default: defaults }]) => {
      const lines: string[] = [];
      if (named.length > 0) {
        lines.push(`import { ${named.join(', ')} } from '${modulePath}';`);
      }
      for (const name of defaults) {
        lines.push(`import ${name} from '${modulePath}/${name}.astro';`);
      }
      return lines.join('\n');
    })
    .filter(Boolean)
    .join('\n');

  const frontmatterJson = JSON.stringify(frontmatter);
  const headingsJson = JSON.stringify(headings);
  const jsxContent = fragments.join('\n');

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
