# MF-174: Hoist edge-case test matrix

Status: Draft

## Goal
Define fixture coverage for code-fence-aware import/export hoisting, capturing real-world edge cases.

## Fixture Matrix (initial)
- Single-line import/export with/without semicolon.
- Multiline imports (braces), multiline exports (`export { foo, bar }`), export default function (multiline), export default async arrow.
- export * from './mod'.
- Inline comments on import/export lines.
- Fenced code blocks containing import/export (should NOT hoist).
- Indented fences and mismatched markers.

## Open Questions
- Should we include shebang / top-of-file comments interactions?
- Windows CRLF vs LF fixture duplication needed?
- Do we assert stable ordering when multiple hoisted statements appear?

## Next Steps
- Write fixtures under `fixtures/core/hoist/` mirroring the above cases.
- Add Rust and NAPI tests that consume the fixtures.
- Document expected hoisted list vs body output for each case.
