# 2025-12-25: JSX output in NAPI codegen

## Decision
- The Rust compiler returns JSX source (string) instead of HTML for Astro integration.
- `generate_module_code_from_ir` emits JSX function components and embeds JSX body as code, not as a string literal.
- The generated module includes `import { Fragment as _Fragment, jsx as _jsx } from 'astro/jsx-runtime';`.
- `markHTMLString` / `createComponent` are not used in JSX mode.
- JSON AST-based performance optimization is deferred to a future improvement task.

## Rationale
- JSX keeps Astro components evaluatable at runtime and avoids tight coupling to Astro internal HTML APIs.

## Status
- Accepted.

## Follow-up
- Vite virtual modules should use a `.markflow.jsx` suffix so the JSX output is parsed as JSX.
- Vite virtual module IDs use `.markflow.jsx`, and `load` returns `meta: { vite: { jsx: true } }` to force JSX parsing.
- Always emit `import { Fragment as _Fragment, jsx as _jsx } from 'astro/jsx-runtime';` before hoisted imports; do not include `jsxs` / `jsxDEV`.
- Vite `configResolved` fills `esbuild.jsx="automatic"` and `esbuild.jsxImportSource="astro"` when unset (unless `esbuild` is `false`).
- Vite `load` bridges JSX with `transformWithEsbuild` (no new deps) using `loader: "jsx"` and `jsxImportSource: "astro"` to satisfy import analysis.
- Vite `load` uses classic JSX transform (`jsx: "transform"`) with `jsxFactory: "_jsx"` and `jsxFragment: "_Fragment"` to avoid `Fragment` reference errors.
- `Content` and default exports are wrapped with `createComponent((result, props) => renderJSX(result, _jsx(Component, { ...props })), file)` to avoid NoMatchingRenderer.
- `createComponent`/`renderJSX` are imported from `astro/runtime/server/index.js` to avoid subpath resolution failures.
- `MarkflowContent` itself is an Astro component factory (not a plain function) so JSX rendering never routes through a framework renderer.
- `_jsx` is wrapped to coerce `props ?? {}` because Astro `createVNode` throws on `props = null`.
- `Fragment` is imported directly, and `_Fragment` is derived from it so both `<Fragment>` and fragments `<>` resolve.
- JSX 出力の初期確認は `cargo test -p markflow-napi` の既存テスト結果に依存し、追加テストは行わない（2025-12-25）。
- JSON AST 化によるパフォーマンス最適化は将来タスクとして記録し、現段階では実装しない。

## Implemented in
- crates/napi/src/lib.rs
- crates/napi/src/compiler.rs
- crates/core/src/renderer/jsx_renderer.rs
- packages/vite-plugin-markflow/src/index.js
- docs/specs/mf-172-jsx-layout-integration.md
- docs/specs/mf-173-astro-plugin-harness.md
- docs/specs/mf-170-markdown-to-jsx.md
- Starlight-specific heading wrappers/anchors should be separated from Astro-generic output in a follow-up task.
