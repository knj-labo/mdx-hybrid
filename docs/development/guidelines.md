# Repository Guidelines

_This document expands on the quickstart in [AGENTS.md](../../AGENTS.md). Update both whenever contributor guidance changes._

## Project Structure & Workspace
Markflow is a Cargo workspace with `crates/core` (Rust parser + streaming rewriter), `crates/napi` (Node bindings), and `crates/wasm` (browser bindings). Tests live beside their crates (`crates/core/src/**/*.rs`, `crates/napi/tests/*.test.js`). Shared resources sit in `fixtures/` (Markdown samples), `benchmarks/` (perf harnesses), `samples/` (integration demos), `scripts/` (smoke tools), and `docs/` (architecture notes). Treat `ROADMAP.md` as the canonical roadmap when planning work.

## Build, Test, and Development Commands
Run `cargo fmt --all && cargo clippy --workspace --all-targets` before committing to keep the Rust code formatted and lint-clean. Use `cargo test --workspace` for Rust suites (including the inline tests inside `crates/core/src/lib.rs`). For the Node bindings, execute `pnpm install --filter markflow` inside `crates/napi`, followed by `pnpm run build` to compile the N-API binary, `pnpm test` for AVA suites, and `node ../../scripts/smoke-napi.mjs fixtures/core/markdown/hello.md` for a fast end-to-end check. Add `cargo bench -p markflow-core` only when you need perf evidence for a PR.

## Parser & Glue Expectations
The core parser should continue to expose pull-based markdown events that stream directly into the PipeAdapter and `lol_html` rewriter—avoid materializing ASTs. Keep public APIs aligned (`parse`, `parseWithOptions`, `parseWithStats`) so docs and bindings stay synchronized. When touching adapter or rewriter modules, document any changes in streaming behavior (buffer sizes, lazy attributes, math passthrough) so WASM and N-API crates can mirror them.

## Coding Style & Naming
The workspace targets Rust 2024 and the default `rustfmt` profile; prefer module-focused files (`streaming_rewriter.rs`, `markdown_adapter.rs`) and snake_case identifiers. Keep `#![deny(missing_docs)]` at the crate root and describe how each struct participates in the streaming pipeline. JavaScript/TypeScript inside `crates/napi` is pure ESM: camelCase for functions, PascalCase for exported classes, kebab-case file names.

## Testing & Benchmarks
Use table-driven tests in Rust modules with fixtures pulled from `fixtures/core/markdown`. Mirror the same scenarios in AVA tests under `crates/napi/tests`, keeping filenames `<feature>.test.js`. Before opening a PR, run `cargo test --workspace`, `pnpm test`, and the smoke script with at least one real `.md` file. When a change claims performance wins, capture benchmark output in `benchmarks/results.md` or attach the CLI log to the PR.

For Phase 2 integration work, keep the Astro harness (`fixtures/integration/astro-harness`) in sync: `pnpm install` inside the harness once, then `node scripts/run-astro-harness.mjs markflow` / `baseline` to ensure the build stays green. When you need numbers, run `node scripts/compare-astro-harness.mjs --runs=5` to capture average build times for Markflow vs. the baseline compiler.

## Commit & PR Workflow
Follow lightweight Conventional Commits (e.g., `feat: apply format`, `fix: clippy regressions`). PR descriptions must list affected crates, manual verification (commands executed or fixtures used), and any ROADMAP.md items closed. Include screenshots only when rendered HTML changes; otherwise paste benchmark or smoke-test logs. Never commit generated artifacts from bindings or secrets—use `.env.local` and keep tokens out of the repo.
