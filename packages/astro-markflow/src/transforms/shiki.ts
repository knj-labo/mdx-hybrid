/**
 * Shiki syntax highlighting transforms for code blocks
 * @module transforms/shiki
 */

import { parseFragment, serialize } from 'parse5';
import type { DocumentFragment, Node, Element, TextNode } from '../vite-plugin/types.js';

/**
 * Shiki highlighter function type.
 */
export type ShikiHighlighter = (code: string, lang?: string) => Promise<string>;

/**
 * Walks an HTML AST tree and applies a visitor function to each node.
 */
function walk(node: Node, visit: (node: Node) => void): void {
  visit(node);
  if ('childNodes' in node && node.childNodes) {
    for (const child of node.childNodes) {
      walk(child, visit);
    }
  }
}

/**
 * Gets an attribute value from an HTML AST node.
 */
function getAttr(node: Element, name: string): string | null {
  const attrs = node.attrs || [];
  const found = attrs.find((attr) => attr.name === name);
  return found ? found.value : null;
}

/**
 * Extracts text content from an HTML AST node recursively.
 */
function getText(node: Node): string {
  if (!('childNodes' in node) || !node.childNodes) return '';
  let text = '';
  for (const child of node.childNodes) {
    if (child.nodeName === '#text') {
      text += (child as TextNode).value || '';
    } else {
      text += getText(child);
    }
  }
  return text;
}

/**
 * Highlights code blocks in HTML using Shiki syntax highlighter.
 */
export async function highlightHtmlBlocks(
  html: string,
  highlight: ShikiHighlighter
): Promise<string> {
  // PERF: Early skip if no <pre> tags exist (avoids expensive parse5 parsing)
  if (!/<pre[\s>]/.test(html)) {
    return html;
  }

  // Suppress parse5 warnings for JSX components in HTML
  const fragment = parseFragment(html, {
    onParseError: (error) => {
      // Silently ignore end-tag-mismatch errors (typically from JSX in <p> tags)
      if ((error.code as string) === 'end-tag-mismatch') {
        return;
      }
      // Log other parse errors for debugging
      console.warn('[markflow] parse5 warning:', error);
    },
  }) as DocumentFragment;

  const tasks: Promise<void>[] = [];

  walk(fragment, (node) => {
    if (node.nodeName !== 'pre') return;
    const element = node as Element;
    const codeNode = (element.childNodes || []).find(
      (child): child is Element => child.nodeName === 'code'
    );
    if (!codeNode) return;

    const codeText = getText(codeNode).trimEnd();
    if (!codeText) return;
    const classAttr = getAttr(codeNode, 'class') || '';
    const lang = classAttr
      .split(/\s+/)
      .find((value) => value.startsWith('language-'))
      ?.slice('language-'.length);

    tasks.push(
      highlight(codeText, lang).then((shikiHtml) => {
        const highlighted = parseFragment(shikiHtml) as DocumentFragment;
        const pre = (highlighted.childNodes || []).find(
          (child): child is Element => child.nodeName === 'pre'
        );
        if (pre) {
          element.nodeName = pre.nodeName;
          element.tagName = pre.tagName;
          element.attrs = pre.attrs;
          element.childNodes = pre.childNodes;
        }
      })
    );
  });

  if (tasks.length > 0) {
    await Promise.all(tasks);
  }
  return serialize(fragment);
}

/**
 * Rewrites Astro set:html fragments with Shiki-highlighted code.
 * Searches for <_Fragment set:html={...} /> patterns and applies syntax highlighting.
 * Processes ALL occurrences in the code, not just the first one.
 */
export async function rewriteAstroSetHtml(
  code: string,
  highlight: ShikiHighlighter
): Promise<string> {
  if (!code || typeof code !== 'string') {
    return code;
  }

  const marker = '<_Fragment set:html={';
  let result = code;
  let searchStart = 0;

  // Process ALL occurrences in a loop
  while (true) {
    const idx = result.indexOf(marker, searchStart);
    if (idx === -1) break;

    const start = idx + marker.length;
    const end = result.indexOf('} />', start);
    if (end === -1) break;

    const literal = result.slice(start, end).trim();
    if (!literal) {
      searchStart = end;
      continue;
    }

    let html: string;
    try {
      html = JSON.parse(literal) as string;
    } catch {
      searchStart = end;
      continue;
    }

    const rewritten = await highlightHtmlBlocks(html, highlight);
    const encoded = JSON.stringify(rewritten);
    result = result.slice(0, start) + encoded + result.slice(end);
    searchStart = start + encoded.length + 4; // Move past this occurrence
  }

  return result;
}
