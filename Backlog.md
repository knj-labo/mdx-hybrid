# Backlog

Codex と一緒に仕様駆動開発を進めるためのタスク一覧。各行は 1 つの仕様/PR に対応し、`docs/development/spec-driven-codex.md` に記載のフローで運用します。

| ID | Status | Title | Spec | Owner | Due | Next |
| --- | --- | --- | --- | --- | --- | --- |
| MF-101 | Ready | Streaming Rewriter Diagnostics | docs/specs/mf-101-streaming.md | Kenji | 2025-12-20 | Have Codex read the spec and propose an instrumentation plan |
| MF-118 | In Progress | N-API smoke coverage parity | docs/specs/mf-118-napi-smoke.md | Aya | 2026-01-10 | Finish tests + update AGENTS.md checklist |
| MF-130 | Blocked | WASM streaming adapter parity | docs/specs/mf-130-wasm.md | Miki | TBD (post API decision) | Awaiting API decision from core team |
| MF-140 | Review | Core Engine Streaming Adapter | docs/specs/mf-140-core-engine.md | Kenji | 2026-01-15 | Collect perf baselines + merge PR |
| MF-150 | Draft | Astro/MDX NAPI Integration | docs/specs/mf-150-astro-mdx-napi.md | Kenji | TBD | Socialize spec + wire roadmap tasks |
| MF-160 | Draft | withastro/docs Starlight harness | docs/specs/mf-160-withastro-docs-harness.md | Kenji | TBD | Choose import strategy (subtree vs script) and bootstrap harness |
| MF-170 | Draft | Markdown→JSX renderer passthrough | docs/specs/mf-170-markdown-to-jsx.md | Kenji | TBD | Outline renderer API and test matrix |

## 運用メモ
- `Status` は `Backlog -> Ready -> In Progress -> Review -> Done` の順で遷移させ、Codex の作業終了時に `Review` へ切り替えます。
- `Spec` カラムには常にリポジトリ内のドキュメントパスを記入し、外部 URL にはしません。
- `Owner`/`Due` を使って責任者とターゲット日を明示し、遅延が発生した場合は履歴を残すため Backlog 上で日付を更新します。
- Codex にタスクを渡す際は、該当行をコピーしてプロンプトに貼り付けるとコンテキスト共有が容易です。
- 提出前に `node scripts/check-backlog.mjs` を実行し、Spec パスが存在することと見出しが ID を含むことを検証します（CI でも `.github/workflows/ci.yml` の `Validate backlog specs` ステップが同チェックを行います）。
