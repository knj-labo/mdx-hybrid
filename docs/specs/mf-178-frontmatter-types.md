# MF-178: Frontmatter extraction & type mapping

Status: Draft

## Scope
- YAML frontmatter extraction rules (delimiters, BOM/blank-line handling).
- YAML→JSON type mapping for NAPI/WASM return values.
- Error handling policy (hard fail vs fallback) and propagation to callers.

## Policy (current)
- Only YAML supported; TOML/JSON not yet parsed (future work).
- Errors are returned (compile abort); no silent `{}` fallback.
- Extraction skips BOM and leading blank lines; stops at closing `---`.
- NAPI returns `frontmatter_json` (stringified JSON) plus error list; WASM returns value+errors.

## Open Questions
- Should we coerce dates/numbers to strings or keep native JSON types?
- Allow empty frontmatter (`---\n---`) as `{}`? (currently yes)
- Do we support per-collection validation hooks later?

## Next Steps
- Document exact YAML→JSON mapping table (scalar/seq/map/null/aliases).
- Add fixtures for tricky values (dates, numbers with underscores, booleans, nested objects).
- Decide on future TOML/JSON support and backward compatibility flags.
