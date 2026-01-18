import { describe, test, expect } from "bun:test";
import { blocksToJsx } from "./blocks-to-jsx.js";
import { createRegistry, starlightLibrary, astroLibrary } from "markflow/registry";

// Create test registry for registry-aware tests
const testRegistry = createRegistry([starlightLibrary, astroLibrary]);

describe("blocksToJsx", () => {
  test("returns minimal JSX module for empty blocks", () => {
    const result = blocksToJsx([]);
    expect(result).toContain("export const frontmatter = {};");
    expect(result).toContain("export function getHeadings()");
    expect(result).toContain("export default function MarkflowContent()");
    expect(result).toContain("return (");
    expect(result).toContain("<>");
    expect(result).toContain("</>");
  });

  test("handles single HTML block", () => {
    const blocks = [{ type: "html", content: "<p>Hello world</p>" }];
    const result = blocksToJsx(blocks);
    expect(result).toContain("<p>Hello world</p>");
  });

  test("handles multiple HTML blocks", () => {
    const blocks = [
      { type: "html", content: "<h1>Title</h1>" },
      { type: "html", content: "<p>Paragraph</p>" },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain("<h1>Title</h1>");
    expect(result).toContain("<p>Paragraph</p>");
  });

  test("handles component without props", () => {
    const blocks = [{ type: "component", name: "Card" }];
    const result = blocksToJsx(blocks);
    expect(result).toContain(
      "import Card from '@astrojs/starlight/components/Card.astro';",
    );
    expect(result).toContain("<Card></Card>");
  });

  test("handles component with string props", () => {
    const blocks = [
      { type: "component", name: "Badge", props: { text: "New" } },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain(
      "import Badge from '@astrojs/starlight/components/Badge.astro';",
    );
    expect(result).toContain('<Badge text="New"></Badge>');
  });

  test("handles component with non-string props", () => {
    const blocks = [
      {
        type: "component",
        name: "Steps",
        props: { count: 5, enabled: true },
      },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain(
      "import Steps from '@astrojs/starlight/components/Steps.astro';",
    );
    expect(result).toContain('<Steps count={5} enabled={true}></Steps>');
  });

  test("handles component with mixed prop types", () => {
    const blocks = [
      {
        type: "component",
        name: "MyComponent",
        props: { title: "Hello", count: 42, items: ["a", "b"] },
      },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain('<MyComponent title="Hello"');
    expect(result).toContain("count={42}");
    expect(result).toContain('items={["a","b"]}');
  });

  test("escapes quotes in string props", () => {
    const blocks = [
      {
        type: "component",
        name: "Quote",
        props: { text: 'She said "hello"' },
      },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain('text="She said \\"hello\\""');
  });

  test("handles component with slotHtml", () => {
    const blocks = [
      {
        type: "component",
        name: "Card",
        slotHtml: "<p>Card content</p>",
      },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain("<Card><p>Card content</p></Card>");
  });

  test("handles component with props and slotHtml", () => {
    const blocks = [
      {
        type: "component",
        name: "Card",
        props: { variant: "info" },
        slotHtml: "<p>Info card</p>",
      },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain('<Card variant="info"><p>Info card</p></Card>');
  });

  test("handles mixed HTML and component blocks", () => {
    const blocks = [
      { type: "html", content: "<h1>Title</h1>" },
      { type: "component", name: "Badge", props: { text: "New" } },
      { type: "html", content: "<p>Content</p>" },
    ];
    const result = blocksToJsx(blocks);
    expect(result).toContain("<h1>Title</h1>");
    expect(result).toContain('<Badge text="New"></Badge>');
    expect(result).toContain("<p>Content</p>");
  });

  test("deduplicates component imports", () => {
    const blocks = [
      { type: "component", name: "Card" },
      { type: "component", name: "Badge" },
      { type: "component", name: "Card" },
    ];
    const result = blocksToJsx(blocks);
    const cardImports = (result.match(/import Card from/g) || []).length;
    expect(cardImports).toBe(1);
  });

  test("exports frontmatter data", () => {
    const frontmatter = { title: "My Page", draft: false, count: 42 };
    const result = blocksToJsx([], frontmatter);
    expect(result).toContain(
      'export const frontmatter = {"title":"My Page","draft":false,"count":42};',
    );
  });

  test("exports headings data", () => {
    const headings = [
      { depth: 1, text: "Title", slug: "title" },
      { depth: 2, text: "Section", slug: "section" },
    ];
    const result = blocksToJsx([], {}, headings);
    expect(result).toContain("export function getHeadings()");
    expect(result).toContain('"depth":1');
    expect(result).toContain('"text":"Title"');
    expect(result).toContain('"slug":"title"');
  });

  test("generates complete valid JSX module", () => {
    const blocks = [{ type: "html", content: "<p>Test</p>" }];
    const frontmatter = { title: "Test" };
    const headings = [{ depth: 1, text: "Test", slug: "test" }];
    const result = blocksToJsx(blocks, frontmatter, headings);

    expect(result).toContain("export const frontmatter");
    expect(result).toContain("export function getHeadings()");
    expect(result).toContain("export default function MarkflowContent()");
    expect(result).toContain("return (");
    expect(result).toContain("<>");
    expect(result).toContain("<p>Test</p>");
    expect(result).toContain("</>");
  });

  test("handles empty slotHtml gracefully", () => {
    const blocks = [{ type: "component", name: "Card", slotHtml: "" }];
    const result = blocksToJsx(blocks);
    expect(result).toContain("<Card></Card>");
  });

  test("preserves order of blocks", () => {
    const blocks = [
      { type: "html", content: "<!-- first -->" },
      { type: "component", name: "A" },
      { type: "html", content: "<!-- second -->" },
      { type: "component", name: "B" },
      { type: "html", content: "<!-- third -->" },
    ];
    const result = blocksToJsx(blocks);
    const firstIndex = result.indexOf("<!-- first -->");
    const aIndex = result.indexOf("<A>");
    const secondIndex = result.indexOf("<!-- second -->");
    const bIndex = result.indexOf("<B>");
    const thirdIndex = result.indexOf("<!-- third -->");

    expect(firstIndex).toBeLessThan(aIndex);
    expect(aIndex).toBeLessThan(secondIndex);
    expect(secondIndex).toBeLessThan(bIndex);
    expect(bIndex).toBeLessThan(thirdIndex);
  });
});

describe("blocksToJsx with registry", () => {
  test("uses named imports for registered components", () => {
    const blocks = [{ type: "component", name: "Aside" }];
    const result = blocksToJsx(blocks, {}, [], testRegistry);
    expect(result).toContain(
      "import { Aside } from '@astrojs/starlight/components';",
    );
    expect(result).toContain("<Aside></Aside>");
  });

  test("uses named imports for Astro Code component", () => {
    const blocks = [{ type: "component", name: "Code" }];
    const result = blocksToJsx(blocks, {}, [], testRegistry);
    expect(result).toContain("import { Code } from 'astro/components';");
    expect(result).toContain("<Code></Code>");
  });

  test("groups named imports by module", () => {
    const blocks = [
      { type: "component", name: "Aside" },
      { type: "component", name: "Tabs" },
      { type: "component", name: "Code" },
    ];
    const result = blocksToJsx(blocks, {}, [], testRegistry);
    // Starlight components grouped
    expect(result).toContain("import { Aside, Tabs } from '@astrojs/starlight/components';");
    // Astro components separate
    expect(result).toContain("import { Code } from 'astro/components';");
  });

  test("maps directives to components via registry", () => {
    const blocks = [{ type: "component", name: "note" }];
    const result = blocksToJsx(blocks, {}, [], testRegistry);
    // note directive should map to Aside component with type prop
    expect(result).toContain("import { Aside } from '@astrojs/starlight/components';");
    expect(result).toContain('<Aside type="note"></Aside>');
  });

  test("applies injected props for directive mappings", () => {
    const blocks = [
      { type: "component", name: "tip", props: { title: "Pro Tip" } },
    ];
    const result = blocksToJsx(blocks, {}, [], testRegistry);
    expect(result).toContain('type="tip"');
    expect(result).toContain('title="Pro Tip"');
  });
});
