# Markflow

## Harness Commands

### Astro harness (fixtures/integration/astro-harness)
1. Install deps once: `pnpm --dir fixtures/integration/astro-harness install`
2. Markflow build: `node scripts/run-astro-harness.mjs astro markflow`
3. Baseline build: `node scripts/run-astro-harness.mjs astro baseline`

### withastro/docs Starlight harness
1. Fetch upstream repo: `./scripts/setup-withastro-docs.sh --reset`
2. Install deps: `pnpm --dir fixtures/integration/withastro-docs/repo install`
3. Markflow build: `node scripts/run-astro-harness.mjs withastro-docs markflow`
4. Baseline build: `node scripts/run-astro-harness.mjs withastro-docs baseline`
5. (Optional, local only) Build NAPI and run harness manually if you need perf numbers; CI no longer runs `compare-astro-harness`.
