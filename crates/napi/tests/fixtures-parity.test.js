import test from 'ava'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { compileIr } from '../index.js'

const fixturePath = resolve(process.cwd(), '../../fixtures/core/markdown/hello.md')
const markdown = readFileSync(fixturePath, 'utf8')

test('compileIr produces consistent output for hello.md', (t) => {
  const result = compileIr(markdown, '/hello.md')

  t.is(typeof result.html, 'string')
  t.true(result.html.length > 0)
  t.true(Array.isArray(result.headings))
})

test('compileIr with url option produces same html', (t) => {
  const result1 = compileIr(markdown, '/hello.md')
  const result2 = compileIr(markdown, '/hello.md', { url: '/test' })

  t.is(result1.html, result2.html)
  t.is(result2.url, '/test')
})
