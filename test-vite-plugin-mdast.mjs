#!/usr/bin/env node
import { parseBlocks } from './crates/napi/index.js';

console.log('=== Testing Vite Plugin mdast Integration ===\n');

// Simulate what the Vite plugin will do
function blocksToJsx(blocks, frontmatter = {}) {
  const fragments = [];
  const componentImports = new Set();

  for (const block of blocks) {
    if (block.type === "html") {
      fragments.push(block.content);
    } else if (block.type === "component") {
      componentImports.add(block.name);
      const propsStr = block.props
        ? Object.entries(block.props)
            .map(([key, value]) => {
              if (typeof value === "string") {
                return `${key}="${value.replace(/"/g, '\\"')}"`;
              }
              return `${key}={${JSON.stringify(value)}}`;
            })
            .join(" ")
        : "";
      const openTag = propsStr ? `<${block.name} ${propsStr}>` : `<${block.name}>`;
      fragments.push(`${openTag}${block.slotHtml || ""}</${block.name}>`);
    }
  }

  const componentImportLines = Array.from(componentImports)
    .map((name) => `import ${name} from '@astrojs/starlight/components/${name}.astro';`)
    .join("\n");

  const frontmatterJson = JSON.stringify(frontmatter);
  const jsxContent = fragments.join("\n");

  return `${componentImportLines}
export const frontmatter = ${frontmatterJson};
export function getHeadings() { return []; }
export default function MarkflowContent() {
  return (
    <>
${jsxContent}
    </>
  );
}
`;
}

// Test case: Mixed content with directive
const testInput = `---
title: Test Page
description: Testing mdast pipeline
---

# Hello World

This is a **bold** paragraph.

:::note[Important]
This is a note with **bold** text.
:::

| Feature | Status |
| --- | --- |
| Tables | ✓ |
| Directives | ✓ |
`;

console.log('Test input:');
console.log(testInput);
console.log('\n' + '='.repeat(60) + '\n');

const blocks = parseBlocks(testInput, {
  inject_starlight_css: false,
  enable_directives: true,
});

console.log('Parsed blocks:');
console.log(JSON.stringify(blocks, null, 2));
console.log('\n' + '='.repeat(60) + '\n');

const jsxOutput = blocksToJsx(blocks, {
  title: 'Test Page',
  description: 'Testing mdast pipeline',
});

console.log('Generated JSX code:');
console.log(jsxOutput);
console.log('\n' + '='.repeat(60) + '\n');

console.log('✓ Integration test completed successfully!');
