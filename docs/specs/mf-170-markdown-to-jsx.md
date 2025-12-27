# MF-170: Markdown → JSX renderer passthrough

> Draft scaffold – fill in API, edge cases, and test plan.

## Scope
- Preserve raw JSX/MDX nodes through the Markdown pipeline and emit JSX source suitable for downstream bundlers.
- Honor code fence–aware import/export hoisting to keep module headers intact.
- `render_to_jsx` は先頭に import 行を含むことがあるため、下流は **行単位で import/空行のみ** を除去して本文を取り出す（文字数スライスは不可）。
- JSXブロック（先頭が大文字タグ/Fragment）の内側はMarkdownとして再パースし、JSX出力へ差し替える。
  - 行単位の検出でブロック開始/終了を判定し、コードフェンス内は検出対象から除外する。
  - JSX行は素通しし、Markdown行だけを再パースする（JSX混在でパーサが壊れるのを防ぐ）。
- JSX行のプレースホルダ挿入時は、元行のインデント/改行を保持してMarkdown構造（リスト/フェンス）を崩さない。
- プレースホルダ単独の段落（`<p>...placeholder...</p>`）は除去し、JSXタグが段落に包まれることで起きるタグ不整合を防ぐ。
  - 段落先頭がJSXブロック（大文字タグ/Fragment）の場合は `<p>` を除去し、JSXの不正ネストを避ける。
  - JSXブロック内Markdownは **共通インデントを除去せず**、元のネスト構造を維持したまま再パースする（リスト/Steps内の構造崩壊を防ぐ）。
- `<Steps>` 内のタイトなリスト項目は `<li><p>...</p></li>` に正規化し、Starlightの余白/整列が崩れないようにする。
  - ブロックJSX経路（`preprocess_jsx_block_lines`）で `<Steps>` を閉じる際にも同じ正規化を適用する。
  - Steps の最終HTMLで `<ol class="sl-steps">` を検出し、内側HTMLに対して正規化を適用する。
  - `<Steps>` 内の JSX ブロック（`<Tabs>` など）は、**直前の `<li>` の直前に挿入**し、`<li>` が存在しない場合は `<Steps>` 末尾に残す。
  - この挿入規則は `preprocess_jsx_block_lines` 経路でも適用する（トップレベル `<Steps>` を含む）。
- プレースホルダは Markdown の HTML ブロック判定を避けるため、プレーン文字列（`@@MF_JSX_N@@`）を用いる。
  - コードフェンス行は ` ```lang ` 形式に正規化し、**先頭インデントは保持**しつつ title などの付加属性のみ除去する。
  - JSX 内のコードフェンス開始/終了行は **trim せず元のインデントを保持**し、フェンスの解釈を崩さない。
- マルチパス解析の土台として、`crates/core/src/renderer/multipass.rs` に `Block` と `scan_blocks` を追加する。
  - JSX ブロックは **先頭 `<` + 大文字タグ** のみ対象とし、self-closing と同名タグのネストを深さカウントで正しく対応付ける。
  - コードフェンス内では JSX タグ検出を行わず、`<BUCKET_NAME>` などを Markdown として保持する。
  - インラインコード（バッククォート）内も JSX 判定をスキップし、コード内の `<...>` を誤検出しない。
  - JSXブロック検出は **行頭（インデント0〜3スペース）** のみに限定し、ネスト内（リスト/Steps内）の JSX は Markdown 側で処理する。
- JSXブロック処理は `render_blocks` 経由に切り替え、行ベースの JSX ブロック再パースは廃止する（Markdownブロックの処理は既存パイプを維持）。
- /@id の markflow モジュール取得が 500 を返す場合は、該当 JSX に **生のコードフェンス行（```json など）が残っている**可能性があるため、エラー行付近の生成断片を確認して原因を切り分ける。
  - 例: aws.mdx の `Expected "}" but found ":"` は ` ```json` 行が JSX に残存しているケース。

## Multipass Parsing (planned)
- JSX scanning builds a recursive `Block::JsxElement { children: Vec<Block> }` tree (no raw `inner` string) so later passes can normalize sibling placement.
- The multipass scanner lives in `crates/core/src/renderer/multipass.rs` and is exported from `renderer::mod`.
- A `scan(input: &str) -> Vec<Block>` entrypoint is defined; initial stub returns a single Markdown block.
- A minimal test locks the stub behavior until the recursive scan replaces it.
- Next step: `scan` will build mixed `Markdown`/`Code`/`JsxElement` trees using recursive descent.
- `Block::Markdown` groups contiguous non-JSX text, `Block::Code` stores code fences/indented code (no JSX scanning), and `Block::JsxElement` stores name/attrs/children/self-closing.
- `scan` returns an empty Vec for empty input (no empty Markdown block).
- `multipass.rs` keeps `#[cfg(test)] mod tests` at the end of the file to avoid items-after-test-module lint.
- `scan` uses a helper to append non-empty Markdown slices so future splitting stays centralized.
- `find_byte` (byte search helper) supports low-level tag scanning for multipass.
- `find_tag_end` is a minimal helper that finds the next `>` (attribute quoting handled later).
- `is_self_closing` is a minimal helper that checks for a trailing `/` before `>` (whitespace-aware handling comes later).
- `is_name_char` only accepts ASCII alnum plus `-`, `:`, and `_` for tag names.
- `is_tag_terminator` treats whitespace, `/`, and `>` as the end of a tag name.
- `parse_open_tag` returns `(name, attrs, open_end_index)` where attrs is the trimmed slice up to `>`; invalid tag names fail.
- `is_close_tag` checks for a matching `</name` followed by a tag terminator.
- `scan` delegates to `scan_nodes(input, cursor, until_tag)` so recursive descent can reuse the same entrypoint.
- `scan_nodes` is structured as a cursor-driven loop and returns when the matching closing tag is encountered.
- If no `<` is found, `scan_nodes` emits the remaining slice as a single Markdown block.
- When a `<` is found, the preceding slice is emitted as Markdown before inspecting the tag.
- If `<` does not start a valid tag, it is emitted as Markdown and the cursor advances by 1.
- When a valid open tag is found, the cursor advances to just after the `>` before emitting blocks.
- Open tags are checked for self-closing (`/>`) during scanning (used when emitting JSX blocks).
- Self-closing tags emit `Block::JsxElement` with empty children immediately.
- Tests cover self-closing JSX element emission with trimmed attrs.
- Non-self-closing tags that lack a matching close fallback to emitting `<` as Markdown.
- Non-self-closing tags with a matching close emit `Block::JsxElement` with recursively scanned children.
- Tests cover non-self-closing JSX elements with child Markdown.
- After emitting a `JsxElement`, `scan_nodes` advances past the closing tag without emitting extra Markdown.
- `attrs` are stored with leading/trailing whitespace trimmed.
- Self-closing attrs tests assert the trimmed value (e.g., `/`).
- Tests cover nested same-tag matching via recursion.
- Tests cover nested self-closing tags alongside content.
- `scan_nodes` begins scanning children immediately after the open tag (`open_end + 1`).
- Tags without attributes store an empty string for `attrs`.
- `scan_nodes` breaks if the cursor does not advance, preventing infinite loops on malformed input.
- `scan_nodes` splits Markdown and JSX at `<` boundaries, emitting JSX as `Block::JsxElement`.
- Adjacent Markdown segments may be emitted as separate blocks (no coalescing yet).
- Fence detection helpers (`is_line_start`/`is_fence_start`) are added to avoid parsing JSX inside code fences.
- Fence end detection helper (`find_fence_end`) is added to locate closing fences.
- Tests cover basic fence start/end detection helpers.
- `scan_nodes` emits `Block::Code` for fenced code blocks to avoid JSX scanning inside fences.
- Unclosed fences emit the first marker as Markdown and then stop scanning, returning the rest as Markdown.
- Tests cover emitting `Block::Code` for fenced blocks.
- `Block::Code` spans from the opening fence through the end of the closing fence line (including newline).
- Indented code blocks are not treated specially by multipass yet.
- JSX scanning resumes after a fenced code block ends.
- `<` inside fenced code blocks is never treated as JSX.
- Tests cover unclosed fence fallback behavior.
- `find_fence_end` returns the position of the closing fence line start.
- Tests cover `~~~` fence detection.
- Tests cover ignoring JSX inside fences.
- Fence detection is restricted to true line starts (indented fences not handled yet).
- `scan_nodes` checks for fences before JSX tags.
- Missing close tags fall back to treating `<` as Markdown.
- `scan_nodes` returns `(blocks, cursor, closed)` where `closed=true` indicates the until tag was found.
- `scan_nodes` may return `cursor == input.len()` when it reaches the end of the input.
- `scan_nodes` returns immediately when it encounters the `until_tag` close.
- If a close tag is detected but `find_tag_end` fails, `scan_nodes` returns with `closed=false`.

## Open Questions
- JSX escaping rules for text nodes (current minimal escape: `& < > { }`).
- How to surface options (e.g., runtime imports, layout wrapping) while keeping a streaming interface.
- Alignment with NAPI/WASM bindings (naming and return shape).
- Whether to serialize a JSON AST for performance (deferred).

## Next Steps
- Define renderer API surface (options struct, return type).
- Enumerate fixtures for JSX-in-markdown edge cases (props, children, spread, comments, fragments).
- Decide on HTML/JSX interop rules (when to `dangerouslySetInnerHTML` vs. literal text).
- Track JSON AST serialization as a future optimization task.

Note: lint cleanup (clippy warnings) only; no behavior changes.
