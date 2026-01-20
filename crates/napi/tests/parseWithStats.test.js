import test from 'ava';
import { compileIr, compileBatch } from '../index.js';

test('compileIr() returns an object with html and headings', (t) => {
  const input = '# Test Heading';
  const result = compileIr(input, '/virtual.md');

  t.is(typeof result, 'object');
  t.true('html' in result);
  t.true('headings' in result);
});

test('compileIr() html contains correct output', (t) => {
  const input = '# Test Heading';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<h1'));
  t.true(result.html.includes('Test Heading'));
});

test('compileIr() headings contains heading metadata', (t) => {
  const input = '# Test\n\n## Subheading';
  const result = compileIr(input, '/virtual.md');

  t.true(Array.isArray(result.headings));
  t.is(result.headings.length, 2);
  t.is(result.headings[0].depth, 1);
  t.is(result.headings[0].text, 'Test');
  t.is(result.headings[1].depth, 2);
  t.is(result.headings[1].text, 'Subheading');
});

test('compileIr() includes filePath in result', (t) => {
  const input = '# Test';
  const result = compileIr(input, '/path/to/file.md');

  t.is(result.filePath, '/path/to/file.md');
});

test('compileIr() handles images', (t) => {
  const input = '![alt](image.png)';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('img'));
  t.true(result.html.includes('image.png'));
});

test('compileIr() works with large input', (t) => {
  const input = '# Heading\n\n' + 'Lorem ipsum dolor sit amet. '.repeat(100);
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.length > input.length);
});

test('compileIr() works with empty input', (t) => {
  const result = compileIr('', '/virtual.md');

  t.is(typeof result.html, 'string');
});

test('compileBatch() returns processing stats with timing', (t) => {
  const inputs = [
    { id: 'file1.md', source: '# Hello' },
    { id: 'file2.md', source: '# World' },
  ];
  const batchResult = compileBatch(inputs);

  t.is(typeof batchResult.stats, 'object');
  t.is(typeof batchResult.stats.processingTimeMs, 'number');
  t.true(batchResult.stats.processingTimeMs >= 0);
  t.is(batchResult.stats.total, 2);
  t.is(batchResult.stats.succeeded, 2);
});
