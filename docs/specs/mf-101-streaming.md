# MF-101 Streaming Rewriter Diagnostics

## 背景
Streaming Rewriter の挙動をデバッグする際、HTML 断片やリライトオプションの有効/無効を確認する手段が不足している。Codex が安全に変更できるよう、観測可能性を高める。

## オーナー / 期限
- Owner: Kenji (Core)
- Reviewer: Aya (Bindings)
- Target ship date: 2025-12-20 (aligns with Astro MVP Phase 2 kickoff)

## 要求
- `crates/core/src/streaming_rewriter.rs` にデバッグログ/統計を注入するためのフックを追加する。
- `RewriteOptions` に `emit_stats: bool` を追加し、`true` のときに加工済みバイト数と処理時間を返す。
- N-API 側でも同じ統計を `parseWithStats` に露出し、AVA テストで確認する。

## 完了条件
1. Rust 単体テストと `pnpm test` がすべて成功する。
2. `scripts/smoke-napi.mjs` で統計が表示されるスクリーンショット or ログを PR に添付する。
3. `AGENTS.md` の Build/Test セクションに新しい検証手順を追記する。

## 参考
- `docs/development/spec-driven-codex.md#codex-へのプロンプト例`
- `Backlog.md` の MF-101 行

## 受け入れテスト
1. `cargo test --package markflow-core streaming_rewriter::tests::stats_toggle` が統計のオン/オフを確認できる。
2. `pnpm test --filter markflow -- run parseWithStats smoke`（新規 AVA ケース）で Node バインディングが数値を返す。
3. `node ../../scripts/smoke-napi.mjs fixtures/core/markdown/table.md --emit-stats` 実行ログに `processingTimeMs` と `htmlBytes` の両方が表示される。
