# Markflowを用いたAstro/MDX統合のための高パフォーマンスNAPIアーキテクチャ仕様書

## 1. 序論：次世代Webフレームワークにおけるビルドパイプラインの刷新
現代のWeb開発エコシステムにおいて、Astroは「コンテンツ駆動型」のWebサイト構築において支配的な地位を確立しつつある。ゼロJavaScriptの原則と、Markdown/MDXを中心とした高度なコンテンツ管理機能がその核だ。しかし、数千〜数万のMDXファイルを処理するようになると、Unified.js/Remark/RehypeといったJavaScriptベースのビルドツールチェーンに固有のパフォーマンスボトルネックが顕在化する。本仕様書は、Rust製MarkdownプロセッサであるMarkflowをAstroのMDX処理エンジンとして統合するための技術要件、アーキテクチャ設計、および実装詳細を定義する。

既存の `https://github.com/knj-labo/markflow` コードベースと、AstroがMDXファイルに要求する厳格なインターフェース規約（Frontmatter処理、Viteプラグインの振る舞い等）を踏まえ、`@astrojs/mdx` を代替もしくは上回るRustバックエンドを構築することが目標である。

### 1.1 背景と課題：JavaScriptツールチェーンの限界
Astroの現行MDX処理はViteプラグインを介してUnifiedエコシステム上で実行される。柔軟だが、以下の構造的課題を抱える。

- **AST変換のオーバーヘッド**：MDAST→HAST→ESTreeと多段ASTを横断するため、大量のトラバーサル/メモリアロケーションが発生する。
- **シリアライズ/デシリアライズ**：プラグイン間データ受け渡しやキャッシュで頻繁にJSONシリアライズが走る。
- **GC負荷**：大量の短命ASTノードがV8 GCに負荷を与え、大規模ビルド時のレイテンシを悪化させる。

Rust実装のMarkflowは所有権モデルとゼロコピーに近いパース処理によりこれらを根本から緩和できるが、Astro互換のモジュール形式を完全再現しなければ恩恵は限定的になる。

### 1.2 本仕様の範囲と目的
以下の要素を詳細化する。

1. **NAPIバインディング設計**：Rust構造体とTypeScript型定義の整合を保ちつつFFIオーバーヘッドを最小化するインターフェース。
2. **Frontmatter処理**：Astro特有のlayoutプロパティなどをRust側で処理しJSへ橋渡しする仕組み。
3. **Viteプラグイン出力形式**：Astroのビルドが解釈可能なモジュール構造、ソースマップ生成、HMR向け依存関係管理。

## 2. Astro/MDXパイプラインのアーキテクチャ分析
AstroのMDX統合はテキスト変換に留まらず、コンテンツをコンポーネントグラフへコンパイルする役割を担う。Markflowは `@astrojs/mdx` の振る舞いを再現する必要がある。

### 2.1 コンパイルプロセス現状
`.mdx` 取り込み時の流れ：

1. **Resolution**：Viteがパスを解決しプラグインチェーン開始。
2. **Load**：ディスクから生コンテンツ読込。
3. **Transform（Unified pipeline）**
   - パース：テキスト→MDAST。
   - Frontmatter抽出：`remark-frontmatter`/`gray-matter` 等で抽出。
   - 変換：GFMやSmartypants等を適用。
   - コンパイル：ASTをJSX含むESTree→JavaScript文字列化。

Astroはコンポーネント生成に加え、メタデータエクスポートやlayoutラッピングなど高度なコード生成を行う。

### 2.2 出力モジュールの必須エクスポート
Astroが正しく扱うために必要なエクスポートとMarkflow側対応は以下の通り。

| エクスポート | 型 | 役割 | Rust側対応 |
| --- | --- | --- | --- |
| `default` | 関数/コンポーネント | MDX本文を描画し `props.components` を受け取る | JSXコード生成時にprops伝播とlayoutラップを内包 |
| `frontmatter` | オブジェクト | Content Collectionsや`Astro.glob()`用メタデータ | `serde_json` 等でシリアライズし `export const frontmatter = {...}` |
| `file` | string | ファイル絶対パス | JS側から渡されたパスをリテラル挿入 |
| `url` | string | Astroルーティング由来のURL | JS側で計算するか、なくてもよい場合は空を許容 |
| `getHeadings` | function | 見出し一覧（depth/slug/text） | パース中に収集し静的配列として返す関数生成 |

`getHeadings` は目次や検索インデックスで使用されるため、Github Slugger互換アルゴリズムで生成したスラッグを返す必要がある。

## 3. NAPIバインディング設計：`markflow-napi`
NAPI-RSはV8とRust間の高速相互運用を提供する。設計原則は「境界越え呼び出しを最小化する」こと。

### 3.1 コンパイラ設定構造体
MDXコンパイルはプロジェクト設定（GFM有効化、シンタックスハイライト等）に依存するため、永続ステートを持つ `MarkflowCompiler` を用意する。

```rust
#[napi]
pub struct MarkflowCompiler {
    config: CompilerConfig,
}

#[napi(object)]
pub struct CompilerConfig {
    pub gfm: bool,
    pub smartypants: bool,
    pub syntax_highlighting: bool,
    pub jsx_import_source: Option<String>,
}
```

`configResolved` フックで高コスト初期化を一度だけ実施し、変換時はインスタンス再利用する。

### 3.2 `compile_mdx` シグネチャ

```rust
#[napi]
impl MarkflowCompiler {
    #[napi]
    pub fn compile_mdx(
        &self,
        source: String,
        filepath: String,
        options: Option<FileOptions>,
    ) -> Result<CompileResult> {
        // implementation
    }
}
```

### 3.3 `CompileResult` 設計

```rust
#[napi(object)]
pub struct CompileResult {
    pub code: String,
    pub map: Option<String>,
    pub frontmatter_json: String,
    pub headings: Vec<Heading>,
    pub imports: Vec<ImportedModule>,
}

#[napi(object)]
pub struct Heading {
    pub depth: u8,
    pub slug: String,
    pub text: String,
}

#[napi(object)]
pub struct ImportedModule {
    pub path: String,
    pub kind: String,
}
```

Frontmatterを文字列化して返し、JS側で `JSON.parse()` することでFFIコールを最小化する。

## 4. Frontmatter処理戦略とLayout実装
AstroのFrontmatterは `layout` プロパティによりデフォルトエクスポートをラップする。MarkflowはRustレベルでこの挙動を再現する必要がある。

### 4.1 `gray_matter` クレート統合
Rustの `gray_matter` クレートで以下を実施：

1. `---` で囲まれたブロック抽出。
2. YAML/TOML/JSONとしてパースし `serde_json::Value` 化。
3. Frontmatterを除去した本文をMarkdownとして処理。

日付フィールドは `new Date("...")` リテラルとしてコード生成するか、JS側で再変換する。

### 4.2 Layoutプロパティ処理
Frontmatterに `layout` がある場合、Astroはデフォルトエクスポートを差し替える。Markflowはコード生成段階で以下のような出力を行う。

```javascript
import Layout from '../layouts/MyLayout.astro'

function MDXContent(props) {
  return /* JSX body */
}

export default function WrappedContent(props) {
  return (
    <Layout {...props} frontmatter={frontmatter}>
      <MDXContent {...props} />
    </Layout>
  )
}
```

### 4.3 スラッグ生成
`getHeadings` で返す `slug` はGithub Slugger互換ルールに従う。重複スラッグには `-1`, `-2` を付与するなど、ファイル単位で状態を持った生成器が必要。

## 5. Viteプラグイン実装
RustコアとAstroビルドをつなぐ接着剤として `vite-plugin-markflow` を提供する。

### 5.1 基本構造

```ts
import { createMarkflowCompiler } from 'markflow-napi'

export function markflowPlugin(options = {}) {
  let compiler

  return {
    name: 'vite-plugin-markflow',
    enforce: 'pre',
    configResolved(config) {
      compiler = createMarkflowCompiler({
        root: config.root,
        ...options,
      })
    },
    async transform(code, id) {
      if (!id.endsWith('.mdx') && !id.endsWith('.md')) return null
      const result = await compiler.compileMdx(code, id, { url: toAstroUrl(id) })
      result.imports.forEach((dep) => this.addWatchFile(dep.path))
      return { code: result.code, map: result.map }
    },
  }
}
```

### 5.2 生成コード仕様
`code` は `@astrojs/mdx` 出力テンプレに従う。

```javascript
/* @jsxRuntime automatic */
/* @jsxImportSource react */
import { Fragment as _Fragment, jsx as _jsx, jsxs as _jsxs } from 'react/jsx-runtime'

export const frontmatter = { /* ... */ }
export const file = '/absolute/path/to/file.mdx'
export const url = '/docs/example'

export function getHeadings() {
  return [
    { depth: 1, slug: 'intro', text: 'Intro' },
  ]
}

function _createMdxContent(props) {
  const _components = Object.assign({ h1: 'h1' }, props.components)
  return _jsxs(_Fragment, { children: [ /* ... */ ] })
}

import Layout from '../layouts/BaseLayout.astro'

export default function MDXContent(props = {}) {
  return _jsx(Layout, {
    ...props,
    frontmatter,
    children: _jsx(_createMdxContent, { ...props }),
  })
}
```

`props.components.wrapper` が提供された場合の優先順位や多重ラップにも対応できるよう `_createMdxContent` と `MDXContent` を分離する。

### 5.3 ソースマップ
`map: null` はHMR/デバッグ体験を著しく損なうため、Markdown行とJSX行の対応を追跡するSource Map v3を生成する。`swc` 等のRust製トランスパイラのマッピングロジックを転用することを推奨する。

## 6. エコシステム互換性と制約
Astroユーザーはremark/rehypeプラグインに依存する場合が多く、Rust導入時の制約を明示する必要がある。

### 6.1 JavaScriptプラグインの壁
RustでコンパイルしながらJavaScriptプラグインを実行するとFFIオーバーヘッドが大きすぎるため現実的でない。

### 6.2 解決策と妥協
- **主要機能の組み込み**：GFM、Smartypants、脚注、ID自動生成など頻出機能をRust側で実装しフラグ制御する。
- **WASMプラグイン**：将来的にWASMプラグインを受け入れる設計を検討し、言語非依存の拡張性を確保する。
- **制約の明示**：初期リリースでは「remarkプラグインは不可だが高速」を明確に伝え、パフォーマンス重視ユーザーをターゲットにする。

## 7. 結論と推奨ロードマップ
AstroへのMarkflow統合は、Astroが期待するモジュールインターフェースを完全再現できるかにかかっている。推奨実装ステップ：

1. **Core Parser（Rust）**：MDX構文対応パーサーを実装。
2. **Metadata Extraction（Rust）**：`gray_matter` でFrontmatter抽出と `layout` 検出を行う。
3. **Code Generator（Rust）**：JSX文字列生成、`export default` ラッピング、`getHeadings` 静的配列生成を実装。
4. **NAPI Layer**：`CompilerConfig` と `CompileResult` を定義しエラーハンドリングを整備。
5. **Vite Plugin（TypeScript）**：`vite-plugin-markflow` を提供し、Astro設定で簡単に導入できるようにする。

特に `layout` ハンドリングと `getHeadings` の正確な実装が互換性確保の鍵となる。これらを満たすことで、既存プロジェクトのDXを維持しつつビルド時間を大幅に短縮できる。

## 8. コンパイラ実装の詳細
`MarkflowCompiler` は `compile()` を通して以下の処理を一度に行う。

- **Frontmatter抽出**：`extract_frontmatter_block()` がYAML/TOML区間を検出し、Rust側で `serde_json::Value` に変換。エラー時は `Status::InvalidArg` を返す。
- **Markdown→HTMLレンダリング**：`HeadingTrackingStream` が `markflow_core::MarkdownParser` から流れるイベントを盗聴し、`StreamingRewriter` へストリーミングしながら見出しテキストを収集する。Rust側でHTMLを生成するため、JSへ戻るデータは純粋な文字列のみ。
- **見出しメタデータ**：`HeadingCollector` が Github Slugger 互換の `slugify` を用いて `[depth, slug, text]` を生成し、`getHeadings()` エクスポートに焼き付ける。
- **モジュール生成**：`generate_module_code()` が
  - `import { createComponent, markHTMLString } from 'astro/runtime/server/index.js'`
  - （必要に応じて）`renderComponentToString` と layout import
  - `frontmatter` / `file` / `url` / `getHeadings` エクスポート
  - `_MarkflowContent`（HTMLを返すAstroコンポーネント）と、layout指定時の `_MarkflowPage`
  を持つ文字列を構築する。生成結果は `code` フィールドとしてNode側に渡る。
- **依存関係トラッキング**：Frontmatter内に `layout` があれば、`ImportedModule { kind: "layout", path }` として絶対パスを記録し、Viteのwatch graphへ転送できるようにする。

`CompileResult` には `map`（当面 `null`）、`frontmatterJson`、`headings`、`imports` が含まれ、NAPIクライアントはこれをそのままViteへ返せる。

## 9. Viteプラグイン構成
`packages/vite-plugin-markflow` はEsmベースのプラグインを提供する。

- `enforce: 'pre'` で `.md` / `.mdx` を最優先でフック。
- `configResolved` で NAPI バインディングをロード。通常は `import('markflow-napi')` を使用し、Mono-repo内では `../../../crates/napi/index.js` へのフォールバックを持つ。
- `transform()` 内で `compiler.compile(code, id, { file, url })` を呼び出し、`imports` に含まれるファイルを `this.addWatchFile()` へ登録。結果の `code` / `map` をそのままViteへ返してAstroの後段へ流す。
- URL推定: `src/pages` 以下のファイルを `/${relative}` 形式に正規化し、`index.mdx` は `/` または親ディレクトリのルートへマップする。
- 公開APIは `markflowPlugin({ compiler, include })` としてエクスポートされ、Astro以外のVite環境からも再利用可能。

## 10. Astroハーネス統合
`fixtures/integration/astro-harness` では新しい配線を使ってE2Eを確認できる。

- `astro.config.mjs` の `vite.plugins` に `markflowPlugin()` を先頭で差し込み、既存の `virtual:markflow-docs` ハーネスはそのまま並列で動かす。
- `src/layouts/DocsLayout.astro` と `src/content/docs/getting-started.mdx` を追加し、Frontmatter経由で layout を指定。Viteプラグインが `.mdx` をハイジャックして layout import を解決しつつ `getHeadings()` / `frontmatter` をエクスポートする。
- `src/pages/index.astro` に MDX プレビューセクションを追加し、ビルド済みHTMLを `<GettingStartedContent />` として描画することでパイプラインの結果を直接確認できる。

この構成により、「RustでFrontmatter抽出→HTML生成→Astro互換モジュール出力→Viteプラグイン→Astro harness表示」という一連の流れがローカルで再現できる。
