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
| Install Astro harness deps | `pnpm install --frozen-lockfile` in fixtures/integration/astro-harness | keep |  |
| Compare Astro harness | `node scripts/compare-astro-harness.mjs --runs=2 --summary harness-summary.json` | keep (perf) | could be gated if CI time becomes an issue |
| Publish Astro harness summary | Append metrics to GH Step Summary | keep | step already skips when summary file missing |

Removed (decision #98): `Validate backlog specs` (Backlog.md廃止に伴い削除)

Follow-ups to consider:
- pnpm v10 upgrade once lockfile compatibility is validated.
- Optional gating/parallelization for harness comparison if CI time needs reduction.
