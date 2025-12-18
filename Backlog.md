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
| MF-171 | Draft | JSX renderer API & escaping rules | docs/specs/mf-171-jsx-renderer-api.md | Kenji | TBD | Flesh out API surface and escaping matrix |
| MF-172 | Draft | JSX layout/wrapper integration | docs/specs/mf-172-jsx-layout-integration.md | Kenji | TBD | Define layout wrapping, props, and slots |
| MF-173 | Draft | Astro plugin integration & harness rollout | docs/specs/mf-173-astro-plugin-harness.md | Kenji | TBD | Plan plugin wiring and harness rollout steps |
| MF-174 | Draft | Hoist edge-case test matrix | docs/specs/mf-174-hoist-tests.md | Kenji | TBD | List fixtures for import/export edge cases |
| MF-175 | Draft | JSX attribute escaping & sanitization | docs/specs/mf-175-jsx-attr-escaping.md | Kenji | TBD | Draft escaping matrix and sanitizer rules |
| MF-176 | Draft | Image handling policy (lazy-load, attrs) | docs/specs/mf-176-image-policy.md | Kenji | TBD | Draft image attr/lazy rules |
| MF-177 | Draft | Math rendering compatibility (inline/display) | docs/specs/mf-177-math-compat.md | Kenji | TBD | Define tags/classes/escape rules |
| MF-178 | Draft | Frontmatter extraction & type mapping | docs/specs/mf-178-frontmatter-types.md | Kenji | TBD | Define YAML→JSON mapping and error policy |
| MF-179 | Draft | Diagnostics & logging policy | docs/specs/mf-179-diagnostics.md | Kenji | TBD | Define tracing/log levels and harness metrics |
| MF-180 | Draft | JSX import ordering & collision policy | docs/specs/mf-180-jsx-import-order.md | Kenji | TBD | Define runtime/layout/hoisted ordering |
| MF-181 | Draft | Raw HTML & dangerouslySetInnerHTML policy | docs/specs/mf-181-raw-html-policy.md | Kenji | TBD | Define allow/deny rules for raw HTML in JSX |
| MF-182 | Draft | Table alignment support | docs/specs/mf-182-table-alignment.md | Kenji | TBD | Define alignment output for HTML/JSX |
| MF-183 | Draft | CLI/tooling entrypoint spec | docs/specs/mf-183-cli-tooling.md | Kenji | TBD | Outline CLI commands and options |
| MF-184 | Draft | Error recovery strategy | docs/specs/mf-184-error-recovery.md | Kenji | TBD | Define continue/abort rules per phase |

## 運用メモ
- `Status` は `Backlog -> Ready -> In Progress -> Review -> Done` の順で遷移させ、Codex の作業終了時に `Review` へ切り替えます。
- `Spec` カラムには常にリポジトリ内のドキュメントパスを記入し、外部 URL にはしません。
- `Owner`/`Due` を使って責任者とターゲット日を明示し、遅延が発生した場合は履歴を残すため Backlog 上で日付を更新します。
- Codex にタスクを渡す際は、該当行をコピーしてプロンプトに貼り付けるとコンテキスト共有が容易です。
- 提出前に `node scripts/check-backlog.mjs` を実行し、Spec パスが存在することと見出しが ID を含むことを検証します（CI でも `.github/workflows/ci.yml` の `Validate backlog specs` ステップが同チェックを行います）。
