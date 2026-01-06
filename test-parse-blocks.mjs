#!/usr/bin/env node
import { parseBlocks } from './crates/napi/index.js';

console.log('=== Testing parseBlocks() NAPI binding ===\n');

// Test 1: Simple HTML block
console.log('Test 1: Simple paragraph');
const test1 = parseBlocks('Hello, **world**!', { enable_directives: false });
console.log(JSON.stringify(test1, null, 2));
console.log('✓ Expected: HTML block with <p> and <strong> tags\n');

// Test 2: Table with GFM support
console.log('Test 2: Table rendering');
const test2 = parseBlocks(`| Name | Age |
| --- | --- |
| Alice | 30 |
| Bob | 25 |`, { enable_directives: false });
console.log(JSON.stringify(test2, null, 2));
console.log('✓ Expected: HTML block with <table> structure\n');

// Test 3: Strikethrough
console.log('Test 3: Strikethrough');
const test3 = parseBlocks('This is ~~deleted~~ text.', { enable_directives: false });
console.log(JSON.stringify(test3, null, 2));
console.log('✓ Expected: HTML block with <del> tag\n');

// Test 4: Directive to Component
console.log('Test 4: Directive to Component block');
const test4 = parseBlocks(`:::note[Important]
This is **bold** text.
:::`, { enable_directives: true });
console.log(JSON.stringify(test4, null, 2));
console.log('✓ Expected: Component block with name="note", props={title: "Important"}\n');

// Test 5: Mixed content
console.log('Test 5: Mixed HTML and Component');
const test5 = parseBlocks(`# Hello

This is a paragraph.

:::tip[Pro Tip]
Use GFM tables!
:::

| Feature | Status |
| --- | --- |
| Tables | ✓ |
| ~~Strikethrough~~ | ✓ |`, { enable_directives: true });
console.log(JSON.stringify(test5, null, 2));
console.log('✓ Expected: Multiple blocks (HTML + Component + HTML)\n');

console.log('=== All tests completed! ===');
