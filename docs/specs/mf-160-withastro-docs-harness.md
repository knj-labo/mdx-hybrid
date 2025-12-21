# MF-160: withastro/docs Starlight ハーネス統合仕様

> Note: Harness helper scripts (`run-astro-harness.mjs`, `compare-astro-harness.mjs`) はビルド時間計測のみの最小形で復活。withastro/docs 向けの専用スクリプトは未提供なので、必要なら派生版を作成する。

## 1. 目的
Starlight ベースの公式ドキュメント（`withastro/docs`）を Markflow のエンドツーエンド検証対象として取り込み、Starlight 固有の MDX フロー（auto-import される layout、Content Collections、`entry.render()`）と `vite-plugin-markflow` の挙動が衝突しないことを示す。これにより Phase 2 の「Vite Plugin Interception」タスクを Starlight でも完了扱いできるようにする。

## 2. 成果物
- `fixtures/integration/withastro-docs/` 配下に `withastro/docs` を再現するハーネス（git subtree もしくは shallow clone 用スクリプトと README を含む）。
- `astro.config.mjs` に `vite-plugin-markflow` を `enforce: 'pre'` で追加したパッチ。
- Starlight の `starlight.config.mjs` / `src/content/config.ts` と整合する MDX レイアウト検証用ドキュメント（例: `src/content/docs/markflow-integration.mdx`）。
- 既存スクリプトは Astro harness 用（build-only）で、withastro/docs 用は未提供。必要なら派生スクリプトを作成すること。

## 3. 要件
### 3.1 機能要件
1. `withastro/docs` 上で `.md`/`.mdx` のビルドが Markflow プラグイン経由で成功すること。
2. Starlight が提供する `frontmatter.layout` / `entry.render()` / `headings` 参照が壊れないこと。
3. `pnpm dev` と `pnpm build` が Starlight テンプレートで通ること。
4. 速度比較は必要に応じて自前スクリプトで実施する（汎用 compare スクリプトは build 時間のみ記録、差分なし）。

### 3.2 非機能要件
- ハーネス用 `node_modules` / `.astro` / `.vercel` 等の重い成果物は `.gitignore` 済み。
- Backlog は廃止済みのため、更新履歴は decision log (`docs/decisions/0001-lean-architecture.md`) に統一する。
- CI モードでは shallow clone（もしくは zip 展開）で `withastro/docs` を取得し、Markflow ルートから再現可能にする。

## 4. 実装方針
1. **取り込み戦略**  
   - サイズを抑えるため、`git subtree` で特定コミットを `fixtures/integration/withastro-docs/` に追加するか、`scripts/setup-withastro-docs.sh` でリリース zip を展開する方式を採用する。いずれも `README` にコミット SHA と更新手順を明記する。
2. **Vite プラグイン注入**  
   - `withastro/docs/astro.config.mjs` の `vite.plugins` 先頭に `markflowPlugin()` を追加。Starlight の `@astrojs/starlight/config` 連鎖と干渉しないよう、`enforce: 'pre'` を維持する。
3. **モード切替**  
   - `MARKFLOW_HARNESS_MODE` 環境変数（`baseline`/`markflow`）など任意のフラグでプラグイン ON/OFF を切り替える。既存の compare/run スクリプトは Astro harness 用なので、withastro/docs 用に流用する場合はモード解釈を拡張する。
4. **Starlight 互換検証**  
   - `src/content/docs/` にサンプル MDX（Frontmatter layout / コードフェンス / 見出しが複数あるもの）を追加し、`npm run dev` でブラウザへアクセスした際に `frontmatter`, `getHeadings`, `entry.render()` の値が期待通りであることをログ出力＋スクリーンショットで確認。
5. **成果ファイル**  
   - `fixtures/integration/withastro-docs/harness-summary.json` は参考用。必要に応じて手元で更新し、比較結果（平均ビルド時間 / 速度倍率）を記録する。CI からの自動呼び出しは行わない。

## 5. テスト計画
1. `pnpm --filter withastro-docs install`（もしくは `pnpm install`）→ `pnpm dev` で Starlight UI が起動するか確認。
2. ハーネス検証を行う場合は、手元で用意したスクリプトまたは手動操作で baseline/markflow を切り替えてビルドし、差分を確認する（汎用 compare スクリプトを流用するならターゲット切替を追加）。
3. 差分結果やビルド時間を `fixtures/integration/withastro-docs/harness-summary.json` に任意で記録する（CI での自動検証は行わない）。
4. `cargo clippy --workspace --all-targets -- -D warnings` を通す。

## 6. リスクとフォローアップ
- **リポサイズ増大**：`withastro/docs` の取り込みでリポが大きくなる可能性があるため、subtree ではなくダウンロードスクリプト方式に切り替える判断基準を README に明記する。
- **Starlight 更新追従**：Starlight 側の breaking change に追従するため、月次で `withastro/docs` の upstream を pull するタスクを Backlog に追加する。
- **CI 実行時間**：ハーネス比較はCIから外しているため、再導入する場合は所要時間を事前に計測し、`--summary-only` などの短縮策を併用する。
