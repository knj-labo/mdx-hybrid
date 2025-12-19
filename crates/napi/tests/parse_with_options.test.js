import test from 'ava';
import { parseWithOptions } from '../index.js';

test('parseWithOptions rewrites directives into Aside', async (t) => {
  const input = ':::note[Hi]\nBody\n:::';
  const html = await parseWithOptions(input, { enforceImgLoadingLazy: true });

  t.true(html.includes('<Aside type="note" title="Hi">'));
  t.true(html.includes('</Aside>'));
});
