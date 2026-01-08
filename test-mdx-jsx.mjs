#!/usr/bin/env node
import { parseBlocks } from './crates/napi/index.js';

console.log('=== Testing MDX JSX Support ===\n');

// Test case: JSX with nested markdown
const testInput = `---
title: Test MDX
---

# Root Heading

<Card title="My Card" icon="rocket">
  This is **bold** text with a [link](https://example.com).

  ## Nested Heading

  And a list:
  - Item 1
  - Item 2
</Card>

Regular paragraph after component.
`;

console.log('Test input:');
console.log(testInput);
console.log('\n' + '='.repeat(60) + '\n');

try {
  const { blocks, headings } = parseBlocks(testInput, {
    inject_starlight_css: false,
    enable_directives: true,
  });

  console.log('Parsed blocks:');
  console.log(JSON.stringify(blocks, null, 2));
  console.log('\n' + '='.repeat(60) + '\n');

  console.log('Extracted headings (including nested in JSX):');
  console.log(JSON.stringify(headings, null, 2));
  console.log('\n' + '='.repeat(60) + '\n');

  // Check for success criteria
  const hasCardComponent = blocks.some(b => b.type === 'component' && b.name === 'Card');
  const hasBoldInCard = blocks.some(b => b.type === 'component' && b.slotHtml && b.slotHtml.includes('<strong>bold</strong>'));
  const hasNestedHeading = headings.some(h => h.text === 'Nested Heading');
  const hasRootHeading = headings.some(h => h.text === 'Root Heading');

  console.log('Verification:');
  console.log(`✓ Card component block: ${hasCardComponent ? 'YES' : 'NO'}`);
  console.log(`✓ Bold text in Card: ${hasBoldInCard ? 'YES' : 'NO'}`);
  console.log(`✓ Root heading in TOC: ${hasRootHeading ? 'YES' : 'NO'}`);
  console.log(`✓ Nested heading in TOC: ${hasNestedHeading ? 'YES' : 'NO'}`);

  if (hasCardComponent && hasBoldInCard && hasNestedHeading && hasRootHeading) {
    console.log('\n✅ MDX JSX support is working correctly!');
    process.exit(0);
  } else {
    console.log('\n❌ MDX JSX support has issues');
    process.exit(1);
  }
} catch (error) {
  console.error('❌ Error:', error.message || error);
  process.exit(1);
}
