/**
 * @file Shiki syntax highlighting transforms for code blocks
 * @module transforms/shiki
 */

import { parseFragment, serialize } from "parse5";

/**
 * Walks an HTML AST tree and applies a visitor function to each node
 * @param {object} node - The HTML AST node to walk
 * @param {function} visit - The visitor function to call for each node
 */
function walk(node, visit) {
  visit(node);
  if (!node.childNodes) return;
  for (const child of node.childNodes) {
    walk(child, visit);
  }
}

/**
 * Gets an attribute value from an HTML AST node
 * @param {object} node - The HTML AST node
 * @param {string} name - The attribute name to get
 * @returns {string|null} The attribute value or null if not found
 */
function getAttr(node, name) {
  const attrs = node.attrs || [];
  const found = attrs.find((attr) => attr.name === name);
  return found ? found.value : null;
}

/**
 * Extracts text content from an HTML AST node recursively
 * @param {object} node - The HTML AST node
 * @returns {string} The concatenated text content
 */
function getText(node) {
  if (!node.childNodes) return "";
  let text = "";
  for (const child of node.childNodes) {
    if (child.nodeName === "#text") {
      text += child.value || "";
    } else {
      text += getText(child);
    }
  }
  return text;
}

/**
 * Highlights code blocks in HTML using Shiki syntax highlighter
 * @param {string} html - The HTML string containing code blocks
 * @param {function} highlight - Async function (code, lang) => highlightedHtml
 * @returns {Promise<string>} The HTML with highlighted code blocks
 */
export async function highlightHtmlBlocks(html, highlight) {
  // Suppress parse5 warnings for JSX components in HTML
  // MDX allows JSX components inside HTML tags, but parse5 doesn't recognize them
  const fragment = parseFragment(html, {
    onParseError: (error) => {
      // Silently ignore end-tag-mismatch errors (typically from JSX in <p> tags)
      if (error.code === 'end-tag-mismatch') {
        return;
      }
      // Log other parse errors for debugging
      console.warn('[markflow] parse5 warning:', error);
    }
  });
  const tasks = [];

  walk(fragment, (node) => {
    if (node.nodeName !== "pre") return;
    const codeNode = (node.childNodes || []).find(
      (child) => child.nodeName === "code",
    );
    if (!codeNode) return;

    const codeText = getText(codeNode).trimEnd();
    if (!codeText) return;
    const classAttr = getAttr(codeNode, "class") || "";
    const lang = classAttr
      .split(/\s+/)
      .find((value) => value.startsWith("language-"))
      ?.slice("language-".length);

    tasks.push(
      highlight(codeText, lang).then((shikiHtml) => {
        const highlighted = parseFragment(shikiHtml);
        const pre = (highlighted.childNodes || []).find(
          (child) => child.nodeName === "pre",
        );
        if (pre) {
          node.nodeName = pre.nodeName;
          node.tagName = pre.tagName;
          node.attrs = pre.attrs;
          node.childNodes = pre.childNodes;
        }
      }),
    );
  });

  if (tasks.length > 0) {
    await Promise.all(tasks);
  }
  return serialize(fragment);
}

/**
 * Rewrites Astro set:html fragments with Shiki-highlighted code
 * Searches for <Fragment set:html={...} /> patterns and applies syntax highlighting
 * @param {string} code - The JSX code to transform
 * @param {function} highlight - Async function (code, lang) => highlightedHtml
 * @returns {Promise<string>} The transformed code with highlighted blocks
 */
export async function rewriteAstroSetHtml(code, highlight) {
  const marker = "<Fragment set:html={";
  const idx = code.indexOf(marker);
  if (idx === -1) return code;
  const start = idx + marker.length;
  const end = code.indexOf("} />", start);
  if (end === -1) return code;

  let literal = code.slice(start, end).trim();
  if (!literal) return code;

  let html;
  try {
    html = JSON.parse(literal);
  } catch {
    return code;
  }

  const rewritten = await highlightHtmlBlocks(html, highlight);
  const encoded = JSON.stringify(rewritten);
  return `${code.slice(0, start)}${encoded}${code.slice(end)}`;
}
