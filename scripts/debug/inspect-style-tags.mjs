import { chromium } from 'playwright';

const url =
  process.argv[2] ||
  process.env.URL ||
  'http://localhost:4321/en/concepts/why-astro/';

const browser = await chromium.launch();
const page = await browser.newPage();

await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 });
await page.waitForTimeout(500);

const result = await page.evaluate(() => {
  const entries = [];
  const nodes = Array.from(document.head.querySelectorAll('style, link[rel="stylesheet"]'));

  for (const [index, node] of nodes.entries()) {
    if (node.tagName === 'LINK') {
      entries.push({
        index,
        tag: 'link',
        href: node.getAttribute('href') || '',
        dataViteDevId: node.getAttribute('data-vite-dev-id') || '',
        media: node.getAttribute('media') || '',
        hasLayerStatement: false,
        hasMarginReset: false,
        hasLayerBlock: false,
      });
      continue;
    }

    const text = node.textContent || '';
    const hasLayerStatement = /@layer\s+[^\{;]+;/.test(text);
    const hasLayerBlock = /@layer\s+[^\{;]*\{/.test(text);
    const hasMarginReset = /\*\s*\{[^}]*margin\s*:\s*0/.test(text);

    entries.push({
      index,
      tag: 'style',
      dataViteDevId: node.getAttribute('data-vite-dev-id') || '',
      media: node.getAttribute('media') || '',
      textLength: text.length,
      hasLayerStatement,
      hasLayerBlock,
      hasMarginReset,
      firstSnippet: text.slice(0, 160).replace(/\s+/g, ' ').trim(),
    });
  }

  return entries;
});

const lines = [];
lines.push('# inspect-style-tags');
lines.push(`- url: ${url}`);
lines.push('- head stylesheets:');

if (!result.length) {
  lines.push('  - (none)');
} else {
  for (const entry of result) {
    lines.push(`  - #${entry.index} <${entry.tag}>`);
    if (entry.href) {
      lines.push(`    - href: ${entry.href}`);
    }
    if (entry.dataViteDevId) {
      lines.push(`    - data-vite-dev-id: ${entry.dataViteDevId}`);
    }
    if (entry.media) {
      lines.push(`    - media: ${entry.media}`);
    }
    if (entry.tag === 'style') {
      lines.push(`    - textLength: ${entry.textLength}`);
      lines.push(`    - hasLayerStatement: ${entry.hasLayerStatement}`);
      lines.push(`    - hasLayerBlock: ${entry.hasLayerBlock}`);
      lines.push(`    - hasMarginReset: ${entry.hasMarginReset}`);
      if (entry.firstSnippet) {
        lines.push(`    - firstSnippet: ${entry.firstSnippet}`);
      }
    }
  }
}

console.log(lines.join('\n'));

await browser.close();
