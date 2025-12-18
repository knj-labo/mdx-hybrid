# MF-180: JSX import ordering & collision policy

Status: Draft

## Scope
- Ordering of imports/exports in generated JSX modules (runtime, layout, hoisted user imports/exports).
- Collision/duplication handling strategy (same module imported twice, name conflicts).

## Policy (initial)
- Order target: runtime helpers → layout import → hoisted imports/exports → generated consts (`frontmatter`, `file`, `url`, `_html`, etc.) → exports.
- No de-duplication today; collisions are left to the bundler.

## Open Questions
- Should we de-duplicate identical specifiers or alias conflicting names?
- Where to place user exports relative to generated `frontmatter`/`getHeadings`?
- Should hoisted exports be re-ordered or kept in source order?

## Next Steps
- Decide on dedupe/collision rules; add fixtures for conflicting imports.
- Update codegen once policy is finalized.
