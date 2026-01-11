# MF-171: JSX renderer API & escaping rules

Status: Draft (reinforced with current implementation)

## Goal
Define the Rust/NAPI/WASM API surface and the escaping/serialization rules for the Markdown→JSX renderer so downstream bundlers receive predictable JSX with raw JSX passthrough.

## API Surface (proposed)
- Rust:
  - `render_to_jsx(input: &str) -> Result<String, MarkflowError>`
  - `render_to_jsx_with_options(input: &str, options: JsxOptions) -> Result<String, MarkflowError>`
  - `JsxOptions { rewrite_options: RewriteOptions, components: ComponentRegistry }`
    - `ComponentRegistry` handles built-in JSX components (`Steps`, `FileTree`) plus optional plugins and import mappings.
    - Plugins now receive a streaming `ScanEvent` iterator via `render_stream`.
- NAPI:
  - `renderToJsx(input: string, options?: JsxRenderOptions): string`
  - `JsxRenderOptions { rewrite?: RewriteConfig, componentImports?: { name: string, import: string }[] }`
- WASM:
  - `render_jsx(input: string, enable_directives?, enable_hoist?, enable_smartypants?, enable_components?) => string` (legacy signature)
  - `render_jsx_with_options(input: string, options?: JsxRenderOptions) => string`
  - `JsxRenderOptions { enableDirectives?, enableHoist?, enableSmartypants?, enableComponents?, enforceImgLoadingLazy?, componentImports?: { name: string, import: string }[] }`

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

## Multipass Integration (current)
- `render_to_jsx` consumes a streaming `ScanEvent` iterator (`Markdown`, `Code`, `JsxOpen`, `JsxClose`) instead of building a `Block` tree.
- `ScanEvent::Code` is rendered through the event pipeline so fenced code becomes `<pre><code>`, while still skipping JSX scanning inside fences.
- `JsxOpen`/`JsxClose` events are streamed; component plugins can consume until their matching close tag.
- `<Steps>` renders Markdown fragments first, then injects non-Markdown fragments before the last `</li>` (or appends if no list item exists).

## Escaping Policy (current)
- Escape only text nodes: `& < > { }` → `&amp; &lt; &gt; &#123; &#125;`.
- Do **not** escape JSX/HTML nodes (`JsxInline/JsxFlow/Html/InlineHtml`) — treated as trusted.
- Code blocks: wrap in `<code>` but do not escape inner text beyond text-level escaping.
- Attributes: currently written verbatim from events; no quoting/escaping guarantees (needs future tightening).

## Error / Fallback
- Unsupported tag combinations are silently skipped (no panic); matching is guarded by `matches_end`.
- No `dangerouslySetInnerHTML` path; everything is emitted as literal JSX strings.

## Debug
- `MARKFLOW_DEBUG_BINDING=1` enables vite-plugin-markflow logs for binding source (package/fallback/provided) and `NAPI_RS_NATIVE_LIBRARY_PATH`.

## Hoisted Imports (compile_ir)
- Only the leading contiguous `import`/`export` lines are hoisted.
- Leading blank lines and JS-style comments (`//`, `/*`) are allowed and do not stop hoisting, but are not hoisted themselves.

## Component-specific Markdown Handling
- `<FileTree>`: Markdown children are dedented by one indentation level (4 spaces or 1 tab) before rendering so list items inside list contexts are not treated as code blocks.
- `<FileTree>`: Markdown is rendered as a single buffer and only the first `<ul>` root is kept to satisfy the component contract.
- `<FileTree>`: The extracted `<ul>` includes the full closing tag (`</ul>` through `>`), otherwise the original HTML is preserved.
- `<Steps>`: Multiple rendered `<ol>` fragments are merged into a single list; non-list HTML fragments are inserted into the latest `<li>` to keep exactly one root `<ol>`.
- `<Steps>`: Markdown children are dedented by one indentation level before rendering so fenced blocks remain fences (not indented code).

## Usage Examples

### NAPI (Node)
```ts
import { renderToJsx } from 'markflow-napi'

const jsx = renderToJsx('<Badge>Hi</Badge>', {
  componentImports: [
    { name: 'Badge', import: "import Badge from './Badge.astro';" },
  ],
  rewrite: { enableHoist: true },
})
```

### WASM (Browser/Edge)
```ts
import { render_jsx_with_options } from './markflow_wasm'

const jsx = render_jsx_with_options('<Badge>Hi</Badge>', {
  componentImports: [
    { name: 'Badge', import: "import Badge from './Badge.astro';" },
  ],
  enableHoist: true,
})
```

## Open Questions
- Should attributes be HTML-escaped or JSX-brace-wrapped in a future pass?
- How to expose `runtime_import` / layout wrapping defaults without breaking existing NAPI callers?
- Should `<script>`/`<style>` be filtered or left as-is in JSX mode?

## Next Steps
- Consider exposing plugin hooks beyond import mappings (user-defined child transforms).
- Expand table for table alignment, images (current behavior skips image rendering in JSX renderer).
- Add fixtures: text with braces/angles, HTML entities, nested JSX + Markdown mix, attributes with quotes.
