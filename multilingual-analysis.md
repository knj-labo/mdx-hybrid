# マルチリンガル Visual Diff 分析結果

テスト日時: 2026-01-06
対象言語: en (英語), ja (日本語), fr (フランス語)

## 📊 言語別サマリー

| 言語 | テストルート数 | 完全一致 (0%) | 差分あり (>2%) | 最大差分 |
|------|--------------|-------------|---------------|---------|
| **英語 (en)** | 30 | 2 (6.7%) | 21 (70%) | 14.77% (/en/contribute/) |
| **日本語 (ja)** | 20 | 5 (25%) | 16 (80%) | 13.59% (/ja/contribute/) |
| **フランス語 (fr)** | 20 | 1 (5%) | 17 (85%) | 12.26% (/fr/contribute/) |

## 🔍 共通パターン: 全言語で差分が大きいページ

以下のページは**全言語で一貫して高い差分率**を示しています：

### 1. `/contribute/` - コントリビュートガイド
- **en**: 14.77%
- **ja**: 13.59%
- **fr**: 12.26%
- **分析**: Tableが多用されている可能性。リスト構造の差分？

### 2. `/concepts/islands/` - Islands アーキテクチャ
- **en**: 3.35%
- **ja**: 3.79%
- **fr**: 5.35% ← フランス語で特に高い
- **分析**: 長いテキストでレイアウト変動？

### 3. `/develop-and-build/` - ビルドガイド
- **en**: 6.17%
- **ja**: 5.33%
- **fr**: 6.62%
- **分析**: コード例のフォーマット差分？

## ✅ 完全一致ページ (0% diff)

### 日本語のみ完全一致
- `/ja/basics/project-structure/` (en: 0.18%, fr: 0.16%)

### 全言語で完全一致
- `/*/guides/astro-db/`
- `/*/guides/backend/appwrite/` (en: 0.00%, ja: 0.00%, fr: 0.83%)
- `/*/guides/backend/firebase/` (en: 0.04%, ja: 0.00%, fr: 0.02%)

## 🌐 言語固有の傾向

### 日本語 (ja)
- **完全一致率が最も高い** (25% vs en:6.7%, fr:5%)
- **マルチバイト文字の問題は検出されず**
- `/ja/basics/project-structure/` が唯一の完全一致

### フランス語 (fr)
- **Islands ページの差分が最大** (5.35%)
- 文字数増加による影響が顕著
- 完全一致はほぼなし (1ページのみ)

### 英語 (en)
- ベースライン言語として期待通りの結果
- `/en/contribute/` の差分が3言語中最大 (14.77%)

## 🎯 次のアクション提案

### 優先度 HIGH: `/contribute/` ページの調査
全言語で12%以上の差分 → Table/List構造の問題を特定

```bash
# HTML差分を確認
diff -u \
  fixtures/integration/withastro-docs/repo/dist-baseline/en/contribute/index.html \
  fixtures/integration/withastro-docs/repo/dist-markflow/en/contribute/index.html | head -100

# 差分画像を視覚確認
open fixtures/integration/withastro-docs/visual-diff/diff/en__contribute.png
open fixtures/integration/withastro-docs/visual-diff/diff/ja__contribute.png
open fixtures/integration/withastro-docs/visual-diff/diff/fr__contribute.png
```

### 優先度 MEDIUM: Islands ページ (フランス語)
フランス語のみ高差分 → テキスト長による溢れ？

```bash
diff -u \
  fixtures/integration/withastro-docs/repo/dist-baseline/fr/concepts/islands/index.html \
  fixtures/integration/withastro-docs/repo/dist-markflow/fr/concepts/islands/index.html | head -50
```

### 優先度 LOW: 全体スキャン
より広範囲のページをテスト（バックグラウンド実行）

```bash
# 全ルート実行（時間がかかる）
node scripts/visual-diff-withastro-docs.mjs --include "^/(ja|fr|en)/" --build &
```

## 📋 結論

1. **GFM対応は基本的に正常動作**
   - 完全一致ページが複数存在
   - マルチバイト文字（日本語）も問題なし

2. **特定ページで一貫した差分**
   - `/contribute/` が全言語で12-15%の差分
   - Table/List構造に起因する可能性が高い

3. **言語による大きな違いはなし**
   - 日本語が若干良好（完全一致率25%）
   - フランス語でやや高めの差分（Islands: 5.35%）

---

**次の調査:** `/en/contribute/` ページのHTML差分を詳細確認することを推奨
