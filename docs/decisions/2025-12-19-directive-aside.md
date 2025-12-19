# Directive → Aside transclusion (Auto-import)

- 対応 directive: `note`, `tip`, `info`, `caution`, `warning`, `danger` を `<Aside type="{name}">` に変換。
- タイトル決定: `:::name[Title]` の角括弧 > 属性 `title="..."` の順に優先。両方無ければタイトルなし Aside。
- Auto-import: 変換を行ったモジュール先頭へ `import { Aside } from '@astrojs/starlight/components';` を1回だけ追加（既存重複を避ける）。

## 未着手メモ
- パーサで directive を検出する場所とインターフェース
- JSX 生成／HTML ストリームへの反映方法
- NAPI/WASM への反映手順
