# MF-171: JSX renderer API & escaping rules

Status: Draft (reinforced with current implementation)

## Goal
Define the Rust/NAPI/WASM API surface and the escaping/serialization rules for the Markdown→JSX renderer so downstream bundlers receive predictable JSX with raw JSX passthrough.

## API Surface (proposed)
- Rust: `render_to_jsx(input: &str, options: JsxOptions) -> Result<String, MarkflowError>`
  - `JsxOptions { runtime_import: Option<String>, wrap_layout: bool, escape_html: bool }`
  - Current implementation defaults: `runtime_import = None`, `wrap_layout = false`, `escape_html = true (text only)`.
- NAPI: `renderToJsx(input: string, options?: { runtimeImport?: string; wrapLayout?: boolean; escapeHtml?: boolean; }) => string`
- WASM: `render_jsx(input: string, options?: { runtimeImport?: string; wrapLayout?: boolean; escapeHtml?: boolean; }) => string`
- For Phase 2 the options are optional; missing = same defaults as above.

## Event → JSX Mapping (current behavior)
| Event | Output | Escaping |
| --- | --- | --- |
| `Text` | text content | escape `& < > { }` |
| `Code` | `<code>{escaped}</code>` | escape text |
| `Html` / `InlineHtml` | literal string | no escape |
| `JsxInline` / `JsxFlow` | literal string | no escape (raw JSX passthrough) |
| `Start(Tag)` | opening tag (`p`, `h1-6`, `blockquote`, `pre><code`, `ul`/`ol`, `li`, `table`*, `thead`, `tr`, `td`, `em`, `strong`, `del`, `a`) | attributes emitted verbatim; no escape today |
| `End(TagEnd)` | closing tag counterpart | — |
| `InlineMath` | `<span class="math-inline">{math}</span>` | no escape |
| `DisplayMath` | `<div class="math-display">{math}</div>` | no escape |
| `Rule` | `<hr />` | — |
| `HardBreak` | `<br />` | — |
| `SoftBreak` | `\n` | — |
| `FootnoteReference` | `<sup class="footnote-ref"><a href="#fn-x" ...>x</a></sup>` | ids/text unescaped (TODO) |
| `TaskListMarker` | `<input type="checkbox" ... />` | — |
\* Table cell alignment attributes are not yet emitted in JSX mode (future work).

## Escaping Policy (current)
- Escape only text nodes: `& < > { }` → `&amp; &lt; &gt; &#123; &#125;`.
- Do **not** escape JSX/HTML nodes (`JsxInline/JsxFlow/Html/InlineHtml`) — treated as trusted.
- Code blocks: wrap in `<code>` but do not escape inner text beyond text-level escaping.
- Attributes: currently written verbatim from events; no quoting/escaping guarantees (needs future tightening).

## Error / Fallback
- Unsupported tag combinations are silently skipped (no panic); matching is guarded by `matches_end`.
- No `dangerouslySetInnerHTML` path; everything is emitted as literal JSX strings.

## Open Questions
- Should attributes be HTML-escaped or JSX-brace-wrapped in a future pass?
- How to expose `runtime_import` / layout wrapping defaults without breaking existing NAPI callers?
- Should `<script>`/`<style>` be filtered or left as-is in JSX mode?

## Next Steps
- Add options struct to Rust and expose in NAPI/WASM with defaults.
- Expand table for table alignment, images (current behavior skips image rendering in JSX renderer).
- Add fixtures: text with braces/angles, HTML entities, nested JSX + Markdown mix, attributes with quotes.
