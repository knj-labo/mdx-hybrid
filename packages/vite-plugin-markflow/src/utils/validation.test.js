import { describe, test, expect } from "bun:test";
import {
  stripHeadingsMeta,
  hasUnclosedFence,
  shouldBypassSource,
} from "./validation.js";

describe("stripHeadingsMeta", () => {
  test("returns code unchanged when no headings metadata", () => {
    const code = `import React from 'react';\n\nexport default () => <div>Hello</div>;`;
    expect(stripHeadingsMeta(code)).toBe(code);
  });

  test("removes export const headings", () => {
    const code = `export const headings = [{depth: 1, text: "Title"}];\n\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export const headings");
    expect(result).toContain("Content");
  });

  test("removes export function getHeadings", () => {
    const code = `export function getHeadings() { return []; }\n\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export function getHeadings");
    expect(result).toContain("Content");
  });

  test("removes both headings exports", () => {
    const code = `export const headings = [];\nexport function getHeadings() { return headings; }\n\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export const headings");
    expect(result).not.toContain("export function getHeadings");
    expect(result).toContain("Content");
  });

  test("handles multiline headings array", () => {
    const code = `export const headings = [\n  {depth: 1, text: "A"},\n  {depth: 2, text: "B"}\n];\n\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export const headings");
    expect(result).toContain("Content");
  });

  test("handles multiline getHeadings function", () => {
    const code = `export function getHeadings() {\n  return [\n    {depth: 1}\n  ];\n}\n\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export function getHeadings");
    expect(result).toContain("Content");
  });

  test("preserves other exports", () => {
    const code = `export const frontmatter = {};\nexport const headings = [];\nexport function MyComponent() {}`;
    const result = stripHeadingsMeta(code);
    expect(result).toContain("export const frontmatter");
    expect(result).not.toContain("export const headings");
    expect(result).toContain("export function MyComponent");
  });

  test("handles Windows line endings (CRLF)", () => {
    const code = `export const headings = [];\r\n\r\nContent`;
    const result = stripHeadingsMeta(code);
    expect(result).not.toContain("export const headings");
    expect(result).toContain("Content");
  });
});

describe("hasUnclosedFence", () => {
  test("returns false for markdown without code fences", () => {
    const source = "# Hello\n\nSome text";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("returns false for properly closed backtick fence", () => {
    const source = "```\ncode\n```";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("returns false for properly closed tilde fence", () => {
    const source = "~~~\ncode\n~~~";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("returns true for unclosed backtick fence", () => {
    const source = "```\ncode\nmore code";
    expect(hasUnclosedFence(source)).toBe(true);
  });

  test("returns true for unclosed tilde fence", () => {
    const source = "~~~\ncode\nmore code";
    expect(hasUnclosedFence(source)).toBe(true);
  });

  test("returns false for multiple properly closed fences", () => {
    const source = "```\ncode1\n```\n\n```\ncode2\n```";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("handles fence with language specifier", () => {
    const source = "```javascript\nconst x = 1;\n```";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("handles closing fence longer than opening", () => {
    const source = "```\ncode\n`````";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("returns true when closing fence is shorter than opening", () => {
    const source = "`````\ncode\n```";
    // Closing must be >= opening length
    expect(hasUnclosedFence(source)).toBe(true);
  });

  test("backticks and tildes don't close each other", () => {
    const source = "```\ncode\n~~~";
    expect(hasUnclosedFence(source)).toBe(true);
  });

  test("handles indented fences (up to 3 spaces)", () => {
    const source = "   ```\n   code\n   ```";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("ignores fences indented more than 3 spaces", () => {
    const source = "    ```\n    code\n    ```";
    // 4+ spaces = code block in markdown, not fence
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("handles fence with info string on closing line", () => {
    const source = "```\ncode\n```inner";
    // Closing fence can have info string - it closes the first fence
    // Then we're left with just "inner" text, no unclosed fence
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("handles empty lines between fence and content", () => {
    const source = "```\n\ncode\n\n```";
    expect(hasUnclosedFence(source)).toBe(false);
  });

  test("returns true for unclosed fence at end of file", () => {
    const source = "# Title\n\n```javascript\nconst x = 1;";
    expect(hasUnclosedFence(source)).toBe(true);
  });
});

describe("shouldBypassSource", () => {
  test("returns null for normal markdown", () => {
    const source = "# Hello\n\nSome text with `code` inline.";
    expect(shouldBypassSource(source)).toBe(null);
  });

  test("returns reason for inline Code component in backticks", () => {
    const source = "Example: `<Code>hello</Code>`";
    expect(shouldBypassSource(source)).toBe("inline component code sample");
  });

  test("returns reason for inline Prism component", () => {
    const source = "Example: `<Prism language='js'>code</Prism>`";
    expect(shouldBypassSource(source)).toBe("inline component code sample");
  });

  test("returns reason for Code with whitespace", () => {
    const source = "Example: `< Code>hello</Code>`";
    expect(shouldBypassSource(source)).toBe("inline component code sample");
  });

  test("does not match Code without backticks", () => {
    const source = "<Code>hello</Code>";
    expect(shouldBypassSource(source)).toBe(null);
  });

  test("returns reason for unclosed code fence", () => {
    const source = "```javascript\nconst x = 1;\n// missing closing fence";
    expect(shouldBypassSource(source)).toBe("unclosed code fence");
  });

  test("returns first matched reason when multiple issues", () => {
    const source = "`<Code>test</Code>`\n\n```\nunclosed";
    // Code check comes first
    expect(shouldBypassSource(source)).toBe("inline component code sample");
  });

  test("returns null when no bypass conditions met", () => {
    const source = "# Title\n\n```js\ncode\n```\n\nSome text.";
    expect(shouldBypassSource(source)).toBe(null);
  });

  test("handles complex markdown with inline code", () => {
    const source =
      "Use the `import` statement, not `<Code>` or `<Prism>` directly.";
    expect(shouldBypassSource(source)).toBe("inline component code sample");
  });
});
