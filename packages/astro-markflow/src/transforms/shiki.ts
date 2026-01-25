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
 * Searches for <Fragment set:html={...} /> patterns and applies syntax highlighting.
 */
export async function rewriteAstroSetHtml(
  code: string,
  highlight: ShikiHighlighter
): Promise<string> {
  if (!code || typeof code !== 'string') {
    return code;
  }
  const marker = '<Fragment set:html={';
  const idx = code.indexOf(marker);
  if (idx === -1) return code;
  const start = idx + marker.length;
  const end = code.indexOf('} />', start);
  if (end === -1) return code;

  const literal = code.slice(start, end).trim();
  if (!literal) return code;

  let html: string;
  try {
    html = JSON.parse(literal) as string;
  } catch {
    return code;
  }

  const rewritten = await highlightHtmlBlocks(html, highlight);
  const encoded = JSON.stringify(rewritten);
  return `${code.slice(0, start)}${encoded}${code.slice(end)}`;
}
