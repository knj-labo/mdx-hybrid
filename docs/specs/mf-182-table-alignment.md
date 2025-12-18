# MF-182: Table alignment support

Status: Draft

## Scope
- Emission of table alignment in HTML and JSX outputs.
- Tag/attribute choices (`style="text-align:...'"` vs class names) and consistency.

## Policy (current)
- HTML renderer sets `style="text-align:left|right|center"` on `<td>`/`<th>`.
- JSX renderer currently ignores alignment (outputs plain `<td>`/`<th>`).

## Open Questions
- Should JSX renderer mirror the inline style approach or use class names?
- Do we need colgroup support for column-wide alignment?
- How to handle missing/None alignment (default left) in JSX mode?

## Next Steps
- Decide alignment emission for JSX.
- Add fixtures for per-column alignment and mixed header/body cases.
- Update JSX renderer once policy is set.
