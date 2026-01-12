/**
 * @file ExpressiveCode component injection and rewriting transforms
 * @module transforms/expressive-code
 */

import { collectImportedNames, insertAfterImports } from "../utils/imports.js";

/**
 * Decodes HTML entities in a string
 * @param {string} value - The string with HTML entities
 * @returns {string} The decoded string
 */
export function decodeHtmlEntities(value) {
  if (!value || !value.includes("&")) return value;
  return value
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) =>
      String.fromCodePoint(Number.parseInt(hex, 16)),
    )
    .replace(/&#([0-9]+);/g, (_, num) =>
      String.fromCodePoint(Number.parseInt(num, 10)),
    )
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

/**
 * Rewrites <pre><code> blocks to ExpressiveCode components
 * @param {string} code - The code to transform
 * @param {string} componentName - The name of the Code component (e.g., "Code" or "MyCode")
 * @returns {{ code: string, changed: boolean }} The transformed code and whether changes were made
 */
export function rewriteExpressiveCodeBlocks(code, componentName) {
  const pattern =
    /<pre><code(?: class="language-([^"]+)")?>([\s\S]*?)<\/code><\/pre>/g;
  let changed = false;
  const next = code.replace(pattern, (match, lang, raw) => {
    changed = true;
    const decoded = decodeHtmlEntities(raw);
    const props = [`code={${JSON.stringify(decoded)}}`];
    if (lang) {
      props.push(`lang="${lang}"`);
    }
    return `<${componentName} ${props.join(" ")} />`;
  });
  return { code: next, changed };
}

/**
 * Injects ExpressiveCode component import if needed
 * @param {string} code - The code to transform
 * @param {object} config - ExpressiveCode config with component and moduleId
 * @returns {string} The transformed code with import added if needed
 */
export function injectExpressiveCodeComponent(code, config) {
  const importName = config.component;
  const imported = collectImportedNames(code);
  if (imported.has(importName)) {
    return code;
  }
  const importLine =
    importName === "Code"
      ? `import { Code } from '${config.moduleId}';`
      : `import { Code as ${importName} } from '${config.moduleId}';`;
  return insertAfterImports(code, importLine);
}
