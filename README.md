# Markflow

## Harness Commands

### Astro harness (fixtures/integration/astro-harness)
1. Install deps once: `pnpm --dir fixtures/integration/astro-harness install`
2. Markflow build: `node scripts/run-astro-harness.mjs astro markflow`
3. Baseline build: `node scripts/run-astro-harness.mjs astro baseline`

### withastro/docs Starlight harness
See `fixtures/integration/withastro-docs/README.md` for setup notes.

1. Install deps: `pnpm --dir fixtures/integration/withastro-docs/repo install`
2. Semantic diff (HTML): `pnpm compare:withastro-docs -- --mode=semantic`
3. Visual diff (screenshots): `pnpm visual:withastro-docs -- --build`
