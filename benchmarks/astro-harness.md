# Astro Harness Benchmarks

Command: `node scripts/compare-astro-harness.mjs --runs=2 --summary harness-summary.json`
Date: 2025-12-07

Dataset: 400 synthetic heavy docs (each ≈240 repeated sections with tables, code blocks, and directives) plus real sample pages.

| Mode     | Avg (ms) | Min (ms) | Max (ms) |
| -------- | -------- | -------- | -------- |
| Markflow | 17890.14 | 17698.82 | 18081.46 |
| Baseline | 35383.74 | 35142.39 | 35625.08 |

Speedup (baseline / markflow): 1.98x

`harness-summary.json` captures the raw data for automation.
