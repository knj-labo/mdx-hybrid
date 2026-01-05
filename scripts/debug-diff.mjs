
import { promises as fs } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

async function main() {
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const withastroRoot = resolve(repoRoot, 'fixtures/integration/withastro-docs/repo');

  const baselinePath = resolve(withastroRoot, 'dist-baseline/en/getting-started/index.html');
  const markflowPath = resolve(withastroRoot, 'dist-markflow/en/getting-started/index.html');

  console.log('Reading baseline:', baselinePath);
  const baselineHtml = await fs.readFile(baselinePath, 'utf8');

  console.log('Reading markflow:', markflowPath);
  const markflowHtml = await fs.readFile(markflowPath, 'utf8');

  await fs.writeFile('raw_baseline.html', baselineHtml);
  await fs.writeFile('raw_markflow.html', markflowHtml);
  console.log('Wrote raw HTML outputs to raw_baseline.html and raw_markflow.html');
  console.error('Run `diff -u raw_baseline.html raw_markflow.html` to see differences.');
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
