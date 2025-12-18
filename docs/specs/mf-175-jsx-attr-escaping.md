# MF-175: JSX attribute escaping & sanitization

Status: Draft

## Scope
- Escaping rules for attribute values in JSX output (quotes, braces, entities).
- Sanitization policy: which attributes are allowed/blocked, and whether to normalize URLs or strip dangerous ones.
- Interaction with hoisted imports/exports and raw JSX nodes (i.e., do not double-escape JSX-provided attrs).

## Policy (initial)
- Text content escaping covers `& < > { }` (existing behavior); attribute-specific escaping TBD.
- Prefer literal-attribute emission only for trusted values; consider brace-wrapping (`attr={value}`) for computed/escaped paths.
- No `dangerouslySetInnerHTML` usage in JSX renderer.
- Image/link URLs: no rewriting today; sanitization rules to be defined (possible allowlist for `http/https/mailto`).

## Open Questions
- Do we escape quotes (`"`, `'`) inside attributes or require brace syntax?
- Should we strip `on*` event attributes emitted from Markdown/HTML?
- How to handle `class` vs `className`, `style` attributes in JSX mode?
- Interaction with math markup and inline HTML nodes.

## Next Steps
- Draft escaping matrix (text/attr/JSX nodes) and allowed attribute list.
- Decide sanitizer behavior (strip/escape) and add regression fixtures.
- Update renderer implementation plan accordingly.
