import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compile, compileSync } from '@jp-knj/mdx-hybrid-core'
import { compile as mdxCompile, compileSync as mdxCompileSync } from '@mdx-js/mdx'
import chalk from 'chalk'
import { table } from 'table'

const __dirname = dirname(fileURLToPath(import.meta.url))

const fixtures = {
  small: readFileSync(join(__dirname, 'fixtures', 'small.mdx'), 'utf-8'),
  medium: readFileSync(join(__dirname, 'fixtures', 'medium.mdx'), 'utf-8'),
  large: readFileSync(join(__dirname, 'fixtures', 'large.mdx'), 'utf-8'),
}

async function benchmark(name, content, iterations = 10) {
  console.log(chalk.blue(`\n📊 Benchmarking ${name} MDX file...`))
  console.log(chalk.gray(`File size: ${(Buffer.byteLength(content) / 1024).toFixed(2)} KB`))
  console.log(chalk.gray(`Lines: ${content.split('\n').length}`))

  const results = {
    js: [],
    rust: [],
    native: [],
  }

  // Warmup
  await compile(content, { engine: 'js' })
  await mdxCompile(content)

  try {
    compileSync(content, { engine: 'rust' })
  } catch (e) {
    console.log(chalk.yellow('⚠️  Rust engine not available, skipping Rust benchmarks'))
  }

  // Benchmark JS engine
  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    await compile(content, { engine: 'js' })
    results.js.push(performance.now() - start)
  }

  // Benchmark Rust engine (if available)
  try {
    for (let i = 0; i < iterations; i++) {
      const start = performance.now()
      compileSync(content, { engine: 'rust' })
      results.rust.push(performance.now() - start)
    }
  } catch (e) {
    // Rust not available
  }

  // Benchmark native @mdx-js/mdx for comparison
  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    await mdxCompile(content)
    results.native.push(performance.now() - start)
  }

  return results
}

function calculateStats(times) {
  if (times.length === 0) return null

  const sorted = [...times].sort((a, b) => a - b)
  const avg = times.reduce((a, b) => a + b, 0) / times.length
  const median = sorted[Math.floor(sorted.length / 2)]
  const min = sorted[0]
  const max = sorted[sorted.length - 1]

  return { avg, median, min, max }
}

async function runBenchmarks() {
  console.log(chalk.bold.green('🚀 MDX Hybrid Benchmark Suite'))
  console.log(chalk.gray('='.repeat(50)))

  const allResults = []

  for (const [name, content] of Object.entries(fixtures)) {
    const results = await benchmark(name, content)

    const jsStats = calculateStats(results.js)
    const rustStats = calculateStats(results.rust)
    const nativeStats = calculateStats(results.native)

    const tableData = [['Engine', 'Avg (ms)', 'Median (ms)', 'Min (ms)', 'Max (ms)', 'Speedup']]

    if (nativeStats) {
      tableData.push([
        'Native MDX',
        nativeStats.avg.toFixed(2),
        nativeStats.median.toFixed(2),
        nativeStats.min.toFixed(2),
        nativeStats.max.toFixed(2),
        '1.00×',
      ])
    }

    if (jsStats) {
      const speedup = nativeStats ? (nativeStats.avg / jsStats.avg).toFixed(2) : '-'
      tableData.push([
        'JS Engine',
        jsStats.avg.toFixed(2),
        jsStats.median.toFixed(2),
        jsStats.min.toFixed(2),
        jsStats.max.toFixed(2),
        `${speedup}×`,
      ])
    }

    if (rustStats) {
      const speedup = jsStats ? (jsStats.avg / rustStats.avg).toFixed(2) : '-'
      tableData.push([
        chalk.green('Rust Engine'),
        chalk.green(rustStats.avg.toFixed(2)),
        chalk.green(rustStats.median.toFixed(2)),
        chalk.green(rustStats.min.toFixed(2)),
        chalk.green(rustStats.max.toFixed(2)),
        chalk.green.bold(`${speedup}×`),
      ])
    }

    console.log(`\n${table(tableData)}`)

    allResults.push({
      name,
      jsAvg: jsStats?.avg || 0,
      rustAvg: rustStats?.avg || 0,
      speedup: rustStats && jsStats ? jsStats.avg / rustStats.avg : 0,
    })
  }

  // Summary
  console.log(chalk.bold.blue('\n📈 Summary'))
  console.log(chalk.gray('='.repeat(50)))

  const summaryData = [['File', 'JS (ms)', 'Rust (ms)', 'Speedup', 'Target']]

  const targets = {
    small: 5,
    medium: 6.25,
    large: 7.5,
  }

  for (const result of allResults) {
    const meetsTarget = result.speedup >= targets[result.name]
    const speedupText = result.rustAvg > 0 ? `${result.speedup.toFixed(2)}×` : 'N/A'
    const targetText = `${targets[result.name]}×`

    summaryData.push([
      result.name,
      result.jsAvg.toFixed(2),
      result.rustAvg > 0 ? result.rustAvg.toFixed(2) : 'N/A',
      meetsTarget ? chalk.green(speedupText) : chalk.yellow(speedupText),
      targetText,
    ])
  }

  console.log(table(summaryData))
}

// Generate large fixture if it doesn't exist
import { existsSync } from 'node:fs'
if (!existsSync(join(__dirname, 'fixtures', 'large.mdx'))) {
  console.log(chalk.yellow('Generating large.mdx fixture...'))
  await import('./generate-fixtures.js')
}

// Run benchmarks
runBenchmarks().catch(console.error)
