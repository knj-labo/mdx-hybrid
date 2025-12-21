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
| Install Astro harness deps | `pnpm install --frozen-lockfile` in fixtures/integration/astro-harness | keep (for local perf runs) | harness comparison is disabled in CI |

Removed (decision #98): `Validate backlog specs` (Backlog.md廃止に伴い削除)

Follow-ups to consider:
- pnpm v10 upgrade once lockfile compatibility is validated.
- Harness comparison scripts removed from CI; run locally only when native NAPI binding is available.
- NAPI build requires prior `pnpm install` in `crates/napi`; missing node_modules leads to `napi: not found`.
