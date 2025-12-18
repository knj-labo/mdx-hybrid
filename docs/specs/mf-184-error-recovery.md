# MF-184: Error recovery strategy

Status: Draft

## Scope
- Define continue/abort rules per pipeline phase (frontmatter parse, hoist, Markdown parse, JSX render, codegen).
- User-facing vs developer-facing error surfaces (NAPI/WASM/core).

## Policy (initial thoughts)
- Frontmatter syntax errors: abort compile; return errors to caller.
- Hoist parsing errors: prefer abort with clear message; no silent drop.
- Markdown parse errors: abort; collect diagnostics if available.
- JSX render/codegen: abort on structural mismatches; avoid partial output.

## Open Questions
- Should there be a "best-effort" mode for docs preview (log and continue)?
- How to flag recoverable vs fatal errors in NAPI responses?
- Interaction with Diagnostics spec (log categories, trace flags).

## Next Steps
- Define an error categorization and severity levels.
- Specify return shapes for NAPI/WASM when partial success is allowed.
- Add fixtures to exercise recover/abort branches.
