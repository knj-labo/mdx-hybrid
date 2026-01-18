import { describe, test, expect } from "bun:test";
import {
  decodeHtmlEntities,
  rewriteExpressiveCodeBlocks,
  injectExpressiveCodeComponent,
} from "./expressive-code.js";

describe("decodeHtmlEntities", () => {
  test("returns value as-is when null or empty", () => {
    expect(decodeHtmlEntities(null)).toBe(null);
    expect(decodeHtmlEntities("")).toBe("");
  });

  test("returns value as-is when no entities present", () => {
    expect(decodeHtmlEntities("hello world")).toBe("hello world");
  });

  test("decodes hex entities", () => {
    expect(decodeHtmlEntities("&#x41;")).toBe("A");
    expect(decodeHtmlEntities("&#x3c;div&#x3e;")).toBe("<div>");
  });

  test("decodes decimal entities", () => {
    expect(decodeHtmlEntities("&#65;")).toBe("A");
    expect(decodeHtmlEntities("&#60;div&#62;")).toBe("<div>");
  });

  test("decodes named entities", () => {
    expect(decodeHtmlEntities("&quot;hello&quot;")).toBe('"hello"');
    expect(decodeHtmlEntities("&#39;world&#39;")).toBe("'world'");
    expect(decodeHtmlEntities("&lt;div&gt;")).toBe("<div>");
    expect(decodeHtmlEntities("&amp;")).toBe("&");
  });

  test("decodes mixed entities", () => {
    expect(decodeHtmlEntities("&lt;div&#x3e;&#65;&amp;&quot;")).toBe(
      '<div>A&"',
    );
  });

  test("decodes multiple occurrences", () => {
    expect(decodeHtmlEntities("&lt;&lt;&lt;")).toBe("<<<");
    expect(decodeHtmlEntities("&#x41;&#x42;&#x43;")).toBe("ABC");
  });

  test("handles ampersand correctly (decoded last)", () => {
    expect(decodeHtmlEntities("&amp;lt;")).toBe("&lt;");
  });
});

describe("rewriteExpressiveCodeBlocks", () => {
  test("returns unchanged code when no code blocks", () => {
    const code = "# Hello\n\nSome text";
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe(code);
    expect(result.changed).toBe(false);
  });

  test("rewrites simple code block without language", () => {
    const code = '<pre><code>const x = 1;</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe('<Code code={"const x = 1;"} />');
    expect(result.changed).toBe(true);
  });

  test("rewrites code block with language", () => {
    const code = '<pre><code class="language-javascript">const x = 1;</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe(
      '<Code code={"const x = 1;"} lang="javascript" />',
    );
    expect(result.changed).toBe(true);
  });

  test("rewrites multiple code blocks", () => {
    const code =
      '<pre><code class="language-js">let a = 1;</code></pre>\n\n<pre><code class="language-ts">let b: number = 2;</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe(
      '<Code code={"let a = 1;"} lang="js" />\n\n<Code code={"let b: number = 2;"} lang="ts" />',
    );
    expect(result.changed).toBe(true);
  });

  test("decodes HTML entities in code content", () => {
    const code = '<pre><code>&lt;div&gt;&amp;&lt;/div&gt;</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe('<Code code={"<div>&</div>"} />');
    expect(result.changed).toBe(true);
  });

  test("uses custom component name", () => {
    const code = '<pre><code>hello</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "MyCode");
    expect(result.code).toBe('<MyCode code={"hello"} />');
    expect(result.changed).toBe(true);
  });

  test("handles multiline code", () => {
    const code = '<pre><code>line 1\nline 2\nline 3</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe('<Code code={"line 1\\nline 2\\nline 3"} />');
    expect(result.changed).toBe(true);
  });

  test("preserves code with special characters", () => {
    const code = '<pre><code>const str = "hello";</code></pre>';
    const result = rewriteExpressiveCodeBlocks(code, "Code");
    expect(result.code).toBe('<Code code={"const str = \\"hello\\";"} />');
    expect(result.changed).toBe(true);
  });
});

describe("injectExpressiveCodeComponent", () => {
  test("does not inject if already imported", () => {
    const code = `import { Code } from 'astro-expressive-code';\n\n# Hello`;
    const config = { component: "Code", moduleId: "astro-expressive-code" };
    const result = injectExpressiveCodeComponent(code, config);
    expect(result).toBe(code);
  });

  test("injects default Code import", () => {
    const code = `import { useState } from 'react';\n\n# Hello`;
    const config = {
      component: "Code",
      moduleId: "astro-expressive-code/components",
    };
    const result = injectExpressiveCodeComponent(code, config);
    expect(result).toContain(
      `import { Code } from 'astro-expressive-code/components';`,
    );
    expect(result).toContain(`import { useState } from 'react';`);
  });

  test("injects aliased Code import for custom component name", () => {
    const code = `import { useState } from 'react';\n\n# Hello`;
    const config = {
      component: "MyCode",
      moduleId: "astro-expressive-code/components",
    };
    const result = injectExpressiveCodeComponent(code, config);
    expect(result).toContain(
      `import { Code as MyCode } from 'astro-expressive-code/components';`,
    );
  });

  test("inserts after existing imports", () => {
    const code = `import { foo } from 'bar';\nimport { baz } from 'qux';\n\n# Content`;
    const config = { component: "Code", moduleId: "expressive-code" };
    const result = injectExpressiveCodeComponent(code, config);
    const lines = result.split("\n");
    const codeImportIndex = lines.findIndex((line) =>
      line.includes("import { Code }"),
    );
    const lastImportIndex = lines.findIndex((line) =>
      line.includes("import { baz }"),
    );
    expect(codeImportIndex).toBeGreaterThan(lastImportIndex);
  });

  test("inserts at beginning when no imports exist", () => {
    const code = `# Hello\n\nSome content`;
    const config = { component: "Code", moduleId: "expressive-code" };
    const result = injectExpressiveCodeComponent(code, config);
    expect(result).toMatch(/^import { Code } from 'expressive-code';/);
  });

  test("does not inject if custom component name already imported", () => {
    const code = `import { Code as MyCode } from 'somewhere';\n\n# Hello`;
    const config = { component: "MyCode", moduleId: "expressive-code" };
    const result = injectExpressiveCodeComponent(code, config);
    expect(result).toBe(code);
  });
});
