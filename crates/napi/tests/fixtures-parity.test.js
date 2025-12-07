import test from 'ava'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { parse, parseWithOptions, parseWithStats } from '../index.js'

const fixturePath = resolve(process.cwd(), '../../fixtures/core/markdown/hello.md')
const markdown = readFileSync(fixturePath, 'utf8')

test('bindings produce consistent output for hello.md', (t) => {
  const html = parse(markdown)
  const htmlWithOptions = parseWithOptions(markdown, { enforceImgLoadingLazy: true })
  const stats = parseWithStats(markdown)

  t.is(htmlWithOptions, html)
  t.is(stats.html, html)
  t.true(stats.processingTimeMs >= 0)
})
