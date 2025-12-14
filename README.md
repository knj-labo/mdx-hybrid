# Markflow

## Harness Commands

### Astro harness (fixtures/integration/astro-harness)
1. Install deps once: `pnpm --dir fixtures/integration/astro-harness install`
2. Markflow build: `node scripts/run-astro-harness.mjs markflow`
3. Baseline build: `node scripts/run-astro-harness.mjs baseline`

### withastro/docs Starlight harness
1. Fetch upstream repo: `./scripts/setup-withastro-docs.sh --reset`
2. Install deps: `pnpm --dir fixtures/integration/withastro-docs/repo install`
3. Markflow build: `cd fixtures/integration/withastro-docs/repo && pnpm astro build`
4. Baseline build: `cd fixtures/integration/withastro-docs/repo && MARKFLOW_HARNESS_BASELINE=1 pnpm astro build`
