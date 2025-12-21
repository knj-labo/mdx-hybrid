# scripts/ usage quick reference

- `check-backlog.mjs` — runs backlog checks (pending items). Used before PRs when backlog changes。
- ~~`compare-astro-harness.mjs`~~ — **removed** (CI/perf harness disabled; run manual benchmarks with your own scripts if needed)。
- ~~`run-astro-harness.mjs`~~ — **removed** (was harness runner).
- `smoke-napi.mjs` — smoke-test N-API build against a fixture markdown (e.g., `fixtures/core/markdown/hello.md`).

## Usage ledger (mark when last run)

| Script | Purpose | Last run | Notes |
| --- | --- | --- | --- |
| compare-astro-harness.mjs | Astro harness: markflow vs baseline comparison | removed | CI/perf harness削除に伴い廃止 |
| run-astro-harness.mjs | Astro harness runner | removed | 上記と同じ理由で廃止 |
| smoke-napi.mjs | N-API smoke test against fixture | (pending) | 開発時の最小確認に使用。 |

Legend: ★ = 用途不明/重複のため後続ステップで存続判断する候補。
