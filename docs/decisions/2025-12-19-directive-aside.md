# Directive → Aside transclusion (Auto-import)

- 対応 directive: `note`, `tip`, `info`, `caution`, `warning`, `danger` を `<Aside type="{name}">` に変換。
- タイトル決定: `:::name[Title]` の角括弧 > 属性 `title="..."` の順に優先。両方無ければタイトルなし Aside。
- Auto-import: 変換を行ったモジュール先頭へ `import { Aside } from '@astrojs/starlight/components';` を1回だけ追加（既存重複を避ける）。

## 実装状況メモ
- [x] パーサで directive を検出する場所とインターフェース（`directives.rs` で実装済み）
- [x] JSX 生成／HTML ストリームへの反映方法（`jsx_renderer.rs` および parse 関数群で実装済み）
- [x] NAPI/WASM への反映手順（`napi/src/lib.rs` で実装済み）
