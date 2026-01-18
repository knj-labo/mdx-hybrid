/**
 * Tests for context-aware transform wrappers
 */

import { describe, expect, test } from 'bun:test';
import {
  transformExpressiveCode,
  transformInjectAstroComponents,
  transformInjectStarlightComponents,
  transformShikiHighlight,
} from './index.js';
import type { TransformContext } from '../types.js';

/**
 * Helper to create a minimal transform context
 */
function createContext(overrides: Partial<TransformContext> = {}): TransformContext {
  return {
    code: '',
    source: '# Hello World',
    filename: '/test/file.md',
    frontmatter: {},
    headings: [],
    config: {
      expressiveCode: null,
      starlightComponents: false,
      shiki: null,
    },
    ...overrides,
  };
}

describe('transformExpressiveCode', () => {
  test('returns context unchanged when expressiveCode is null', () => {
    const ctx = createContext({
      code: '<pre><code>const x = 1;</code></pre>',
    });
    const result = transformExpressiveCode(ctx);
    expect(result.code).toBe('<pre><code>const x = 1;</code></pre>');
  });

  test('rewrites code blocks when expressiveCode is configured', () => {
    const ctx = createContext({
      code: '<pre><code>const x = 1;</code></pre>',
      config: {
        expressiveCode: { component: 'Code', moduleId: 'astro-expressive-code/components' },
        starlightComponents: false,
        shiki: null,
      },
    });
    const result = transformExpressiveCode(ctx);
    expect(result.code).toContain('<Code');
    expect(result.code).toContain('code={');
    expect(result.code).toContain("import { Code } from 'astro-expressive-code/components'");
  });

  test('rewrites code blocks with language', () => {
    const ctx = createContext({
      code: '<pre><code class="language-javascript">const x = 1;</code></pre>',
      config: {
        expressiveCode: { component: 'Code', moduleId: 'astro-expressive-code/components' },
        starlightComponents: false,
        shiki: null,
      },
    });
    const result = transformExpressiveCode(ctx);
    expect(result.code).toContain('<Code code={"const x = 1;"} lang="javascript" />');
  });

  test('uses custom component name', () => {
    const ctx = createContext({
      code: '<pre><code>const x = 1;</code></pre>',
      config: {
        expressiveCode: { component: 'MyCode', moduleId: 'my-code-module' },
        starlightComponents: false,
        shiki: null,
      },
    });
    const result = transformExpressiveCode(ctx);
    expect(result.code).toContain('<MyCode');
    expect(result.code).toContain("import { Code as MyCode } from 'my-code-module'");
  });
});

describe('transformInjectAstroComponents', () => {
  test('injects Code component when used', () => {
    const ctx = createContext({
      code: `export default function Content() {
  return <Code lang="js">const x = 1;</Code>;
}`,
    });
    const result = transformInjectAstroComponents(ctx);
    expect(result.code).toContain("import { Code } from 'astro/components'");
  });

  test('does not inject when components are not used', () => {
    const ctx = createContext({
      code: `export default function Content() {
  return <div>Hello</div>;
}`,
    });
    const result = transformInjectAstroComponents(ctx);
    expect(result.code).not.toContain('import { Code }');
    expect(result.code).not.toContain('import { Prism }');
  });

  test('does not duplicate existing imports', () => {
    const ctx = createContext({
      code: `import { Code } from 'astro/components';
export default function Content() {
  return <Code lang="js">const x = 1;</Code>;
}`,
    });
    const result = transformInjectAstroComponents(ctx);
    // Count occurrences of the import
    const matches = result.code.match(/import \{ Code \}/g);
    expect(matches).toHaveLength(1);
  });
});

describe('transformInjectStarlightComponents', () => {
  test('returns context unchanged when starlightComponents is false', () => {
    const ctx = createContext({
      code: `export default function Content() {
  return <Aside>Note</Aside>;
}`,
    });
    const result = transformInjectStarlightComponents(ctx);
    expect(result.code).not.toContain('import { Aside }');
  });

  test('injects components when starlightComponents is true', () => {
    const ctx = createContext({
      code: `export default function Content() {
  return <Aside>Note</Aside>;
}`,
      config: {
        expressiveCode: null,
        starlightComponents: true,
        shiki: null,
      },
    });
    const result = transformInjectStarlightComponents(ctx);
    expect(result.code).toContain("import { Aside } from '@astrojs/starlight/components'");
  });

  test('injects multiple components when used', () => {
    const ctx = createContext({
      code: `export default function Content() {
  return (
    <>
      <Aside>Note</Aside>
      <Card>Content</Card>
    </>
  );
}`,
      config: {
        expressiveCode: null,
        starlightComponents: true,
        shiki: null,
      },
    });
    const result = transformInjectStarlightComponents(ctx);
    expect(result.code).toContain('Aside');
    expect(result.code).toContain('Card');
  });
});

describe('transformShikiHighlight', () => {
  test('returns context unchanged when shiki is null', async () => {
    const ctx = createContext({
      code: '<Fragment set:html={"<pre><code>test</code></pre>"} />',
    });
    const result = await transformShikiHighlight(ctx);
    expect(result.code).toBe('<Fragment set:html={"<pre><code>test</code></pre>"} />');
  });

  test('applies shiki highlighting when enabled', async () => {
    const mockHighlight = async (_code: string, _lang?: string): Promise<string> => {
      return `<pre class="shiki"><code>${_code}</code></pre>`;
    };
    const ctx = createContext({
      code: '<Fragment set:html={"<pre><code>const x = 1;</code></pre>"} />',
      config: {
        expressiveCode: null,
        starlightComponents: false,
        shiki: mockHighlight,
      },
    });
    const result = await transformShikiHighlight(ctx);
    expect(result.code).toContain('shiki');
  });
});

describe('context immutability', () => {
  test('transforms return new context objects', () => {
    const original = createContext({
      code: '<pre><code>test</code></pre>',
      config: {
        expressiveCode: { component: 'Code', moduleId: 'astro-expressive-code/components' },
        starlightComponents: false,
        shiki: null,
      },
    });
    const result = transformExpressiveCode(original);
    expect(result).not.toBe(original);
    expect(result.code).not.toBe(original.code);
  });

  test('transforms preserve other context properties', () => {
    const ctx = createContext({
      code: '<pre><code>test</code></pre>',
      source: '```\ntest\n```',
      filename: '/path/to/file.md',
      frontmatter: { title: 'Test' },
      headings: [{ text: 'Heading', depth: 1, slug: 'heading' }],
      config: {
        expressiveCode: { component: 'Code', moduleId: 'astro-expressive-code/components' },
        starlightComponents: false,
        shiki: null,
      },
    });
    const result = transformExpressiveCode(ctx);
    expect(result.source).toBe(ctx.source);
    expect(result.filename).toBe(ctx.filename);
    expect(result.frontmatter).toEqual(ctx.frontmatter);
    expect(result.headings).toEqual(ctx.headings);
  });
});
