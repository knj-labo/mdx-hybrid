#!/usr/bin/env node

// Test the preprocessing stage to see what JSX markers are generated
const input = `---
title: Test
---

# Main Heading

Regular paragraph.

:::note[Title]
Content inside directive.
:::

After directive.
`;

console.log('Input:');
console.log(input);
console.log('\n' + '='.repeat(60) + '\n');

// Simulate what preprocess_directives does
let output = '';
let inFence = false;
let directiveStack = [];

const lines = input.split('\n');

for (const line of lines) {
  // Check for fence
  if (line.trimStart().startsWith('```')) {
    inFence = !inFence;
    output += line + '\n';
    continue;
  }

  if (inFence) {
    output += line + '\n';
    continue;
  }

  // Check for directive opening: :::name[title] or :::name
  const directiveMatch = line.match(/^:::([\w-]+)(?:\[(.*?)\])?(?:\s+(.*))?$/);
  if (directiveMatch) {
    const name = directiveMatch[1];
    const title = directiveMatch[2];
    const attrs = directiveMatch[3] || '';

    directiveStack.push(name);

    output += `<mf-directive-start name="${name}"`;
    if (title) {
      output += ` title="${title.replace(/"/g, '\\"')}"`;
    }
    if (attrs) {
      output += ` attrs="${attrs.replace(/"/g, '\\"')}"`;
    }
    output += ' />\n';
    continue;
  }

  // Check for directive closing: :::
  if (line.trim() === ':::' && directiveStack.length > 0) {
    directiveStack.pop();
    output += '<mf-directive-end />\n';
    continue;
  }

  output += line + '\n';
}

console.log('Preprocessed output:');
console.log(output);
