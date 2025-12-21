# MF-173: Astro plugin integration & harness rollout

> Note: Harness scripts (`run-astro-harness.mjs`, `compare-astro-harness.mjs`) are reintroduced with build-only checks (no HTML diff). CI runs them only when opted-in (perf label or workflow_dispatch).

Status: Draft

## Goal
Plan how to wire the updated NAPI/WASM JSX renderer into the Astro plugin, and validate via the harness / withastro/docs.

## Scope
- Plugin options surface (`renderToJsx` toggle, runtime imports, layout handling).
- Harness execution plan and success criteria (build/dev, directive parity, perf smoke).
- Dependency pinning and fallback strategy (monorepo path vs published package).

## Open Questions
- Should the plugin auto-detect MDX vs MD and select JSX vs HTML output?
- How to stage rollout in withastro/docs (subset of routes vs full swap)?
- Versioning/feature-flagging strategy to avoid breaking existing users.

## Plugin Options (proposal)
- `renderToJsx` (boolean, default: false) — route Markdown through `renderToJsx` instead of HTML render.
- `runtimeImport` (string, optional) — forwarded to `render_to_jsx` options.
- `wrapLayout` (boolean, default: true) — whether to wrap with layout when frontmatter has `layout`.
- `escapeHtml` (boolean, default: true) — text escaping flag passed through.
- `useWasm` (boolean, default: false) — switch between NAPI and WASM backend.
- `trace` (boolean, default: false) — emit timing/logs for harness comparisons.

## Harness Validation Plan
- `scripts/run-astro-harness.mjs` で baseline / markflow をビルド。`compare-astro-harness.mjs` はビルド時間を記録するのみ（HTML diff なし）。
- 必要に応じて dev モードの手動比較やセマンティック diff を追加実装する。
- 回帰ポイント: directive hydration、frontmatter layout、auto-imported components、コードフェンス内 import。

## withastro/docs Rollout Plan (staged)
1) **Preview subset**: apply plugin to a small folder (e.g., `/en/getting-started`), feature-flag via `renderToJsx` option.
2) **Partial rollout**: expand to one locale or section; keep baseline fallback path.
3) **Full rollout**: enable across docs; remove fallback once CI + manual checks are clean.
Fallback: flip `renderToJsx=false` and/or `useWasm=false` to revert to existing pipeline without code churn.

## Next Steps
- Implement plugin options and thread them into NAPI/WASM calls.
- Prepare fixture pages listと期待出力（HTML/JSX）をまとめたチェックリストを用意する。
- Rollout milestones: internal preview → partial docs → full docs。必要なら ad-hoc harness を一時的に用意。
