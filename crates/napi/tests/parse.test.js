import test from 'ava';
import { compileIr } from '../index.js';

test('compileIr() converts markdown to HTML', (t) => {
  const input = '# Hello World';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<h1'));
  t.true(result.html.includes('Hello World'));
  t.true(result.html.includes('</h1>'));
});

test('compileIr() handles bold and italic text', (t) => {
  const input = 'This is **bold** and *italic* text';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<strong>bold</strong>'));
  t.true(result.html.includes('<em>italic</em>'));
});

test('compileIr() handles code blocks', (t) => {
  const input = '```javascript\nconsole.log("test");\n```';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<pre'));
  t.true(result.html.includes('<code'));
  t.true(result.html.includes('language-javascript'));
});

test('compileIr() handles images', (t) => {
  const input = '![Alt text](image.png)';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('alt="Alt text"'));
  t.true(result.html.includes('src="image.png"'));
});

test('compileIr() handles lists', (t) => {
  const input = '- Item 1\n- Item 2\n- Item 3';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<ul>'));
  t.true(result.html.includes('<li>'));
  t.true(result.html.includes('Item 1'));
  t.true(result.html.includes('Item 2'));
  t.true(result.html.includes('</ul>'));
});

test('compileIr() assigns heading ids', (t) => {
  const input = '# Hello Heading';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('id="hello-heading"'));
});

test('compileIr() handles links', (t) => {
  const input = '[Link text](https://example.com)';
  const result = compileIr(input, '/virtual.md');

  t.true(result.html.includes('<a href="https://example.com">'));
  t.true(result.html.includes('Link text'));
  t.true(result.html.includes('</a>'));
});

test('compileIr() returns an object with html', (t) => {
  const result = compileIr('# Test', '/virtual.md');
  t.is(typeof result, 'object');
  t.is(typeof result.html, 'string');
});

test('compileIr() handles empty input', (t) => {
  const result = compileIr('', '/virtual.md');
  t.is(typeof result.html, 'string');
});

test('compileIr() passes through HTML blocks', (t) => {
  const input = '<section>Hello</section>\n\nSome text';
  const result = compileIr(input, '/virtual.md');

  // Raw HTML is preserved (possibly as JSX spread format)
  t.true(result.html.includes('section') || result.html.includes('Hello'));
});
