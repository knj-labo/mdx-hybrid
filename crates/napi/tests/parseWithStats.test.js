import test from 'ava';
import { parseWithStats } from '../index.js';

test('parseWithStats() returns an object with html and processingTimeMs', (t) => {
  const input = '# Test Heading';
  const result = parseWithStats(input);

  t.is(typeof result, 'object');
  t.true('html' in result);
  t.true('processingTimeMs' in result);
});

test('parseWithStats() html contains correct output', (t) => {
  const input = '# Test Heading';
  const result = parseWithStats(input);

  t.true(result.html.includes('<h1'));
  t.true(result.html.includes('Test Heading'));
});

test('parseWithStats() processingTimeMs is a number', (t) => {
  const input = '# Test';
  const result = parseWithStats(input);

  t.is(typeof result.processingTimeMs, 'number');
});

test('parseWithStats() processingTimeMs is non-negative', (t) => {
  const input = '# Test';
  const result = parseWithStats(input);

  t.true(result.processingTimeMs >= 0);
});

test('parseWithStats() html applies lazy loading by default', (t) => {
  const input = '![alt](image.png)';
  const result = parseWithStats(input);

  t.true(result.html.includes('loading="lazy"'));
});

test('parseWithStats() works with large input', (t) => {
  const input = '# Heading\n\n' + 'Lorem ipsum dolor sit amet. '.repeat(100);
  const result = parseWithStats(input);

  t.true(result.html.length > input.length);
  t.true(result.processingTimeMs >= 0);
});

test('parseWithStats() works with empty input', (t) => {
  const result = parseWithStats('');

  t.is(typeof result.html, 'string');
  t.true(result.processingTimeMs >= 0);
});
