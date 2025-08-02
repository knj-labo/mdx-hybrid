import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { compile, createCompiler } from '@mdx-hybrid/core'

const __dirname = dirname(fileURLToPath(import.meta.url))

function generateMDX(lines) {
  const sections = []

  sections.push('# Benchmark MDX File')
  sections.push('\nThis file is generated for benchmarking purposes.\n')

  for (let i = 0; i < lines / 10; i++) {
    sections.push(`## Section ${i + 1}`)
    sections.push('\nLorem ipsum dolor sit amet, consectetur adipiscing elit.')
    sections.push('\n```javascript')
    sections.push(`const value${i} = ${i * 10};`)
    sections.push(`console.log('Value:', value${i});`)
    sections.push('```\n')
    sections.push(`<div className="section-${i}">`)
    sections.push(`  <p>This is paragraph ${i + 1}</p>`)
    sections.push('</div>\n')
  }

  return sections.join('\n')
}

async function benchmark() {
  console.log('📊 MDX Hybrid Benchmark\n')

  const compiler = createCompiler()
  const sizes = [
    { name: 'Small', lines: 10 },
    { name: 'Medium', lines: 100 },
    { name: 'Large', lines: 1000 },
  ]

  const results = []

  for (const size of sizes) {
    console.log(`\n📏 Testing ${size.name} file (${size.lines} lines)...`)
    const content = generateMDX(size.lines)

    // Test JS engine
    const jsRuns = []
    for (let i = 0; i < 5; i++) {
      const start = performance.now()
      await compile(content, { engine: 'js' })
      jsRuns.push(performance.now() - start)
    }
    const jsAvg = jsRuns.reduce((a, b) => a + b) / jsRuns.length

    // Test Rust engine if available
    let rustAvg = null
    try {
      const rustRuns = []
      for (let i = 0; i < 5; i++) {
        const start = performance.now()
        await compile(content, { engine: 'rust' })
        rustRuns.push(performance.now() - start)
      }
      rustAvg = rustRuns.reduce((a, b) => a + b) / rustRuns.length
    } catch (error) {
      console.log('  ⚠️  Rust engine not available')
    }

    results.push({
      size: size.name,
      lines: size.lines,
      js: jsAvg,
      rust: rustAvg,
    })

    console.log(`  JS Engine:   ${jsAvg.toFixed(2)}ms`)
    if (rustAvg) {
      console.log(`  Rust Engine: ${rustAvg.toFixed(2)}ms`)
      console.log(`  Speedup:     ${(jsAvg / rustAvg).toFixed(2)}x`)
    }
  }

  // Display results table
  console.log('\n📈 Benchmark Results:')
  console.log('┌─────────┬───────┬──────────┬──────────┬─────────┐')
  console.log('│ Size    │ Lines │ JS (ms)  │ Rust (ms)│ Speedup │')
  console.log('├─────────┼───────┼──────────┼──────────┼─────────┤')

  for (const result of results) {
    const js = result.js.toFixed(2).padStart(8)
    const rust = result.rust ? result.rust.toFixed(2).padStart(8) : '    N/A'
    const speedup = result.rust ? `${(result.js / result.rust).toFixed(2)}x`.padStart(7) : '    N/A'
    console.log(
      `│ ${result.size.padEnd(7)} │ ${String(result.lines).padStart(5)} │ ${js} │ ${rust} │ ${speedup} │`
    )
  }

  console.log('└─────────┴───────┴──────────┴──────────┴─────────┘')

  // Check if we're meeting performance targets
  console.log('\n🎯 Performance Targets:')
  const info = compiler.getEngineInfo()
  console.log(`  JS Engine:   ${info.js ? '✅ Available' : '❌ Not available'}`)
  console.log(`  Rust Engine: ${info.rust ? '✅ Available' : '❌ Not available'}`)
}

benchmark().catch(console.error)
