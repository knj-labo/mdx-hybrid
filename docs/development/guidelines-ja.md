# リポジトリガイドライン

_この文書は [AGENTS.md](../../AGENTS.md) のクイックスタートを補完します。コントリビュータ向けルールを更新する際は両方を同期してください。_

## プロジェクト構成とワークスペース
Markflow は `crates/core`（Rust パーサー + ストリーミングリライタ）、`crates/napi`（Node バインディング）、`crates/wasm`（ブラウザ向けバインディング）で構成された Cargo ワークスペースです。テストは各クレート直下（`crates/core/src/**/*.rs`、`crates/napi/tests/*.test.js`）に置きます。共通リソースは `fixtures/`（Markdown サンプル）、`benchmarks/`（性能測定）、`samples/`（統合デモ）、`scripts/`（スモークツール）、`docs/`（アーキテクチャ資料）にまとめ、計画立案時は `ROADMAP.md` を唯一のロードマップとして参照してください。

## ビルド・テスト・開発コマンド
コミット前に `cargo fmt --all && cargo clippy --workspace --all-targets` を必ず実行し、`cargo test --workspace` で Rust のテストスイート（`crates/core/src/lib.rs` のインラインテストを含む）を確認します。Node バインディングは `crates/napi` で `pnpm install --filter markflow` を行い、`pnpm run build` で N-API バイナリをビルド、`pnpm test` で AVA テストを実行し、`node ../../scripts/smoke-napi.mjs fixtures/core/markdown/hello.md` でエンドツーエンドの結果を確認します。性能証跡が必要な場合のみ `cargo bench -p markflow-core` を追加してください。

## パーサーとグルーの期待値
コアパーサーはプル型イベントを維持し、PipeAdapter と `lol_html` リライタへそのままストリーミングする設計を守ります（AST を構築しない）。公開 API 名（`parse`、`parseWithOptions`、`parseWithStats`）を揃え、ドキュメントとバインディングの同期を崩さないでください。アダプタやリライタを変更する際は、バッファサイズや遅延読み込み属性などストリーミングの振る舞いをコメントで明示し、WASM/N-API クレートが追従できるようにします。

## コーディングスタイルと命名
ワークスペースは Rust 2024 と `rustfmt` デフォルトを採用しています。モジュール単位のファイル構成（`streaming_rewriter.rs`、`markdown_adapter.rs` など）と snake_case 識別子を徹底し、各クレートで `#![deny(missing_docs)]` を有効化してストリームパイプライン内での構造体の役割を記述します。`crates/napi` の JavaScript/TypeScript は純粋な ESM で、関数は camelCase、エクスポートクラスは PascalCase、ファイル名はケバブケースに揃えます。

## テストとベンチマーク
Rust モジュールではテーブル駆動テストを使い、`fixtures/core/markdown` の入力で HTML 断片を検証します。AVA テストは `crates/napi/tests` に `<feature>.test.js` 形式で配置し、Rust と同じシナリオを反映させます。PR を出す前に `cargo test --workspace`、`pnpm test`、そして 1 つ以上の `.md` でスモークスクリプトを実行してください。性能改善を主張する場合は `benchmarks/results.md` か PR の CLI ログに結果を記録します。

Phase 2 の統合作業では `fixtures/integration/astro-harness` も忘れずに。初回のみ `pnpm install` を実行し、`node scripts/run-astro-harness.mjs markflow`（Markflow 経路）と `node scripts/run-astro-harness.mjs baseline`（既存スタック）でビルドが通ることを確認してください。パフォーマンス差が欲しい場合は `node scripts/compare-astro-harness.mjs --runs=5` を実行すると Markflow と baseline の平均ビルド時間が表示されます。

## コミットと PR ワークフロー
`feat: apply format` や `fix: clippy regressions` のような軽量な Conventional Commits を使います。PR 説明には影響したクレート、実施した手動検証（実行コマンドや使用フィクスチャ）、クローズする ROADMAP.md 項目を必ず記載してください。レンダリング結果が変わる場合のみスクリーンショットを添付し、それ以外はベンチマークやスモークテストのログを貼ります。バインディングの生成物やシークレットはコミットしないでください（`.env.local` を利用）。
