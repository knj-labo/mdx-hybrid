
import { promises as fs } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

async function main() {
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const withastroRoot = resolve(repoRoot, 'fixtures/integration/withastro-docs/repo');

  const dirs_to_delete = [
    resolve(withastroRoot, 'dist-baseline'),
    resolve(withastroRoot, 'dist-markflow'),
    resolve(withastroRoot, '.astro'),
    resolve(withastroRoot, 'node_modules', '.vite'),
    resolve(withastroRoot, 'node_modules', '.astro'),
  ];

  console.log('Deleting the following directories:');
  for (const dir of dirs_to_delete) {
    console.log(` - ${dir}`);
    await fs.rm(dir, { recursive: true, force: true });
  }

  console.log('Cleanup complete.');
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
