# withastro/docs Starlight Harness

This fixture mirrors the official `withastro/docs` repository so we can run the Markflow + Starlight end-to-end benchmarks described in [MF-160](../../../docs/specs/mf-160-withastro-docs-harness.md).

## Bootstrap

```bash
# clone/update the upstream repository under fixtures/integration/withastro-docs/repo
./scripts/setup-withastro-docs.sh

# or pin a specific release/commit
WITHASTRO_DOCS_REF=v4.0.0 ./scripts/setup-withastro-docs.sh
```

The script downloads the docs site into `fixtures/integration/withastro-docs/repo`. Pass `--reset` to drop the existing checkout before re-cloning.

Environment variables:
- `WITHASTRO_DOCS_REMOTE` (default: `https://github.com/withastro/docs.git`)
- `WITHASTRO_DOCS_REF` (default: `main`)
- `WITHASTRO_DOCS_DEPTH` (default: `1`, set to an empty string to fetch full history)

Markflow settings:
- `fixtures/integration/withastro-docs/repo/astro.config.ts` enables `markflowPlugin({ starlightComponents: true })`
  so Starlight components can be auto-imported when their tags appear.

## Directory layout
- `repo/` – upstream `withastro/docs` checkout managed by the setup script (ignored in git)
- `harness-summary.json` – legacy benchmark results (compare-astro-harness script has been removed; regenerate with custom tooling if needed)

## Updating upstream
1. Choose a commit/tag in `withastro/docs` (record the SHA in PR description or this README when bumping).
2. Run `WITHASTRO_DOCS_REF=<sha> ./scripts/setup-withastro-docs.sh --reset`.
3. Re-run `pnpm install`, `pnpm dev`, and the harness comparison to verify the new snapshot.

## Visual regression (optional)
Build baseline/markflow outputs and run screenshot diffs:

```bash
pnpm --dir fixtures/integration/withastro-docs/repo install
pnpm visual:withastro-docs -- --build
```

Notes:
- `--build` runs baseline then markflow builds and saves `dist-baseline/` and `dist-markflow/`.
- Output diff PNGs and a summary JSON are written to `fixtures/integration/withastro-docs/visual-diff/`.
- Use `--max 0` to compare all routes (default is a capped subset for speed).

## HTML semantic diff (optional)
Build baseline/markflow outputs and compare normalized HTML:

```bash
pnpm --dir fixtures/integration/withastro-docs/repo install
pnpm compare:withastro-docs -- --mode=semantic
```

Notes:
- Summary is written to `fixtures/integration/withastro-docs/harness-summary.json`.
- Use `--skip-install` if dependencies are already installed.
- The default route list is in `fixtures/integration/withastro-docs/semantic-routes.txt`.
  - Override with `--routes <file>` or compare all HTML with `--all`.
