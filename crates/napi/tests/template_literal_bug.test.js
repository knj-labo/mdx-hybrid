import test from 'ava';
import { createCompiler } from '../index.js';

const compiler = createCompiler();

test('inline code with template literal syntax should not be evaluated', async (t) => {
  // This is the pattern that causes runtime errors:
  // The ${variable} inside backticks should be treated as literal text
  const source = 'Use `style={`--myVar:${value}`}` to set the style.';
  const result = await compiler.compile(source, '/virtual.mdx');

  // The output should NOT contain an actual template literal that would
  // evaluate ${value} as a JavaScript expression
  // It should contain escaped or quoted content
  t.false(
    result.code.includes('`--myVar:${value}`'),
    'Template literal should not be present as raw backticks in output'
  );

  // The content should be safely rendered as a string
  t.true(
    result.code.includes('<code>'),
    'Should contain code element'
  );
});

test('inline code with dollar sign should be safe in JSX', async (t) => {
  const source = 'Use `${variable}` in template literals.';
  const result = await compiler.compile(source, '/virtual.mdx');

  // Should not cause reference errors when evaluated
  // The ${variable} should be escaped or rendered as text
  t.true(
    result.code.includes('<code>'),
    'Should contain code element'
  );
});

test('double backtick inline code preserves content', async (t) => {
  // Double backticks are used to include backticks in inline code
  const source = 'Use `` `template` `` for templates.';
  const result = await compiler.compile(source, '/virtual.mdx');

  t.true(
    result.code.includes('<code>'),
    'Should contain code element'
  );
});

test('REPRO: double backtick with template literal causes runtime error', async (t) => {
  // This is the exact pattern from astro-syntax.mdx that causes:
  // "value is not defined" runtime error
  const source = 'then you can manually add a ``style={`--myVar:${value}`}`` to your Element.';
  const result = await compiler.compile(source, '/virtual.mdx');

  console.log('Generated code:', result.code);

  // The output should NOT contain an unescaped template literal
  // that would cause ${value} to be evaluated
  t.false(
    /`[^`]*\$\{value\}[^`]*`/.test(result.code),
    'Template literal should be escaped or quoted, not raw'
  );

  t.true(
    result.code.includes('<code>'),
    'Should contain code element'
  );
});
