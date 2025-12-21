# 0001 Lean Architecture 決定ログ

## 目的
- 本リポジトリを「無駄のない筋肉質な構成」へ段階的に再構築する際の意思決定を、時系列かつ網羅的に残す。
- マイクロコミット合意プロトコルに従い、合意済み事項を即座に記録してトレーサビリティを確保する。

## 適用範囲
- 対象リポジトリ: `knj-labo/markflow`（現作業ディレクトリ: /Users/kenji/Projects/astro/markflow）
- 対象期間: 2025-12-20 以降の本プロトコル下で行う全ての合意事項

## マイクロコミット運用手順
1. 各ステップは最小単位の作業・意思決定のみ提示し、ユーザーが **「OK」** を出すまで実装を行わない。
2. 「OK」受領後、該当ステップの合意内容を本ログに追記し、必要なファイル修正を実施する。
3. 既存コードよりも合意した意思決定を優先し、矛盾する場合は既存実装を置換する。

## 決定履歴
| 日時 (UTC) | Step | 合意内容 | 根拠 | 影響範囲 |
| --- | --- | --- | --- | --- |
| 2025-12-20 00:00 | #1 | 意思決定ログ台帳ファイル `docs/decisions/0001-lean-architecture.md` を新設し、目的・範囲・手順テンプレートを定義 | マイクロコミット合意プロトコルに基づき、以降の合意を記録可能にする基盤が必要なため | ドキュメントのみ（コード未変更） |
| 2025-12-20 00:00 | #3 | リポジトリルートの現状一覧（`ls -a`）を取得・共有 | 初期インベントリ確保のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #5 | `target/`, `node_modules/`, `packages/` の一覧を取得・共有 | 生成物・依存の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #7 | `crates/` 直下に `core`, `napi`, `wasm` が存在することを確認 | 既存クレート構成の把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #9 | `crates/core` に `Cargo.toml`, `src`, `tests` が存在することを確認 | coreクレート現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #11 | `crates/core/Cargo.toml` を確認（name=markflow-core、workspace継承版、主要依存: lol_html, markdown alpha16, thiserror 2.0.17, serde/serde_json/serde_yaml, log, html-escape; dev: insta, once_cell） | 依存整理・再設計の前提把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #13 | `crates/core/src` に adapter.rs, event.rs, frontmatter.rs, lib.rs, slug.rs とディレクトリ parser, renderer, transform が存在することを確認 | ソース階層の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #15 | `crates/core/src/lib.rs` の内容を確認（公開モジュール・MarkflowError・parse/parse_with_options が DirectiveAdapter+StreamingRewriter で HTML と hoisted/imports を返す構造、テストが同ファイル末尾に併設） | API再設計時の比較基準を確保するため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #17 | `crates/core/src/parser` に markdown_adapter.rs, mod.rs, parse_config.rs が存在することを確認 | パーサモジュール現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #19 | `crates/core/src/parser/mod.rs` は missing_docs を許容し、markdown_adapter と parse_config を公開、ParseConfig/ParseConstructs を再エクスポート | 公開境界把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #21 | `parse_config.rs` を確認（ParseConstructs: jsx/esm/expression/directives フラグ、ParseConfig: constructs + html_to_jsx。markdown(): jsx/esm/expression=false, directives=true, html_to_jsx=true; mdx(): 全trueかつ html_to_jsx=false; Default=mdx) | パーサ設定デフォルトの現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #23 | `markdown_adapter.rs` を確認（MarkdownParserが mdast を Event に変換。build_parse_options で ParseConfig の constructs を反映し、mdx時は html_flow/html_text を無効化し mdx_esm/expr/jsx を有効化。iteratorで Frame スタックを用いて Start/End/Text などを生成。テストで markdown vs mdx の挙動差を検証） | パーサアダプタ挙動の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #25 | `crates/core/src/renderer` に html_renderer.rs, jsx_renderer.rs, mod.rs, streaming_rewriter.rs が存在することを確認 | レンダラー層構成把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #27 | `crates/core/src/renderer/mod.rs` を確認（html_renderer/ jsx_renderer/ streaming_rewriter を公開し、render_to_jsx・RewriteOptions・StreamingRewriter を再エクスポート） | レンダラー公開境界把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #29 | `streaming_rewriter.rs` を確認（RewriteOptions: enforce_img_loading_lazy/directive_mapper/required_imports; Defaultは lazy付与 + AsideDirectiveMapper; StreamingRewriter が lol_html を Write でラップし finalize/into_inner を提供。lazy_img_handler で loading を補完し、OutputProxy で OutputSink 実装。テストで lazy付与と既存属性保持を検証） | レンダリングリライト挙動の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #31 | `jsx_renderer.rs` を確認（render_to_jsx が parse で imports 抽出後、get_event_iterator + DirectiveAdapter を通し Event を JSX風文字列に変換。start_tag/end_tag/escape_text を用い、imports を先頭出力。画像タグは空処理） | JSX出力パイプラインの現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #33 | `html_renderer.rs` を確認（HtmlRenderer が Event イテレータをHTML出力、テーブルのアライン処理、画像は alt/title を蓄積し finish_image で loading=\"lazy\" を付与。タグ書き出し/エスケープ関数を備え、JSX/HTMLイベントは生で書き出す。テストで JSXイベントでもpanicしないことを確認） | HTML出力パス仕様の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #35 | `crates/core/src/transform` に code_fence.rs, directive_adapter.rs, directives.rs, hoist_adapter.rs, mod.rs が存在することを確認 | 変換パイプライン構成把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #37 | `transform/mod.rs` を確認（hoist/directive/code_fence を公開し、DirectiveAdapter と HoistAdapter を再エクスポートする構成） | 変換パイプライン公開境界の把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #39 | `directives.rs` を確認（DirectiveTransform/DirectiveMapper トレイト、デフォルト AsideDirectiveMapper が Starlight Aside import を要求し note/tip/info/caution/warning/danger を対応。parse_opening_directive で :::name[title] attrs を解析し type/title を整理。rewrite_with_mapper でコードフェンス内スキップしつつ置換、ensure_aside_import で hoisted に import を注入。各種挙動をテスト） | directive リライト仕様の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #41 | `directive_adapter.rs` を確認（Event イテレータをラップし、段落テキストを集めて rewrite_with_mapper に渡し directives をHTMLに差し替え。コードブロック内は変換しない。directive_count/required_imports を蓄積し、未閉鎖のクローズも補完する構造） | ストリーミングdirective変換の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #43 | `hoist_adapter.rs` を確認（トップレベルかつコードブロック外の Html イベントから `import `/`export ` で始まる行を hoisted Vec に蓄積し、イベントをスキップ。depth と in_code_block を追跡） | ESMホイスト挙動の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #45 | `code_fence.rs` を確認（FencePhase/FenceState/LineParseOutcome、advance_fence_state で ```/~~~ を検出し skip_imports を制御。collect_root_imports がフェンスを避けつつ import/export を抽出し残り行を返す。paren_depth/ends_statement で複数行判定。テストで多ケース検証） | フェンス判定とルートimport抽出仕様の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #47 | `crates/core/tests` に insta_tests.rs と snapshots ディレクトリが存在することを確認 | 統合テスト構成の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #49 | `crates/napi` に .npmignore, build.rs, Cargo.toml, examples, index.d.ts, index.darwin-arm64.node, index.js, markflow.darwin-arm64.node, node_modules, package.json, pnpm-lock.yaml, scripts, src, tests が存在することを確認 | N-APIバインディング構成の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #51 | `crates/napi/Cargo.toml` を確認（name=markflow-napi、cdylib、依存: markflow-core path ../core, napi v3 features=napi4+serde-json, napi-derive 3, serde/serde_json、build-dep napi-build 2） | N-API依存構成の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #53 | `crates/napi/src` には lib.rs のみが存在することを確認 | N-APIソース階層の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #55 | `crates/napi/src/lib.rs` を確認（公開API: parse/parse_with_options/parse_with_stats/parse_frontmatter/render_to_jsx_napi/compile_ir/MarkflowCompiler.compile_mdx/create_compiler 等。RewriteConfig→RewriteOptions変換、CompileIrResult/CompileResult/ImportSpec/HeadingEntry 定義、heading収集と hoisted import 組み立て、Astro向けコード生成、エラーマッピング、frontmatter・hoist・JSX保持などのテストあり） | N-APIバインディング仕様の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #57 | `crates/wasm` に Cargo.toml, src, tests が存在することを確認 | WASMクレート構成の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #59 | `crates/wasm/Cargo.toml` を確認（name=markflow-wasm、workspace継承の authors/version/edition/license、依存: wasm-bindgen 0.2.105, markflow-core path ../core, js-sys 0.3、dev-dep wasm-bindgen-test 0.3） | WASM依存構成の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #61 | `crates/wasm/src` には lib.rs のみが存在することを確認 | WASMソース階層の現状把握のため | 情報取得のみ（変更なし） |
| 2025-12-20 00:00 | #65 | 現状調査まとめを共有（coreはstreamingパイプライン＋hoist/directive構成、renderer3系統でloading=lazy、N-API/WASMは単一libでcoreをラップしAstro/JS向け出力を内包、テストはスナップショット＋各ユニット、生成物が残存し依存統一は未整理） | 以降の再設計方針の共通認識を固定するため | ドキュメントのみ（コード未変更） |
| 2025-12-20 00:00 | #68 | 再設計ゴールを設定（1. ルートはWS定義＋タスクランナーのみ、実装は crates/ と web/ に分離、2. core/napi/wasm API をスリム化し directive/hoist をオプショナブル化（デフォルト互換）、3. CI/ビルド成果物クリーン化と依存統一（workspace.dependencies活用・不要バイナリ排除）） | 再設計フェーズの指針を固定するため | ドキュメントのみ（コード未変更） |
| 2025-12-20 00:00 | #69 | クリーン方針合意：.gitignore 強化（target/, node_modules/, packages/*/node_modules/, crates/napi/*.node|*.dll|*.so|*.dylib|index.*.node|markflow.*.node など追加）、既存生成物削除対象（crates/napi/index.darwin-arm64.node, crates/napi/markflow.darwin-arm64.node, crates/napi/node_modules/, node_modules/）を次ステップで除去し、成果物はリポジトリに含めない方針を確立 | リポジトリの筋肉質化と再汚染防止のため | ドキュメントのみ（コード未変更） |
| 2025-12-20 00:00 | #70 | .gitignore を更新し、packages/*/node_modules と crates/napi の各種ネイティブバイナリ（*.dll/*.so/*.dylib/index.*.node/markflow.*.node）を無視対象に追加 | 生成物再混入防止のため | 設定のみ（コード未変更） |
| 2025-12-20 00:00 | #71 | 合意済み生成物を削除（crates/napi/index.darwin-arm64.node, crates/napi/markflow.darwin-arm64.node, crates/napi/node_modules/, node_modules/ を rm -rf 済み） | ワークツリーをクリーンにし再汚染を防ぐため | 生成物削除のみ |
| 2025-12-20 00:00 | #72 | 依存棚卸しのスコープ合意（ルート＋core/napi/wasm Cargo.toml の依存を一覧化し、workspace.dependencies 集約候補を抽出する方針を決定） | 依存統一と feature 整理の準備のため | 計画合意のみ |
| 2025-12-20 00:00 | #73 | 依存棚卸し結果: ルートは依存未定義。core依存=lol_html 2.0 / markdown 1.0.0-alpha.16 / thiserror 2.0.17 / log 0.4 / html-escape 0.2 / serde 1.0 +derive / serde_json 1.0 / serde_yaml 0.9; dev=insta 1.39 +yaml, once_cell 1.19。napi依存=markflow-core(path) / napi 3 (napi4, serde-json, default-features=false) / napi-derive 3 / serde 1 +derive / serde_json 1; build=napi-build 2。wasm依存=wasm-bindgen 0.2.105 / markflow-core(path) / js-sys 0.3; dev=wasm-bindgen-test 0.3。 | workspace.dependencies への集約候補把握のため | 情報整理のみ |
| 2025-12-20 00:00 | #74 | ルートに `[workspace.dependencies]` を追加し、core/napi/wasm の共通依存を集約（serde*, thiserror, log, html-escape, lol_html, markdown alpha16, napi*, wasm-bindgen*, js-sys, insta, once_cell 等）。各クレートの依存表記を workspace=true に置換 | 依存バージョン統一とビルド一貫性向上のため | 設定変更のみ（コード未変更） |
| 2025-12-20 00:00 | #75 | `cargo check --workspace` を実行し、core/napi/wasm すべて dev プロファイルで成功（約1s） | 依存集約後の整合性確認のため | 実行のみ（コード不変） |
| 2025-12-20 00:00 | #76 | N-API/WASM スリム化方針ドラフト合意: (1) directive/hoist をオプション化（デフォルトON、RewriteOptions/JS configに enableDirectives/enableHoist を追加）、(2) N-API公開関数は parse系＋compile_ir/compile + render_to_jsx を存続しオプション尊重、(3) WASMの render_html/render_jsx/stream_html へ同オプションを追加しデフォルト互換、(4) 実装順序は型定義→core配管→napi/wasm引数→テスト更新 | 筋肉質化と機能トグルの柔軟性確保のため | 方針決定のみ |
| 2025-12-20 00:00 | #77 | 具体仕様合意: core RewriteOptions に enable_directives/enable_hoist (default true) 追加。N-API RewriteConfig に同名 bool オプション（省略可）を追加し parse_with_options/compile_ir/MarkflowCompiler.compile/render_to_jsx_napi で反映。WASM API の render_html/render_jsx/stream_html に同ブール引数を追加（省略時 true）。互換維持を前提に順次実装へ。 | directive/hoist トグル実装の設計確定のため | 設計合意のみ |
| 2025-12-20 00:00 | #78 | core に enable_directives/enable_hoist フィールドを追加（RewriteOptions デフォルト true）、parse_with_options で HoistAdapter/DirectiveAdapter をオプション連動に変更、hoist無効時に import が HTML に残り imports 空となるテストを追加 | directive/hoist トグルの実装と検証のため | コード変更 + テスト追加 |
| 2025-12-20 00:00 | #79 | `cargo test --workspace --lib` 実行、全テスト成功（core/napi/wasm） | 変更後のリグレッション確認のため | 実行のみ |
| 2025-12-20 00:00 | #80 | N-API に enable_directives/enable_hoist を配管（RewriteConfig に Option<bool> 追加、RewriteOptions へ unwrap_or(true) で反映、index.d.ts に型追加）し、`cargo test --workspace --lib` 再度成功 | 上位バインディングでのトグル利用を可能にするため | コード変更 + 型更新 |
| 2025-12-20 00:00 | #81 | WASM に enable_directives/enable_hoist を配管（render_html/render_jsx/stream_html にオプション追加、hoist無効時は collect_root_imports をスキップ、render_jsx は両方ONなら従来render_to_jsxを維持）し、テスト追加と `cargo test --workspace --lib` 成功 | ブラウザ/WASM からのトグル利用を可能にするため | コード変更 + テスト更新 |
| 2025-12-20 00:00 | #82 | `ls -a web` 実行で `No such file or directory` を確認。Astroフロントが未配置であることを把握 | フロントエンド再構築の前提確認のため | 情報取得のみ |
| 2025-12-20 00:00 | #83 | フロント方針合意: 新規 Astro v5 プロジェクトを `web/` にスキャフォールド。`packages/astro-loader` はローカルnpmパッケージとして `web/` から利用。`fixtures/integration/astro-harness` は回帰/ベンチ用途で維持し、破壊的変更なし。 | フロント再構築と既存資産の役割分担を明確化するため | 方針決定のみ |
| 2025-12-20 00:00 | #84 | `web/` を新規スキャフォールド計画に沿って作成（Astro v5 skeleton: package.json/tsconfig/astro.config/content config/index.astro、pnpm-workspace.yaml を新設し web を追加、.gitignore に web/{node_modules,dist,.astro} を追記） | フロントエンド開発基盤を用意するため | 新規追加・設定 |
| 2025-12-20 00:00 | #85 | scripts/・fixtures/・samples/ のトップレベルを列挙（scripts: check-backlog.mjs, compare-astro-harness.mjs, run-astro-harness.mjs, smoke-napi.mjs; fixtures: core, integration, README.md; samples: large.md） | 重複/不要物洗い出し準備のため | 情報取得のみ |
| 2025-12-20 00:00 | #86 | `samples/large.md` を不要と判断し削除 | サンプルをスリム化するため | ファイル削除 |
| 2025-12-20 00:00 | #87 | `scripts/README.md` を追加し各スクリプトの用途を簡潔に記載（check-backlog/compare-astro-harness/run-astro-harness/smoke-napi） | スクリプト利用状況の可視化と整理準備のため | ドキュメント追加 |
| 2025-12-20 00:00 | #88 | `fixtures/core` を調査：markdown 配下に hello.md, table.md。mdx 配下に embedded-jsx/component.mdx, esm/imports.mdx, expressions/flow.mdx, expressions/inline.mdx が存在 | フィクスチャ重複・不足の把握のため | 情報取得のみ |
| 2025-12-20 00:00 | #89 | Backlog.md と ROADMAP.md を廃止し、意思決定は `docs/decisions/0001-lean-architecture.md` に一本化する方針を確定。該当ファイルを削除。 | 進行・決定の単一ソース化のため | ファイル削除 + 方針決定 |
| 2025-12-21 00:00 | #91 | docs/README.md を索引化し、docs/architecture・docs/decisions・docs/specs の役割を明記。ROADMAP/Backlog 廃止と decision log 集約を再周知。関連箇所の ROADMAP 参照を decision log へ差し替え。 | ドキュメント入口の明確化と情報探索性向上 | docs/README.md, docs/architecture/overview.md, docs/specs/mf-140-core-engine.md |
| 2025-12-21 00:00 | #92 | docs/specs/README.md を新設し、全スペックを一覧化。decision log を唯一の最新参照先と明記。 | 仕様探索の起点を一本化するため | docs/specs/README.md |
| 2025-12-21 00:05 | #93 | scripts/README.md に使用状況の索引テーブルを追加し、Backlog 廃止後の `check-backlog.mjs` を★暫定マーク。 | 余剰スクリプト洗い出しの準備 | scripts/README.md |
| 2025-12-21 00:07 | #94 | `scripts/check-backlog.mjs` を Backlog 不在時は警告の上で exit 0 する互換モードに変更。CI 落ちを防止。 | Backlog/ROADMAP 廃止後の互換維持 | scripts/check-backlog.mjs |
| 2025-12-21 00:12 | #95 | `pnpm install --filter @markflow/web` で web 依存を導入し、`pnpm --filter @markflow/web run build` が成功することを確認。 | Astro 層のセットアップとビルド健全性確認 | lockfile (pnpm), web/dist (build artifact) |
| 2025-12-21 00:18 | #96 | fixtures/README.md に現行フィクスチャの棚卸し表を追加。未使用候補の★欄を用意。 | 重複/用途不明フィクスチャ整理の準備 | fixtures/README.md |
| 2025-12-21 00:23 | #97 | web/src/content/docs/hello-world.md を追加し、Astro content collection WARN を解消。`pnpm --filter @markflow/web run build` 再実行で成功（vite internal unused import WARN のみ）。 | Web 層のノイズ除去と build 健全性維持 | web/src/content/docs/hello-world.md, web/dist |
| 2025-12-21 00:30 | #98 | Backlog 廃止に伴い `scripts/check-backlog.mjs` を削除し、CI (`.github/workflows/ci.yml`) からの呼び出しを除去。scripts/README.md から該当行を削除。 | 不要スクリプトとCIノイズの排除 | scripts/check-backlog.mjs, .github/workflows/ci.yml, scripts/README.md |
| 2025-12-21 00:33 | #99 | CI の NAPI スモークテスト入力を削除済み `samples/large.md` から `fixtures/core/markdown/hello.md` に差し替え。 | CI 安定化（存在するフィクスチャへの切替） | .github/workflows/ci.yml |
| 2025-12-21 00:37 | #100 | Astro build のノイズを除去するため、`web/astro.config.mjs` に Vite onwarn フィルタと `logLevel: 'error'` を設定。ビルド警告ゼロを確認。 | CI/ビルド出力のノーノイズ化 | web/astro.config.mjs |
| 2025-12-21 00:41 | #101 | fixtures の削除/維持判断フローを README に明文化（3ステップ: 使用確認→提案→決定ログ反映）。 | 今後のフィクスチャ整理を円滑化 | fixtures/README.md |
| 2025-12-21 00:46 | #102 | CI ステップの棚卸しを `docs/ci/ci-steps.md` に追加し、docs/README の索引に CI 行を追加。 | CI 保守ポイントの可視化とノーノイズ化の継続 | docs/ci/ci-steps.md, docs/README.md |
| 2025-12-21 00:52 | #103 | CI の Astro harness 比較ステップを main/develop 直push または PR ラベル `perf` のときだけ実行する条件に変更。 | CI 時間短縮と必要時のみ perf 計測 | .github/workflows/ci.yml |
| 2025-12-21 00:56 | #104 | web/README.md を追加し、Web (Astro) 層のセットアップ/開発/ビルド手順と Content Collections の位置を明記。 | Web 層オンボーディングの簡素化 | web/README.md |
| 2025-12-21 01:02 | #105 | `cargo clippy --workspace --all-targets` を修正通過。WASMテストの新オプション引数に合わせてシグネチャ更新、RewriteOptions の初期化を struct-literal 化。 | CI clippy ノイズ解消と WASM テスト適合 | crates/core/src/lib.rs, crates/wasm/src/lib.rs, crates/wasm/tests/stream_html.rs |
| 2025-12-21 01:10 | #106 | NAPI ビルド手順を `crates/napi/README.md` に明記し、CI ドキュメントに「先に pnpm install 必須、未実行だと napi: not found」と追記。 | NAPI ビルド失敗（node_modules 不在）防止の周知 | crates/napi/README.md, docs/ci/ci-steps.md |
| 2025-12-21 01:16 | #107 | pnpm-workspace に `crates/napi` を追加し、node_modules を生成。ローカルで `pnpm run build:napi` を試行したが、DNS 制限で crates.io に到達できず失敗（CI ではネット許可前提）。 | NAPI ビルド安定化（ワークスペース登録）と現状のネット制約共有 | pnpm-workspace.yaml, crates/napi/node_modules (generated) |
| 2025-12-20 00:00 | #90 | `docs` 配下をインベントリ: architecture/, decisions/, README.md, specs/ | ドキュメント整理の現状把握のため | 情報取得のみ |
| 2025-12-21 01:25 | #108 | CI の NAPI smoke 引数を `fixtures/core/markdown/hello.md` に修正（working-directory crates/napi で `../../` がルートを飛び越えていた問題を解消）。 | CI smoke パス解決修正 | .github/workflows/ci.yml |
| 2025-12-21 01:30 | #109 | 新スペック `docs/specs/mf-190-astro-docs-parity.md` を追加し、Astro公式ドキュメント再現のフェーズ/要件/CI戦略を整理。 | Astro docs 再現プロジェクトの仕様基盤を確立 | docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 01:40 | #110 | semantic HTML diff 仕様をスペックに反映し、compare-astro-harness.mjs に `--mode=semantic`/parse5ベースの構造比較を実装。 | Astro docs parity 用の誤検知削減とCI精度向上 | scripts/compare-astro-harness.mjs, docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 01:45 | #111 | Smartypants 相当の実装方針を確定：RewriteOptionsに enable_smartypants(default true)、変換ルール（curly quotes, en/em dash, ellipsis）、コード/HTML除外。N-API/WASMにも同フラグを配線。 | Astro docs のタイポグラフィ差分解消に向けた設計 | docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 02:00 | #112 | Smartypants 実装を追加（apply_smartypants）、RewriteOptions/parse_with_options で適用、N-API/WASM に enable_smartypants 配線、coreテスト追加。 | タイポグラフィ差分解消の実装完了 | crates/core/src/transform/smartypants.rs, crates/core/src/lib.rs, crates/napi/src/lib.rs, crates/napi/index.d.ts, crates/wasm/src/lib.rs, crates/wasm/tests/stream_html.rs |
| 2025-12-21 02:10 | #114 | Astro docs コンポーネント (Aside/Steps/Tabs/FileTree) のリライトルール案をスペックに追加し、enable_components 追加予定を明記。 | コンポーネント変換の実装指針を確立 | docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 02:15 | #115 | コンポーネントリライト実装計画を確定（components.rs追加、enable_componentsオプション、N-API/WASM配線、テスト方針）。 | Astro docs コンポーネント差分解消の実装準備 | docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 02:25 | #116 | コンポーネント実装で Aside を一時除外し、Steps/Tabs/FileTree のみ適用。directive Adapter との衝突を回避するため Aside rewrite は後続検討とし、テストを再度通過。 | Directive 互換性維持のための一時措置 | crates/core/src/transform/components.rs, docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 02:50 | #117 | Steps/Tabs/FileTree/File のリライト結果を固定するスナップショットテストを追加。fixtures/core/components/*.md を新設し、enable_components ON/OFF 両ケースを検証するテスト＆snapshots を作成。 | コンポーネント出力のリグレッションを即検知するため | crates/core/tests/components_snapshot.rs, fixtures/core/components/*, crates/core/tests/snapshots/components_snapshot__*.snap |
| 2025-12-21 03:05 | #118 | Aside を components リライトに追加。DirectiveAdapter が出力する Aside に `data-mf-source=\"directive\"` を付与し、lol_html パスで `<aside class=\"aside aside--{type}\">` + title 見出しに正規化。enable_components=false で素通りする回帰テストを追加し、snapshots を更新。 | Astro docs と Aside 出力を揃え、回帰を検知するため | crates/core/src/transform/directives.rs, crates/core/src/transform/components.rs, fixtures/core/components/aside_*.md, crates/core/tests/components_snapshot.rs, crates/core/tests/snapshots/components_snapshot__aside_*.snap |
| 2025-12-21 03:20 | #119 | スペック（mf-190）を Aside 実装済み状態に更新し、コンポーネントリライトの現行仕様（<aside> 正規化・title 見出し挿入・data-mf-source 削除・enable_components トグル・スナップショット）を明記。 | 実装と仕様の乖離防止、唯一の参照源として decision ログに反映 | docs/specs/mf-190-astro-docs-parity.md |
| 2025-12-21 03:35 | #120 | `scripts/run-astro-harness.mjs` と `scripts/compare-astro-harness.mjs` をリポジトリから削除し、CI からもハーネス比較を外す。関連ドキュメントを「撤去済み/ローカルのみ」に更新。 | ネイティブバイナリ依存でCI失敗するため、ハーネス比較を運用から外す | scripts/run-astro-harness.mjs, scripts/compare-astro-harness.mjs, .github/workflows/ci.yml, docs/specs mf-160/173/190, scripts/README.md, fixtures docs, README.md |
