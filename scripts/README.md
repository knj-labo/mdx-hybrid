# scripts/ usage quick reference

- `compare-astro-harness.mjs` — build baseline/markflow harness and record timings to `fixtures/integration/astro-harness/harness-summary.json`。`--mode=semantic` で簡易HTML構造diff（属性ソート/空白圧縮/コメント除去）を行い、差分があれば非0終了。
- `compare-withastro-docs.mjs` — withastro/docs を baseline/markflow でビルドし、`--mode=semantic` でHTML構造diffを行う（デフォルトは `fixtures/integration/withastro-docs/semantic-routes.txt` の固定ルート）。
- `run-astro-harness.mjs` — run the harness once in `markflow` (default) or `baseline` mode. Accepts `--skip-install` to reuse node_modules。
- `smoke-napi.mjs` — smoke-test N-API build against a fixture markdown (e.g., `fixtures/core/markdown/hello.md`).
- `visual-diff-withastro-docs.mjs` — withastro/docs の baseline/markflow をスクリーンショット比較（Playwright + pixelmatch）。

## Usage ledger (mark when last run)

| Script | Purpose | Last run | Notes |
| --- | --- | --- | --- |
| compare-astro-harness.mjs | Astro harness: build baseline & markflow, record times | (pending) | CI は opt-in（perf ラベル or workflow_dispatch）。CI 実行は `--mode=time` 固定。semantic diff はローカル専用。 |
| compare-withastro-docs.mjs | withastro/docs semantic diff | (new) | `pnpm compare:withastro-docs -- --mode=semantic --all` |
| ast-compare/compare.mjs | AST compare (single file) via unified vs Markflow | (new) | `node scripts/ast-compare/compare.mjs --file fixtures/integration/astro-harness/content/docs/components.mdx` |
| ast-compare/run.mjs | AST compare runner (directory) | (new) | `node scripts/ast-compare/run.mjs --dir fixtures/integration/astro-harness/content/docs` |
| run-astro-harness.mjs | Astro harness runner | (pending) | `mode` 引数: `markflow`/`baseline` |
| smoke-napi.mjs | N-API smoke test against fixture | (pending) | 開発時の最小確認に使用。 |
| visual-diff-withastro-docs.mjs | Visual diff for withastro/docs | (new) | `pnpm visual:withastro-docs -- --build` |

Legend: ★ = 用途不明/重複のため後続ステップで存続判断する候補。
