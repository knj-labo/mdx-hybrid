# MF-181: Raw HTML & dangerouslySetInnerHTML policy

Status: Draft

## Scope
- Handling of raw HTML chunks in Markdown→JSX output.
- Whether/when to allow `dangerouslySetInnerHTML` vs literal strings.
- Interaction with hoist/JSX passthrough and sanitization policies.

## Policy (initial)
- Current behavior: `Html`/`InlineHtml` and `Jsx*` nodes are emitted verbatim; no `dangerouslySetInnerHTML` is used.
- No sanitization performed on raw HTML.

## Open Questions
- Should raw HTML be blocked or sanitized in JSX mode? If blocked, what is the fallback (escape vs drop)?
- If `dangerouslySetInnerHTML` is allowed, under what option flag and scope (block-only? inline?)?
- How does this interact with attribute sanitization (MF-175) and diagnostics (MF-179)?

## Next Steps
- Decide default allow/deny and optional flags.
- Add fixtures for inline/block HTML, script/style tags, event handlers.
- Update renderer behavior according to the decision.
