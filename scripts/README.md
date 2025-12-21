# scripts/ usage quick reference

- `check-backlog.mjs` — runs backlog checks (pending items). Used before PRs when backlog changes。
- `compare-astro-harness.mjs` — compare Markflow vs baseline on Astro harness; accepts `--runs` etc.
- `run-astro-harness.mjs` — run the integration harness (`markflow|baseline`) for sanity/perf checks.
- `smoke-napi.mjs` — smoke-test N-API build against a fixture markdown (e.g., `fixtures/core/markdown/hello.md`).

## Usage ledger (mark when last run)

| Script | Purpose | Last run | Notes |
| --- | --- | --- | --- |
| compare-astro-harness.mjs | Astro harness: markflow vs baseline comparison | (pending) | 主要ベンチ。残す前提。 |
| run-astro-harness.mjs | Astro harness runner | (pending) | 主要ベンチ。残す前提。 |
| smoke-napi.mjs | N-API smoke test against fixture | (pending) | 開発時の最小確認に使用。 |

Legend: ★ = 用途不明/重複のため後続ステップで存続判断する候補。
