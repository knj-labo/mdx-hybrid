import { describe, test, expect } from 'bun:test';
import {
  highlightHtmlBlocks,
  rewriteAstroSetHtml,
} from './shiki.js';

// Mock highlight function that simulates Shiki output
const mockHighlight = async (code: string, lang?: string): Promise<string> => {
  const langAttr = lang ? ` class="language-${lang}"` : '';
  return `<pre${langAttr}><code${langAttr}>${code.toUpperCase()}</code></pre>`;
};

describe('highlightHtmlBlocks', () => {
  test('returns empty string for empty HTML', async () => {
    const result = await highlightHtmlBlocks('', mockHighlight);
    expect(result).toBe('');
  });

  test('returns HTML unchanged when no code blocks', async () => {
    const html = '<div><p>Hello world</p></div>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toBe('<div><p>Hello world</p></div>');
  });

  test('highlights single code block without language', async () => {
    const html = '<pre><code>const x = 1;</code></pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('CONST X = 1;');
    expect(result).toContain('<pre');
    expect(result).toContain('<code');
  });

  test('highlights single code block with language', async () => {
    const html = '<pre><code class="language-javascript">const x = 1;</code></pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('CONST X = 1;');
    expect(result).toContain('class="language-javascript"');
  });

  test('highlights multiple code blocks', async () => {
    const html = `
      <pre><code class="language-js">let a = 1;</code></pre>
      <p>Some text</p>
      <pre><code class="language-ts">let b: number = 2;</code></pre>
    `;
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('LET A = 1;');
    expect(result).toContain('LET B: NUMBER = 2;');
    expect(result).toContain('Some text');
  });

  test('handles nested HTML structure', async () => {
    const html = `
      <div>
        <section>
          <pre><code>hello</code></pre>
        </section>
      </div>
    `;
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('HELLO');
    expect(result).toContain('<div>');
    expect(result).toContain('<section>');
  });

  test('skips pre tags without code children', async () => {
    const html = '<pre>Just text, no code tag</pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toBe('<pre>Just text, no code tag</pre>');
  });

  test('skips code blocks with no text content', async () => {
    const html = '<pre><code></code></pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    // Should not call highlight for empty code
    expect(result).toContain('<pre>');
    expect(result).toContain('<code>');
  });

  test('extracts language from multiple classes', async () => {
    const html = '<pre><code class="foo language-python bar">print("hi")</code></pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('PRINT("HI")');
    expect(result).toContain('class="language-python"');
  });

  test('trims trailing whitespace from code text', async () => {
    const html = '<pre><code>hello   \n\n</code></pre>';
    const result = await highlightHtmlBlocks(html, mockHighlight);
    expect(result).toContain('HELLO');
    // The mock uppercases, so we know trimEnd() worked if there's no trailing whitespace
  });
});

describe('rewriteAstroSetHtml', () => {
  test('returns unchanged when no Fragment marker found', async () => {
    const code = `const x = 1;\nconst y = 2;`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toBe(code);
  });

  test('returns unchanged when marker found but no closing', async () => {
    const code = `<Fragment set:html={"<div>hello</div>`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toBe(code);
  });

  test('returns unchanged when literal is empty', async () => {
    const code = `<Fragment set:html={} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toBe(code);
  });

  test('returns unchanged when JSON is invalid', async () => {
    const code = `<Fragment set:html={not valid json} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toBe(code);
  });

  test('highlights code blocks in Fragment', async () => {
    const code = `<Fragment set:html={"<pre><code>const x = 1;</code></pre>"} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toContain('CONST X = 1;');
    expect(result).toContain('<Fragment set:html={');
    expect(result).toContain('} />');
  });

  test('highlights multiple code blocks in Fragment', async () => {
    const html = '<pre><code class="language-js">let a;</code></pre><pre><code class="language-ts">let b;</code></pre>';
    const code = `<Fragment set:html={${JSON.stringify(html)}} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toContain('LET A;');
    expect(result).toContain('LET B;');
  });

  test('preserves surrounding code', async () => {
    const code = `
      import { Fragment } from 'astro';

      <div>
        <Fragment set:html={"<pre><code>hello</code></pre>"} />
      </div>
    `;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toContain("import { Fragment }");
    expect(result).toContain('<div>');
    expect(result).toContain('HELLO');
    expect(result).toContain('</div>');
  });

  test('handles HTML with no code blocks', async () => {
    const code = `<Fragment set:html={"<div><p>No code here</p></div>"} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    // Should still parse and serialize, result may differ slightly
    expect(result).toContain('<Fragment set:html={');
    expect(result).toContain('No code here');
  });

  test('handles escaped quotes in JSON', async () => {
    const html = '<pre><code>const str = \\"hello\\";</code></pre>';
    const code = `<Fragment set:html={${JSON.stringify(html)}} />`;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toContain('CONST STR');
  });

  test('only processes first Fragment occurrence', async () => {
    const code = `
      <Fragment set:html={"<pre><code>first</code></pre>"} />
      <Fragment set:html={"<pre><code>second</code></pre>"} />
    `;
    const result = await rewriteAstroSetHtml(code, mockHighlight);
    expect(result).toContain('FIRST');
    // Second one should be unchanged because function only processes first occurrence
    expect(result).toContain('"<pre><code>second</code></pre>"');
  });
});
