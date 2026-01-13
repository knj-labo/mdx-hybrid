#!/usr/bin/env node
/**
 * Fix MDX indentation issues for markdown-rs compatibility.
 *
 * This script fixes:
 * 1. Code fence closing indentation not matching opening
 * 2. Mixed tabs/spaces in code fence indentation (converts to spaces)
 * 3. JSX closing tags with different indentation than opening
 *
 * Usage: node scripts/fix-mdx-indentation.mjs [directory]
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const TAB_SIZE = 4;

function countLeadingSpaces(line) {
  let count = 0;
  for (const char of line) {
    if (char === ' ') count++;
    else if (char === '\t') count += TAB_SIZE;
    else break;
  }
  return count;
}

function getLeadingWhitespace(line) {
  const match = line.match(/^[\t ]*/);
  return match ? match[0] : '';
}

function normalizeIndent(whitespace, targetSpaces) {
  return ' '.repeat(targetSpaces);
}

function fixCodeFenceIndentation(content) {
  const lines = content.split('\n');
  const result = [];
  let inCodeBlock = false;
  let codeBlockIndent = 0;
  let codeBlockStartLine = -1;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!inCodeBlock && trimmed.startsWith('```')) {
      // Opening fence
      inCodeBlock = true;
      codeBlockIndent = countLeadingSpaces(line);
      codeBlockStartLine = i;

      // Normalize to spaces if mixed tabs/spaces
      const whitespace = getLeadingWhitespace(line);
      if (whitespace.includes('\t')) {
        result.push(normalizeIndent(whitespace, codeBlockIndent) + trimmed);
      } else {
        result.push(line);
      }
    } else if (inCodeBlock && trimmed === '```') {
      // Closing fence - ensure it matches opening indent
      const closingIndent = countLeadingSpaces(line);

      if (closingIndent !== codeBlockIndent) {
        // Fix indentation to match opening
        result.push(' '.repeat(codeBlockIndent) + '```');
      } else {
        // Normalize tabs to spaces if needed
        const whitespace = getLeadingWhitespace(line);
        if (whitespace.includes('\t')) {
          result.push(' '.repeat(codeBlockIndent) + '```');
        } else {
          result.push(line);
        }
      }
      inCodeBlock = false;
    } else {
      result.push(line);
    }
  }

  return result.join('\n');
}

function fixJsxTagIndentation(content) {
  const lines = content.split('\n');
  const result = [];
  const tagStack = []; // Stack of {name, indent, line}

  // JSX self-closing tags we care about
  const jsxComponentPattern = /^(\s*)<([A-Z][a-zA-Z0-9]*|Fragment)(\s|>|$)/;
  const jsxClosingPattern = /^(\s*)<\/([A-Z][a-zA-Z0-9]*|Fragment)>/;

  for (let i = 0; i < lines.length; i++) {
    let line = lines[i];

    // Check for opening JSX tags
    const openMatch = line.match(jsxComponentPattern);
    if (openMatch) {
      const indent = countLeadingSpaces(line);
      const tagName = openMatch[2];

      // Check if it's self-closing
      if (!line.includes('/>') && !line.includes(`</${tagName}>`)) {
        tagStack.push({ name: tagName, indent, line: i });
      }
    }

    // Check for closing JSX tags
    const closeMatch = line.match(jsxClosingPattern);
    if (closeMatch) {
      const tagName = closeMatch[2];
      const closingIndent = countLeadingSpaces(line);

      // Find matching opening tag
      for (let j = tagStack.length - 1; j >= 0; j--) {
        if (tagStack[j].name === tagName) {
          const openingIndent = tagStack[j].indent;

          if (closingIndent !== openingIndent) {
            // Fix the closing tag indentation
            const trimmed = line.trim();
            line = ' '.repeat(openingIndent) + trimmed;
          }

          tagStack.splice(j, 1);
          break;
        }
      }
    }

    result.push(line);
  }

  return result.join('\n');
}

function fixMdxFile(filePath) {
  let content = fs.readFileSync(filePath, 'utf8');
  const original = content;

  // Apply fixes
  content = fixCodeFenceIndentation(content);
  content = fixJsxTagIndentation(content);

  if (content !== original) {
    fs.writeFileSync(filePath, content, 'utf8');
    return true;
  }
  return false;
}

function walkDir(dir, callback) {
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const filePath = path.join(dir, file);
    const stat = fs.statSync(filePath);
    if (stat.isDirectory()) {
      walkDir(filePath, callback);
    } else if (file.endsWith('.mdx')) {
      callback(filePath);
    }
  }
}

// Main
const targetDir = process.argv[2] || path.join(__dirname, '../fixtures/integration/withastro-docs/repo/src/content/docs');

console.log(`Scanning for MDX files in: ${targetDir}`);

let totalFiles = 0;
let fixedFiles = 0;

walkDir(targetDir, (filePath) => {
  totalFiles++;
  try {
    if (fixMdxFile(filePath)) {
      fixedFiles++;
      console.log(`Fixed: ${path.relative(targetDir, filePath)}`);
    }
  } catch (err) {
    console.error(`Error processing ${filePath}: ${err.message}`);
  }
});

console.log(`\nDone! Scanned ${totalFiles} files, fixed ${fixedFiles} files.`);
