import { describe, it, expect } from 'bun:test';
import { blocksToJsx, type Block } from './blocks-to-jsx.js';
import { createRegistry, starlightLibrary } from 'markflow/registry';

describe('blocksToJsx', () => {
  describe('user imports', () => {
    it('should include user imports in output', () => {
      const blocks: Block[] = [
        { type: 'html', content: '<p>Hello</p>' },
      ];
      const userImports = ["import Card from '~/components/Card.astro';"];

      const result = blocksToJsx(blocks, {}, [], null, undefined, userImports);

      expect(result).toContain("import Card from '~/components/Card.astro';");
    });

    it('should skip registry imports for user-imported components', () => {
      const registry = createRegistry([starlightLibrary]);

      const blocks: Block[] = [
        { type: 'component', name: 'Card', props: {}, slotHtml: '<p>Content</p>' },
      ];
      const userImports = ["import Card from '~/components/Landing/Card.astro';"];

      const result = blocksToJsx(blocks, {}, [], registry, undefined, userImports);

      // Should include user import
      expect(result).toContain("import Card from '~/components/Landing/Card.astro';");
      // Should NOT include registry import for Card
      expect(result).not.toContain('@astrojs/starlight/components');
    });

    it('should generate registry imports for non-user-imported components', () => {
      const registry = createRegistry([starlightLibrary]);

      const blocks: Block[] = [
        { type: 'component', name: 'Card', props: {}, slotHtml: '<p>Card Content</p>' },
        { type: 'component', name: 'Aside', props: {}, slotHtml: '<p>Aside Content</p>' },
      ];
      // Only Card is user-imported
      const userImports = ["import Card from '~/components/Card.astro';"];

      const result = blocksToJsx(blocks, {}, [], registry, undefined, userImports);

      // User import for Card
      expect(result).toContain("import Card from '~/components/Card.astro';");
      // Registry import for Aside (since it's not user-imported)
      expect(result).toContain("import { Aside } from '@astrojs/starlight/components';");
    });

    it('should handle multiple user imports', () => {
      const blocks: Block[] = [
        { type: 'component', name: 'Card', props: {} },
        { type: 'component', name: 'Button', props: {} },
      ];
      const userImports = [
        "import Card from '~/components/Card.astro';",
        "import Button from '~/components/Button.astro';",
      ];

      const result = blocksToJsx(blocks, {}, [], null, undefined, userImports);

      expect(result).toContain("import Card from '~/components/Card.astro';");
      expect(result).toContain("import Button from '~/components/Button.astro';");
    });

    it('should handle named imports in user imports', () => {
      const registry = createRegistry([starlightLibrary]);

      const blocks: Block[] = [
        { type: 'component', name: 'Aside', props: {} },
      ];
      // User provides named import for Aside
      const userImports = ["import { Aside } from './my-components';"];

      const result = blocksToJsx(blocks, {}, [], registry, undefined, userImports);

      // Should include user import
      expect(result).toContain("import { Aside } from './my-components';");
      // Should NOT include registry import for Aside
      expect(result).not.toContain('@astrojs/starlight/components');
    });

    it('should handle aliased imports in user imports', () => {
      const registry = createRegistry([starlightLibrary]);

      const blocks: Block[] = [
        { type: 'component', name: 'MyCard', props: {} },
      ];
      // User imports Card as MyCard
      const userImports = ["import { Card as MyCard } from './my-components';"];

      const result = blocksToJsx(blocks, {}, [], registry, undefined, userImports);

      // Should include user import
      expect(result).toContain("import { Card as MyCard } from './my-components';");
    });

    it('should default to empty user imports when not provided', () => {
      const blocks: Block[] = [
        { type: 'html', content: '<p>Hello</p>' },
      ];

      // Call without userImports parameter
      const result = blocksToJsx(blocks, {}, [], null, undefined);

      // Should generate valid output without errors
      expect(result).toContain('export const frontmatter');
      expect(result).toContain('export default MarkflowContent');
    });
  });

  describe('basic functionality', () => {
    it('should generate valid JSX module for HTML blocks', () => {
      const blocks: Block[] = [
        { type: 'html', content: '<p>Hello World</p>' },
      ];

      const result = blocksToJsx(blocks);

      expect(result).toContain('export const frontmatter');
      expect(result).toContain('export function getHeadings()');
      expect(result).toContain('export const Content');
      expect(result).toContain('export default MarkflowContent');
    });

    it('should use set:html for HTML content', () => {
      const blocks: Block[] = [
        { type: 'html', content: '<p>Test</p>' },
      ];

      const result = blocksToJsx(blocks);

      expect(result).toContain('set:html=');
    });

    it('should include runtime imports', () => {
      const blocks: Block[] = [];

      const result = blocksToJsx(blocks);

      expect(result).toContain("import { createComponent, renderJSX } from 'astro/runtime/server/index.js';");
      expect(result).toContain("import { Fragment as _Fragment, jsx as _jsx } from 'astro/jsx-runtime';");
    });
  });

  describe('nested components', () => {
    it('should embed nested components directly without set:html', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'CardGrid',
          props: {},
          // Use HTML-style attributes (what the Rust renderer produces)
          slotHtml: '<Card title="Getting Started">Content here</Card>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly, not use set:html
      expect(result).toContain('<CardGrid><Card title="Getting Started">Content here</Card></CardGrid>');
      expect(result).not.toContain('set:html={');
    });

    it('should use set:html for pure HTML slot content', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Card',
          props: {},
          slotHtml: '<p>Hello <strong>world</strong></p>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should use set:html for HTML content
      expect(result).toContain('set:html=');
      expect(result).toContain('<Card><_Fragment set:html=');
    });

    it('should use set:html for uppercase HTML tags (not components)', () => {
      // Uppercase HTML tags like <SVG>, <DIV> should NOT be treated as components
      // Only true PascalCase (uppercase followed by lowercase) should be components
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Container',
          props: {},
          slotHtml: '<SVG><path d="M0 0h24v24H0z"/></SVG>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should use set:html because <SVG> is not a PascalCase component
      expect(result).toContain('set:html=');
      expect(result).toContain('<Container><_Fragment set:html=');
    });

    it('should detect acronym-prefixed PascalCase components like MDXProvider', () => {
      // Components that start with acronyms like MDX, URL, API should be detected
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Container',
          props: {},
          slotHtml: '<MDXProvider>content</MDXProvider>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly, not use set:html
      expect(result).toContain('<Container><MDXProvider>content</MDXProvider></Container>');
      expect(result).not.toContain('set:html={');
    });

    it('should detect URLTable and other acronym-prefixed components', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Section',
          props: {},
          slotHtml: '<URLTable /><APIClient>data</APIClient>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly because these are PascalCase components
      expect(result).toContain('<Section><URLTable /><APIClient>data</APIClient></Section>');
      expect(result).not.toContain('set:html={');
    });

    it('should still use set:html for all-uppercase tags like HTML, DIV', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Container',
          props: {},
          slotHtml: '<DIV>content</DIV><HTML><BODY></BODY></HTML>',
        },
      ];

      const result = blocksToJsx(blocks);

      // All-uppercase should use set:html path
      expect(result).toContain('set:html=');
      expect(result).toContain('<Container><_Fragment set:html=');
    });

    it('should handle multiple nested components', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'CardGrid',
          props: {},
          // Use HTML-style attributes (what the Rust renderer produces)
          slotHtml: '<Card title="First">First card</Card><Card title="Second">Second card</Card>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly
      expect(result).toContain('<CardGrid><Card title="First">First card</Card><Card title="Second">Second card</Card></CardGrid>');
      expect(result).not.toContain('set:html={');
    });

    it('should handle self-closing nested components', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Container',
          props: {},
          slotHtml: '<Icon name="star" />',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly for self-closing component
      expect(result).toContain('<Container><Icon name="star" /></Container>');
      expect(result).not.toContain('set:html={');
    });

    it('should handle mixed HTML and component content', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Section',
          props: {},
          slotHtml: '<p>Intro text</p><Card>Content</Card><p>More text</p>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Should embed JSX directly because it contains a component
      expect(result).toContain('<Section><p>Intro text</p><Card>Content</Card><p>More text</p></Section>');
      expect(result).not.toContain('set:html={');
    });

    it('should convert HTML entities to JSX expressions in nested component content', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Card',
          props: {},
          // HTML entities that would appear literally in JSX
          slotHtml: '<Badge>a &lt; b &amp;&amp; c</Badge>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Entities should become JSX expressions: &lt; becomes {"<"}, &amp; becomes {"&"}
      expect(result).toContain('<Card><Badge>a {"<"} b {"&"}{"&"} c</Badge></Card>');
      // Should NOT contain the encoded entities
      expect(result).not.toContain('&lt;');
      expect(result).not.toContain('&amp;');
      // Should NOT decode to raw characters (that would break JSX)
      expect(result).not.toContain('<Card><Badge>a < b && c</Badge></Card>');
    });

    it('should convert literal ampersands to JSX expressions in nested component content', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Card',
          props: {},
          // Literal & character (not encoded as entity)
          slotHtml: '<Badge>Languages & Frameworks</Badge>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Literal & should become JSX expression
      expect(result).toContain('Languages {"&"} Frameworks');
      // Should NOT contain raw & (that would break JSX)
      expect(result).not.toContain('Languages & Frameworks');
    });

    it('should preserve unknown HTML entities in nested component content', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'Card',
          props: {},
          // Unknown entity like &nbsp; should be preserved
          slotHtml: '<Badge>Hello&nbsp;World</Badge>',
        },
      ];

      const result = blocksToJsx(blocks);

      // Unknown entities should be left as-is
      expect(result).toContain('&nbsp;');
    });

    it('should preserve valid JSX expressions in nested components', () => {
      const blocks: Block[] = [
        {
          type: 'component',
          name: 'CardGrid',
          props: {},
          // Valid JSX expression that should NOT be escaped
          slotHtml: '<Card title={title}>Content</Card>',
        },
      ];

      const result = blocksToJsx(blocks);

      // JSX expressions should be preserved, not escaped
      expect(result).toContain('title={title}');
      expect(result).not.toContain("{'{'}");
      expect(result).toContain('<CardGrid><Card title={title}>Content</Card></CardGrid>');
    });
  });
});
