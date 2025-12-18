# MF-177: Math rendering compatibility (inline/display)

Status: Draft

## Scope
- Inline vs display math rendering in HTML and JSX outputs.
- Tag/class conventions (`<span class="math-inline">`, `<div class="math-display">`).
- Escaping and passthrough rules for math content.

## Policy (current)
- Inline math emitted as `<span class="math-inline">...</span>`.
- Display math emitted as `<div class="math-display">...</div>`.
- Math content is not escaped; assumed to be handled by downstream KaTeX/MathJax.

## Open Questions
- Should we wrap math in `{` `}` for JSX to avoid HTML escaping side effects?
- Do we need configurable class names to align with KaTeX/MathJax themes?
- Should `$...$`/`$$...$$` be preserved verbatim in JSX mode for later processing instead of HTML tags?

## Next Steps
- Decide final tag/class mapping for JSX vs HTML paths.
- Add fixtures covering inline/display math, nested with text, and mixed with HTML/JSX nodes.
- Update renderer to align with chosen mapping (if changes needed).
