import test from 'ava';
import { compileIr, FileInputType } from '../index.js';

test('compileIr() handles images', (t) => {
  const input = '![alt](image.png)';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('alt="alt"'));
  t.true(result.html.includes('src="image.png"'));
});

test('compileIr() handles raw HTML img tags', (t) => {
  const input = '<img src="/hero.png" />';
  const result = compileIr(input, '/virtual.md');

  // Raw HTML is converted to JSX spread format
  t.true(result.html.includes('img'));
  t.true(result.html.includes('/hero.png'));
});

test('compileIr() with url option sets the url in result', (t) => {
  const input = '# Test';
  const result = compileIr(input, '/virtual.md', { url: '/test-page' });

  t.is(result.url, '/test-page');
});

test('compileIr() with fileType option can force MDX mode', (t) => {
  // Even with .md extension, can force MDX processing
  const input = '# Header\n\n**Bold** text';
  const result = compileIr(input, '/virtual.md', { fileType: FileInputType.Mdx });

  t.true(result.html.includes('<h1'));
  t.true(result.html.includes('Header'));
  t.true(result.html.includes('<strong>Bold</strong>'));
});

test('compileIr() converts markdown to HTML correctly', (t) => {
  const input = '# Header\n\n**Bold** text';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<h1'));
  t.true(result.html.includes('Header'));
  t.true(result.html.includes('<strong>Bold</strong>'));
});

test('compileIr() returns an object with html property', (t) => {
  const result = compileIr('# Test', '/virtual.md');
  t.is(typeof result, 'object');
  t.is(typeof result.html, 'string');
});
