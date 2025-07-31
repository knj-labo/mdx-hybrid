import { writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))

function generateLargeMDX() {
  const sections = []

  // Header
  sections.push('# Large MDX File - Performance Test')
  sections.push('')
  sections.push(
    'This is a large MDX file with approximately 1000 lines of content for performance testing.'
  )
  sections.push('')

  // Generate 50 sections
  for (let i = 1; i <= 50; i++) {
    sections.push(`## Section ${i}`)
    sections.push('')
    sections.push(
      `This is section ${i} of the large MDX file. It contains various markdown elements to test the performance of the MDX compiler.`
    )
    sections.push('')

    // Add a paragraph
    sections.push(
      'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.'
    )
    sections.push('')

    // Add a code block
    sections.push('```javascript')
    sections.push(`function example${i}() {`)
    sections.push(`  const value = ${i * 10}`)
    sections.push(`  console.log('Processing section ${i}')`)
    sections.push('  return value * 2')
    sections.push('}')
    sections.push('```')
    sections.push('')

    // Add a list
    sections.push(`### Features of Section ${i}`)
    sections.push('')
    sections.push(`- Feature ${i}.1: Description of the first feature`)
    sections.push(`- Feature ${i}.2: Description of the second feature`)
    sections.push(`- Feature ${i}.3: Description of the third feature`)
    sections.push('')

    // Add a JSX component
    sections.push(`<CustomComponent id="${i}" title="Section ${i} Component">`)
    sections.push(`  This is a custom component in section ${i}.`)
    sections.push('</CustomComponent>')
    sections.push('')

    // Add a table every 10 sections
    if (i % 10 === 0) {
      sections.push('| Column 1 | Column 2 | Column 3 |')
      sections.push('|----------|----------|----------|')
      for (let j = 1; j <= 5; j++) {
        sections.push(`| Row ${j}-1 | Row ${j}-2 | Row ${j}-3 |`)
      }
      sections.push('')
    }
  }

  return sections.join('\n')
}

// Generate large.mdx
const largeMDX = generateLargeMDX()
const outputPath = join(__dirname, 'fixtures', 'large.mdx')
writeFileSync(outputPath, largeMDX)

console.log(`Generated large.mdx with ${largeMDX.split('\n').length} lines`)
console.log(`File size: ${(Buffer.byteLength(largeMDX) / 1024).toFixed(2)} KB`)
