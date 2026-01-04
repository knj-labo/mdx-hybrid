# CI Steps Audit (as of 2025-12-21)

Workflow: `.github/workflows/ci.yml`

| Step | Purpose | Status | Notes |
| --- | --- | --- | --- |
| Checkout code | Fetch repository | keep |  |
| Install pnpm (v9) | Install pnpm CLI | keep (upgrade candidate to v10 after lockfile validation) |  |
| Install Node.js 20 (cache: crates/napi/pnpm-lock.yaml) | Runtime for NAPI build/tests | keep | cache scope limited to NAPI lockfile |
| Install Rust (+fmt, clippy) | Rust toolchain with format/lint | keep |  |
| Cache Rust dependencies | Speed up Rust builds | keep | using swatinem/rust-cache |
| Check formatting | `cargo fmt --all -- --check` | keep |  |
| Run clippy | `cargo clippy --workspace --all-targets -D warnings` | keep |  |
| Run rust tests | `cargo test --workspace` | keep |  |
| Install dependencies (crates/napi) | `pnpm install --frozen-lockfile` | keep | lockfile at crates/napi/pnpm-lock.yaml |
| Build NAPI binding | `pnpm run build:napi` | keep |  |
| Smoke test NAPI binding | `pnpm run smoke:napi -- ../../fixtures/core/markdown/hello.md` | keep | input swapped from removed samples/large.md |
| Install Astro harness deps | fixtures/integration/astro-harness | opt-in job (`astro-harness`) only | harness比較はperfラベル or workflow_dispatchで実行。`--mode=semantic` で簡易構造diff可 |
| Install withastro/docs deps | fixtures/integration/withastro-docs/repo | opt-in job (`withastro-docs-harness`) only | perfラベル or workflow_dispatchで実行。`compare:withastro-docs --mode=semantic` |

Removed (decision #98): `Validate backlog specs` (Backlog.md廃止に伴い削除)

Follow-ups to consider:
- pnpm v10 upgrade once lockfile compatibility is validated.
- Harness比較が恒常運用に必要なら、HTMLセマンティックdiffを compare スクリプトに再導入する（現状は build 時間のみ記録）。
- NAPI build requires prior `pnpm install` in `crates/napi`; missing node_modules leads to `napi: not found`.
