import test from 'ava';
import { parse, parseWithOptions, parseWithStats } from '../index.js';

test('parse() converts markdown to HTML', (t) => {
  const input = '# Hello World';
  const output = parse(input);

  t.true(output.includes('<h1'));
  t.true(output.includes('Hello World'));
  t.true(output.includes('</h1>'));
});

test('parse() handles bold and italic text', (t) => {
  const input = 'This is **bold** and *italic* text';
  const output = parse(input);

  t.true(output.includes('<strong>bold</strong>'));
  t.true(output.includes('<em>italic</em>'));
});

test('parse() handles code blocks', (t) => {
  const input = '```javascript\nconsole.log("test");\n```';
  const output = parse(input);

  t.true(output.includes('<pre>'));
  t.true(output.includes('<code'));
  t.true(output.includes('class="language-javascript"'));
});

test('parse() adds lazy loading to images by default', (t) => {
  const input = '![Alt text](image.png)';
  const output = parse(input);

  t.true(output.includes('loading="lazy"'));
  t.true(output.includes('alt="Alt text"'));
});

test('parse() handles lists', (t) => {
  const input = '- Item 1\n- Item 2\n- Item 3';
  const output = parse(input);

  t.true(output.includes('<ul>'));
  t.true(output.includes('<li>Item 1</li>'));
  t.true(output.includes('<li>Item 2</li>'));
  t.true(output.includes('</ul>'));
});

test('parse() assigns heading ids', (t) => {
  const input = '# Hello Heading';
  const output = parse(input);

  t.true(output.includes('id="hello-heading"'));
});

test('parse() handles links', (t) => {
  const input = '[Link text](https://example.com)';
  const output = parse(input);

  t.true(output.includes('<a href="https://example.com">'));
  t.true(output.includes('Link text'));
  t.true(output.includes('</a>'));
});

test('parse() returns a string', (t) => {
  const output = parse('# Test');
  t.is(typeof output, 'string');
});

test('parse() handles empty input', (t) => {
  const output = parse('');
  t.is(typeof output, 'string');
});

test('parse() passes through HTML blocks and math', (t) => {
  const input = '<section>Hello</section>\n\nInline $x$ and $$y$$';
  const output = parse(input);

  t.true(output.includes('<section>Hello</section>'));
  const matches = output.match(/math-inline/g) ?? [];
  t.true(matches.length >= 2);
});
