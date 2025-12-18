# MF-176: Image handling policy (lazy-load, attrs)

Status: Draft

## Scope
- Lazy-loading behavior (`loading="lazy"` default) across HTML/JSX renderers.
- Preservation/normalization of `alt`, `title`, and `src` attributes.
- Interaction with hoisted imports/JSX passthrough (e.g., images inside JSX nodes are untouched).

## Policy (current)
- HTML renderer enforces `loading="lazy"` when missing.
- No URL rewriting or validation performed.
- `alt` text gathered from Markdown image text; `title` preserved when provided.
- JSX renderer currently bypasses image rendering; policy needs to decide whether to emit `<img>` or leave to caller.

## Open Questions
- Should we always emit `loading="lazy"` in JSX output, or leave images untouched?
- Do we normalize relative vs absolute URLs?
- Should we reject data URLs or other schemes?
- How to handle width/height extraction (if at all)?

## Next Steps
- Decide JSX image emission strategy (emit vs skip) and align with HTML path.
- Add fixtures for: missing alt, missing title, existing loading, JSX-wrapped images.
- Update renderer implementations once rules are agreed.
