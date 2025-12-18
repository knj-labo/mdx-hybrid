# MF-179: Diagnostics & logging policy

Status: Draft

## Scope
- Logging levels and categories for core/NAPI/WASM.
- Harness/tracing outputs (timings, counters) for benchmarking and regression checks.
- Error message format guidelines for user-facing and developer-facing contexts.

## Policy (initial)
- Keep core silent by default; enable tracing via explicit flags (e.g., `trace=true` plugin option).
- Prefer structured logs (JSON) in CI/harness, human-readable in local dev.
- Errors should include category (`frontmatter`, `hoist`, `jsx-render`, `napi`) and short message.

## Open Questions
- What minimal metrics should harness emit? (build time, render time, hoist counts?)
- Should NAPI expose a `diagnostics` array alongside results?
- How to gate expensive tracing in WASM (feature flag vs runtime option)?

## Next Steps
- Define flag names and propagation (CLI/env/plugin options).
- Decide metric set and format for harness scripts.
- Add fixtures/tests to assert key error messages and log categories.
