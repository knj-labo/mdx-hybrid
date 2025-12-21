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
