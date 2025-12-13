# MF-160: withastro/docs Starlight ハーネス統合仕様

## 1. 目的
Starlight ベースの公式ドキュメント（`withastro/docs`）を Markflow のエンドツーエンド検証対象として取り込み、Starlight 固有の MDX フロー（auto-import される layout、Content Collections、`entry.render()`）と `vite-plugin-markflow` の挙動が衝突しないことを示す。これにより Phase 2 の「Vite Plugin Interception」タスクを Starlight でも完了扱いできるようにする。

## 2. 成果物
- `fixtures/integration/withastro-docs/` 配下に `withastro/docs` を再現するハーネス（git subtree もしくは shallow clone 用スクリプトと README を含む）。
- `astro.config.mjs` に `vite-plugin-markflow` を `enforce: 'pre'` で追加したパッチ。
- Starlight の `starlight.config.mjs` / `src/content/config.ts` と整合する MDX レイアウト検証用ドキュメント（例: `src/content/docs/markflow-integration.mdx`）。
- `scripts/run-astro-harness.mjs` 互換の実行手順、ならびに `scripts/compare-astro-harness.mjs --runs=2 --summary fixtures/integration/withastro-docs/harness-summary.json` の成果ファイル。

## 3. 要件
### 3.1 機能要件
1. `withastro/docs` 上で `.md`/`.mdx` のビルドが Markflow プラグイン経由で成功すること。
2. Starlight が提供する `frontmatter.layout` / `entry.render()` / `headings` 参照が壊れないこと。
3. `pnpm dev` と `pnpm build` が Starlight テンプレートで通ること。
4. 速度比較を `baseline`（標準 `@astrojs/mdx`）と `markflow` の 2 モードで最低 2 回ずつ実行し、`harness-summary.json` を生成すること。

### 3.2 非機能要件
- ハーネス用 `node_modules` / `.astro` / `.vercel` 等の重い成果物は `.gitignore` 済み。
- `scripts/check-backlog.mjs` で spec 見出し（`# MF-160 ...`）と Backlog の整合が取れていること。
- CI モードでは shallow clone（もしくは zip 展開）で `withastro/docs` を取得し、Markflow ルートから再現可能にする。

## 4. 実装方針
1. **取り込み戦略**  
   - サイズを抑えるため、`git subtree` で特定コミットを `fixtures/integration/withastro-docs/` に追加するか、`scripts/setup-withastro-docs.sh` でリリース zip を展開する方式を採用する。いずれも `README` にコミット SHA と更新手順を明記する。
2. **Vite プラグイン注入**  
   - `withastro/docs/astro.config.mjs` の `vite.plugins` 先頭に `markflowPlugin()` を追加。Starlight の `@astrojs/starlight/config` 連鎖と干渉しないよう、`enforce: 'pre'` を維持する。
3. **モード切替**  
   - `MARKFLOW_HARNESS_MODE` 環境変数（`baseline`/`markflow`）を使い、`@astrojs/mdx` を無効化/有効化するか、`vite-plugin-markflow` を条件付きで登録する。比較スクリプトからは既存の `scripts/run-astro-harness.mjs` を拡張し、`withastro-docs` ターゲットを呼び分ける。
4. **Starlight 互換検証**  
   - `src/content/docs/` にサンプル MDX（Frontmatter layout / コードフェンス / 見出しが複数あるもの）を追加し、`npm run dev` でブラウザへアクセスした際に `frontmatter`, `getHeadings`, `entry.render()` の値が期待通りであることをログ出力＋スクリーンショットで確認。
5. **成果ファイル**  
   - `fixtures/integration/withastro-docs/harness-summary.json` を Git 管理下に置き、比較結果（平均ビルド時間 / 速度倍率）を格納。CI が `node scripts/compare-astro-harness.mjs --target=withastro-docs --runs=2 --summary ...` を呼び出し、変更があれば差分を検出する。

## 5. テスト計画
1. `pnpm --filter withastro-docs install`（もしくは `pnpm install`）→ `pnpm dev` で Starlight UI が起動するか確認。
2. `node scripts/run-astro-harness.mjs withastro-docs markflow` と `baseline` をそれぞれ 1 回以上実行し、ビルド完了を確認。
3. `node scripts/compare-astro-harness.mjs --target=withastro-docs --runs=2 --summary fixtures/integration/withastro-docs/harness-summary.json` を実行し、生成された JSON をコミット。
4. `node scripts/check-backlog.mjs` と `cargo clippy --workspace --all-targets -- -D warnings` を通す。

## 6. リスクとフォローアップ
- **リポサイズ増大**：`withastro/docs` の取り込みでリポが大きくなる可能性があるため、subtree ではなくダウンロードスクリプト方式に切り替える判断基準を README に明記する。
- **Starlight 更新追従**：Starlight 側の breaking change に追従するため、月次で `withastro/docs` の upstream を pull するタスクを Backlog に追加する。
- **CI 実行時間**：withastro/docs の build が長い場合、比較 runs=2 でも 60 秒以内に収まるよう `--summary-only` オプションなどを検討する。
