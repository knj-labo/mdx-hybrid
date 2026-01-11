#!/usr/bin/env node
import { parseBlocks } from './crates/napi/index.js';

console.log('=== Testing Directive + JSX Coexistence ===\n');

// Test case: Both directives (:::note) and JSX (<Card>) in same document
const testInput = `---
title: Test Coexistence
---

# Main Heading

This is a regular paragraph.

:::note[Important Info]
This is a **directive** with markdown inside.

## Heading in Directive
:::

<Card title="My Card" icon="rocket">
  This is a **JSX component** with markdown inside.

  ## Heading in JSX
</Card>

:::tip
Another directive after the JSX component.
:::

Final paragraph.
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

  console.log('Extracted headings:');
  console.log(JSON.stringify(headings, null, 2));
  console.log('\n' + '='.repeat(60) + '\n');

  // Check for success criteria
  const noteComponent = blocks.find(b => b.type === 'component' && b.name === 'note');
  const cardComponent = blocks.find(b => b.type === 'component' && b.name === 'Card');
  const tipComponent = blocks.find(b => b.type === 'component' && b.name === 'tip');

  const hasBoldInNote = noteComponent?.slotHtml?.includes('<strong>directive</strong>');
  const hasBoldInCard = cardComponent?.slotHtml?.includes('<strong>JSX component</strong>');
  const hasMainHeading = headings.some(h => h.text === 'Main Heading');
  const hasDirectiveHeading = headings.some(h => h.text === 'Heading in Directive');
  const hasJsxHeading = headings.some(h => h.text === 'Heading in JSX');

  console.log('Verification:');
  console.log(`✓ Note directive block: ${noteComponent ? 'YES' : 'NO'}`);
  console.log(`✓ Card JSX block: ${cardComponent ? 'YES' : 'NO'}`);
  console.log(`✓ Tip directive block: ${tipComponent ? 'YES' : 'NO'}`);
  console.log(`✓ Bold text in directive: ${hasBoldInNote ? 'YES' : 'NO'}`);
  console.log(`✓ Bold text in JSX: ${hasBoldInCard ? 'YES' : 'NO'}`);
  console.log(`✓ Main heading in TOC: ${hasMainHeading ? 'YES' : 'NO'}`);
  console.log(`✓ Directive heading in TOC: ${hasDirectiveHeading ? 'YES' : 'NO'}`);
  console.log(`✓ JSX heading in TOC: ${hasJsxHeading ? 'YES' : 'NO'}`);

  if (noteComponent && cardComponent && tipComponent &&
      hasBoldInNote && hasBoldInCard &&
      hasMainHeading && hasDirectiveHeading && hasJsxHeading) {
    console.log('\n✅ Directive + JSX coexistence is working correctly!');
    process.exit(0);
  } else {
    console.log('\n❌ Directive + JSX coexistence has issues');
    process.exit(1);
  }
} catch (error) {
  console.error('❌ Error:', error.message || error);
  console.error(error.stack);
  process.exit(1);
}
