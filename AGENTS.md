# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates (`core` parser/renderer, `napi` Node bindings, `wasm` bindings).
- `packages/`: JS/TS packages (`astro-loader`, `vite-plugin-markflow`).
- `web/`: Astro site used for local dev and demos.
- `fixtures/`: integration fixtures and harness content.
- `scripts/`: harness runners and smoke tests.
- `docs/` and `benchmarks/`: specs and performance notes.

## Build, Test, and Development Commands
- `pnpm --dir web dev`: run the Astro dev server for the demo site.
- `pnpm --dir web build`: build the Astro site.
- `pnpm --dir packages/astro-loader build`: build the loader package (`dist/`).
- `pnpm --dir packages/astro-loader lint`: type-check the loader package.
- `pnpm --dir crates/napi build`: build N-API binaries via `@napi-rs/cli`.
- `pnpm --dir crates/napi smoke:napi`: quick N-API smoke test against fixtures.
- `pnpm --dir fixtures/integration/astro-harness install`: install Astro harness deps (required once unless using `--skip-install`).
- `node scripts/run-astro-harness.mjs markflow`: run the Astro harness once (use `baseline`; add `--skip-install` to reuse deps).
- `node scripts/compare-astro-harness.mjs --mode=semantic`: baseline vs Markflow build + structure diff (use `--mode=time` for perf-only).
- `node scripts/ast-compare/run.mjs --dir fixtures/integration/astro-harness/content/docs`: compare AST output for a directory.
- `cargo build`: build the Rust workspace crates.

## Coding Style & Naming Conventions
- JS/TS: 2-space indentation, single quotes, and semicolons (match existing code). Keep file names lowercase with hyphens when applicable.
- Rust: follow `rustfmt` defaults; prefer `snake_case` for functions/modules and `CamelCase` for types. Run `cargo clippy` when touching Rust logic.
- Keep public APIs documented and favor small, focused modules.

## Testing Guidelines
- Rust: `cargo test` (workspace) or `cargo test -p markflow-core` for core-only changes.
- Snapshots: `insta` is used in `crates/core/tests`, with snapshots under `crates/core/tests/snapshots`.
- N-API JS tests: AVA in `crates/napi/tests/*.test.js` via `pnpm --dir crates/napi test`.
- WASM tests use `wasm-bindgen-test`; run only if you change `crates/wasm`.

## Commit & Pull Request Guidelines
- Commit messages follow a conventional pattern such as `feat:`, `fix:`, `chore:`, or scoped `docs(scope):`, with short, present-tense summaries. Avoid `wip` commits in PRs.
- No formal PR template found; include a clear summary, test notes, linked issues (if any), and screenshots for `web/` changes. Mention harness/benchmark results if performance is impacted.

## Agent Notes
- This repo uses pnpm workspaces; prefer `pnpm --dir <workspace>` to target the right package.
- Harness scripts in `scripts/` are the source of truth for Astro baseline vs Markflow comparisons.
