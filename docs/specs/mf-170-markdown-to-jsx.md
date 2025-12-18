# MF-170: Markdown → JSX renderer passthrough

> Draft scaffold – fill in API, edge cases, and test plan.

## Scope
- Preserve raw JSX/MDX nodes through the Markdown pipeline and emit JSX source suitable for downstream bundlers.
- Honor code fence–aware import/export hoisting to keep module headers intact.

## Open Questions
- JSX escaping rules for text nodes (current minimal escape: `& < > { }`).
- How to surface options (e.g., runtime imports, layout wrapping) while keeping a streaming interface.
- Alignment with NAPI/WASM bindings (naming and return shape).

## Next Steps
- Define renderer API surface (options struct, return type).
- Enumerate fixtures for JSX-in-markdown edge cases (props, children, spread, comments, fragments).
- Decide on HTML/JSX interop rules (when to `dangerouslySetInnerHTML` vs. literal text).
