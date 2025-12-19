# Astro Harness

Minimal Astro project wired to the `markflow` Vite plugin (`plugins/markflow-content-plugin.mjs`). The harness now mirrors multiple docs (architecture, components, directives, performance, migration, and localized pages under `content/docs/i18n/`).

## Usage

```bash
cd fixtures/integration/astro-harness
pnpm install            # first time only
pnpm run build          # Markflow-enabled build
pnpm run build:baseline # toggles MARKFLOW_HARNESS_BASELINE=1
# run from repo root when comparing
node scripts/compare-astro-harness.mjs --runs=5

# optional E2E smoke (directive → Aside + auto-import)
MARKFLOW_HARNESS_E2E=1 node tests/directives.aside.test.mjs
```

The harness exposes docs through the virtual module `virtual:markflow-docs`. When the
baseline env var is set, the plugin falls back to a remark-based compiler to simulate
Astro's legacy pipeline, enabling quick performance comparisons.
