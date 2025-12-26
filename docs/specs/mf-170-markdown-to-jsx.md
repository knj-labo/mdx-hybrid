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
