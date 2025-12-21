# Fixtures Directory

```
fixtures/
├── core/
│   ├── markdown/        # CommonMark + legacy markdown samples for core + bindings tests
│   └── mdx/             # MDX-specific samples (embedded JSX, expressions, imports)
└── integration/
    └── astro-harness/   # Minimal Astro/Vite harness fed by markflow
```

Use `fixtures/core/*` for unit/regression suites (Rust, AVA, wasm-bindgen). Larger, framework-specific assets should live under `fixtures/integration/` as they are introduced.

## Current inventory (mark usage; ★ = usage pending確認)

| Path | Purpose / Notes | Status |
| --- | --- | --- |
| fixtures/core/markdown/hello.md | Simple markdown smoke fixture | used (core smoke) |
| fixtures/core/markdown/table.md | Table alignment/spacing fixture | used (core tests) |
| fixtures/core/mdx/* | MDX fixtures (embedded JSX, imports, expressions) | used (core/wasm) |
| fixtures/integration/astro-harness/ | Astro harness project | used (scripts/run-astro-harness.mjs) |
| fixtures/integration/withastro-docs/.gitignore | Keeps harness output clean | used |
| fixtures/integration/withastro-docs/harness-summary.json | Baseline metrics for docs harness | used (compare-astro-harness) |
| fixtures/integration/withastro-docs/README.md | Harness usage notes | used |
| fixtures/README.md | This file | n/a |
| ★ (none current) | 未使用候補が見つかればここに追記 | pending |

## Removal/retention flow (when ★ is added)
1) Locate uses via `rg <fixture-name> fixtures` and test/bench scripts; note findings in decision log.
2) If unused and no near-term plan, propose deletion with a PR/decision entry; otherwise tag with next planned use.
3) Delete or keep based on decision log entry; update this table and remove ★ when resolved.
