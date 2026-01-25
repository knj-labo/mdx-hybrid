import test from 'ava';
import { createCompiler } from '../index.js';

const compiler = createCompiler();

test('compile converts directive to Aside component', (t) => {
  const source = ':::note\nBody\n:::';
  const result = compiler.compile(source, '/virtual.mdx');

  // Directive should be converted to Aside component with type in spread props
  t.true(result.code.includes('<Aside'));
  t.true(result.code.includes('"type": "note"'));
});

test('compile preserves existing Aside import without duplication', (t) => {
  const source = "import { Aside } from '@astrojs/starlight/components';\n\n:::note\nBody\n:::";
  const result = compiler.compile(source, '/virtual.mdx');

  // User-provided import should be preserved
  t.true(result.code.includes("import { Aside } from '@astrojs/starlight/components';"));

  // Should not be duplicated
  const occurrences = result.code.split("import { Aside } from '@astrojs/starlight/components';").length - 1;
  t.is(occurrences, 1);
});
