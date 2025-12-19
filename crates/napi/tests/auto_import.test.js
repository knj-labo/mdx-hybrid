import test from 'ava';
import { createCompiler } from '../index.js';

const compiler = createCompiler();

test('compile injects Aside import when directive is present', async (t) => {
  const source = ':::note\nBody\n:::';
  const result = await compiler.compile(source, '/virtual.mdx');

  t.true(result.code.includes("import { Aside } from '@astrojs/starlight/components';"));
});

test('compile does not duplicate existing Aside import', async (t) => {
  const source = "import { Aside } from '@astrojs/starlight/components';\n\n:::note\nBody\n:::";
  const result = await compiler.compile(source, '/virtual.mdx');

  const occurrences = result.code.split("import { Aside } from '@astrojs/starlight/components';").length - 1;
  t.is(occurrences, 1);
});
