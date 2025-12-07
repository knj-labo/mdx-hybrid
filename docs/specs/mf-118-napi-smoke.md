# MF-118 N-API Smoke Coverage Parity

## 背景
`scripts/smoke-napi.mjs` は手動実行されるのみで、自動テストとの差分が不明。Codex が保守する際も、どこまで検証すべきか分かりづらい。

## オーナー / 期限
- Owner: Aya (Bindings)
- Reviewer: Kenji (Core)
- Target ship date: 2026-01-10（Astro docs CI hardening）

## 要求
- `crates/napi/tests/` に smoke シナリオを追加し、`parse`, `parseWithOptions`, `parseWithStats` が全て同じフィクスチャを処理するテストを用意する。
- `AGENTS.md` の Testing Guidelines に「AVA smoke テスト + スクリプト実行」の手順を明記する。
- `Backlog.md` のステータス更新プロセス（Review → Done）の説明を補強する。

## 完了条件
1. `pnpm test` に新しい AVA テストが含まれている。
2. `node ../../scripts/smoke-napi.mjs fixtures/core/markdown/hello.md` のログを PR に貼る。
3. ドキュメント更新（AGENTS.md, docs/development/guidelines*.md）が含まれている。

## メモ
必要に応じて `fixtures/core/markdown/` に追加サンプルを置いてよい。

## 受け入れテスト
1. `pnpm test --filter markflow -- match smoke-parity` が `parse`, `parseWithOptions`, `parseWithStats` の 3 ケースを通過する。
2. `node ../../scripts/smoke-napi.mjs fixtures/core/markdown/hello.md` の実行結果を CI artifact としてアップロードできる。
3. `AGENTS.md` と `docs/development/guidelines*.md` に「AVA smoke + スクリプト」を追記した差分が存在する。
