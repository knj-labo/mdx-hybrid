# Repository Guidelines

## Project Structure & Module Organization
- `crates/core` hosts the Rust parser, streaming rewriter, and shared error types; keep new pipeline primitives near the modules they extend.
- `crates/napi` exposes the Node bindings plus AVA tests under `tests/`; it ships via `index.js` and generated `markflow-napi` artifacts.
- `crates/wasm` mirrors the core surface for browser runtimes; keep its features in lockstep with `core`.
- Supporting assets live in `fixtures/` (see `fixtures/README.md` for layout), `benchmarks/` (performance harnesses), `samples/` (integration demos), and `docs/` (architecture + development notes).

## Build, Test & Development Commands
- `cargo fmt --all && cargo clippy --workspace --all-targets` keeps the Rust workspace formatted and lint-clean.
- `cargo test --workspace` executes the Rust unit tests, including the inline suites in `crates/core/src/lib.rs`.
- `pnpm install --filter markflow` (run inside `crates/napi`) bootstraps the Node toolchain with the repo `.npmrc` settings.
- `pnpm run build` compiles the N-API binary for supported targets; follow with `node ../../scripts/smoke-napi.mjs fixtures/core/markdown/hello.md` to sanity-check outputs.
- `pnpm run build --filter @markflow/astro-loader` uses `tsdown@0.15.0` to bundle the Astro loader package and emit ESM + `.d.ts` artifacts under `dist/`.

## Coding Style & Naming Conventions
- Rust code depends on the default `rustfmt` profile (edition 2024); prefer module-oriented files (`streaming_rewriter.rs` vs. monoliths) and snake_case identifiers.
- JavaScript/TypeScript inside `crates/napi` is ESM-only; use camelCase for functions, PascalCase for exported classes, and keep file names kebab-case.
- Public APIs should mirror the `parse`, `parseWithOptions`, `parseWithStats` naming already exposed so docs and bindings stay synchronized.

## Testing Guidelines
- Favor table-driven tests in Rust modules and keep assertions focused on HTML fragments so rewrites stay resilient to structural changes.
- AVA tests live under `crates/napi/tests/*.test.js`; mirror Rust scenarios and use fixtures from `fixtures/core/markdown` for parity.
- Before opening a PR, run `cargo test --workspace`, `pnpm test` (inside `crates/napi`), and the `scripts/smoke-napi.mjs` script against at least one real `.md` file.
- When touching the WASM bindings, run `wasm-pack test --node crates/wasm` to keep browser/edge parity.
- Use the Astro harness under `fixtures/integration/astro-harness/` (`node scripts/run-astro-harness.mjs markflow|baseline`) when making changes that affect integration/perf behavior, and `node scripts/compare-astro-harness.mjs --runs=3` to record Markflow vs. baseline timing deltas.

## Commit & Pull Request Guidelines
- Follow lightweight Conventional Commits (`feat:`, `fix:`, `chore:`, `wip:`) as seen in recent history (`feat: apply format`, `fix clippy`).
- Each PR should describe the affected crate(s), list manual verification steps, link to any GitHub issue or ROADMAP slice it resolves, and note the result of `node scripts/check-backlog.mjs` when Backlog entries change.
- Include screenshots only when behavior changes affect rendered Markdown; otherwise attach benchmark diffs or CLI output.

## Security & Configuration Tips
- The repo trusts local `.npmrc` defaults; avoid committing tokens and prefer environment variables for any service keys referenced by benchmarks.
- When working on bindings, keep generated artifacts out of version control and confirm license compatibility before publishing.
