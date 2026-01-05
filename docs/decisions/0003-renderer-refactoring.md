# 0003. Mdast Renderer Refactoring: From Regex to State Machine

## Status
**Proposed** - 2026-01-05

## Context (現状と課題)

現在、`mdast_renderer.rs` は正規表現と文字列操作（`TagScanner`）を多用してHTMLの書き換えを行っている。
しかし、ネストされた構造（例: Cardの中のリスト、Asideの中の強調など）において、以下の問題が発生している。

### 主要な問題点

1. **二重パース:** `lol_html` と自前の `TagScanner` の整合性が取れず、タグの閉じ忘れや位置ズレが発生しやすい。
   - `collect_tag_replacements_multi` で一度走査 → `VecDeque` に詰める
   - `lol_html` で再度パース → キューから `pop_front`
   - **リスク**: ズレが発生すると無関係な場所に無関係なデータが注入される「ロシアンルーレット」状態

2. **状態管理の複雑さ:** `Rc<RefCell<VecDeque<...>>>` を大量に使用しており、可読性と保守性が著しく低い。
   ```rust
   let steps_queue = Rc::new(RefCell::new(VecDeque::from(self.steps)));
   let file_tree_queue = Rc::new(RefCell::new(VecDeque::from(self.file_trees)));
   let aside_queue = Rc::new(RefCell::new(VecDeque::from(self.asides)));
   // ... 各コンポーネントごとに個別のキュー
   ```

3. **文字列操作の連鎖:** ソースマップ（元の行番号）が完全に失われる。
   - `preprocess_directives` で `:::` を `<Aside>` に置換
   - Markdownパース
   - `normalize_aside_html` で再度加工
   - **結果**: エラーメッセージで元の位置を特定不可能

4. **UTF-8非対応:** バイト単位のインデックス操作が多く、マルチバイト文字（日本語、アラビア語など）でパニック。
   ```rust
   // 問題のあるコード例
   while pos < input.len() {
       if input[pos..].starts_with('<') {  // UTF-8非対応！
           let next_char = input.as_bytes()[pos + 1];  // パニックの原因
   ```

### 現在発生している具体的なバグ

**症状**: MDX内のマークダウン（リンク、リストなど）が `<pre><code>` で囲まれてしまう

```html
<!-- 期待される出力 -->
<p>Explore <a href="https://astro.build/themes/">Astro starter themes</a>...</p>

<!-- 実際の出力 -->
<pre><code>Explore [Astro starter themes](https://astro.build/themes/)...</code></pre>
```

**原因**: JSXタグ内のマークダウンが未レンダリングのまま残り、インデント（4スペース以上）がコードブロックと判定される。

**影響範囲**: withastro/docs で 28/40 ルート（70%）が失敗

## Decision (決定事項)

現在の「文字列置換アプローチ」を放棄し、**「ASTトラバーサルと状態遷移（State Machine）によるレンダリング」**へ移行する。

### 設計方針

1. **TagScanner の完全削除**: 二重パースを排除
2. **Context 構造体の導入**: プッシュダウン・オートマトンで状態管理
3. **AST直接トラバース**: `markdown-rs` の mdast を直接走査
4. **段階的移行**: 既存コードを壊さず、並行実装してから切り替え

## Future Implementation Plan (実装の詳細設計)

### 1. 新しい状態管理構造体 `Context` の導入

グローバルな状態や `Rc/RefCell` を排除し、レンダリング中のコンテキストを単一の構造体で管理する。

```rust
use std::fmt::Write; // 文字列への書き込み用

/// レンダリング中の「現在の状態」を管理する構造体
pub struct Context<'a> {
    /// 出力バッファ（ここにHTMLを書き込んでいく）
    pub buf: String,

    /// 現在処理中のノードのスタック（プッシュダウン・オートマトンの要）
    /// 例: [Root, BlockQuote, Paragraph, Strong] のように積まれる
    pub stack: Vec<NodeKind>,

    /// 設定やメタデータ（必要に応じて）
    pub options: &'a Options,

    /// このページで Starlight コンポーネントが使用されたかを追跡
    pub needs_starlight_css: bool,
}

/// スタックに積む「現在の居場所」の種類
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Root,
    Paragraph,
    List,
    Aside(AsideMeta), // Asideの中にいるときは、そのメタ情報を持つ
    Card(CardMeta),
    // ... 他の要素
}

/// Aside固有の情報
#[derive(Debug, Clone, PartialEq)]
pub struct AsideMeta {
    pub kind: String, // "note", "tip", "caution" など
    pub title: Option<String>,
}

/// Card固有の情報
#[derive(Debug, Clone, PartialEq)]
pub struct CardMeta {
    pub title: String,
    pub icon: Option<String>,
}

impl<'a> Context<'a> {
    pub fn new(options: &'a Options) -> Self {
        Self {
            buf: String::with_capacity(1024 * 16),
            stack: vec![NodeKind::Root], // 最初はRootにいる
            options,
            needs_starlight_css: false,
        }
    }

    /// バッファに文字列を書き込むヘルパー
    pub fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// 現在のスタックの一番上を見る
    pub fn current_node(&self) -> &NodeKind {
        self.stack.last().unwrap_or(&NodeKind::Root)
    }

    /// スタックにノードを積む
    pub fn enter(&mut self, kind: NodeKind) {
        self.stack.push(kind);
    }

    /// スタックからノードを降りる
    pub fn exit(&mut self) -> Option<NodeKind> {
        self.stack.pop()
    }

    /// Starlight コンポーネントが使用されたことをマーク
    pub fn mark_starlight_used(&mut self) {
        self.needs_starlight_css = true;
    }
}

pub struct Options {
    pub inject_starlight_css: bool,
    pub enable_directives: bool,
    // 必要な設定を追加
}
```

### 2. 再帰的なレンダリング関数 `render_node`

MarkdownのAST（またはイベントストリーム）を受け取り、再帰的に処理する関数を核とする。

```rust
/// ASTノードを受け取り、状態を遷移させながらHTMLを書き込む
fn render_node(node: &Node, ctx: &mut Context) {
    match node {
        // コンテナディレクティブ (::: note ...) に相当するノードが来た場合
        Node::ContainerDirective(directive) => {
            // 1. Enter: <aside>タグを開く処理
            let kind = directive.name.clone();
            let title = directive.attributes.get("title").cloned();

            ctx.mark_starlight_used(); // CSS注入が必要

            ctx.push_str(&format!(r#"<aside class="starlight-aside starlight-aside--{}">"#, kind));
            if let Some(t) = &title {
                ctx.push_str(&format!(r#"<p class="starlight-aside__title">{}</p>"#, escape_html(t)));
            }
            ctx.push_str(r#"<div class="starlight-aside__content">"#);

            // 2. 状態をスタックに積む
            ctx.enter(NodeKind::Aside(AsideMeta { kind, title }));

            // 3. 子要素を再帰的に処理（ここが重要！文字列操作では難しかった部分）
            for child in &directive.children {
                render_node(child, ctx);
            }

            // 4. Exit: 閉じタグ処理
            ctx.exit(); // スタックから降りる
            ctx.push_str("</div></aside>");
        }

        // 通常のパラグラフ
        Node::Paragraph(para) => {
            ctx.push_str("<p>");
            ctx.enter(NodeKind::Paragraph);

            for child in &para.children {
                render_node(child, ctx);
            }

            ctx.exit();
            ctx.push_str("</p>");
        }

        // テキストノード
        Node::Text(text) => {
            ctx.push_str(&escape_html(&text.value));
        }

        // リンク
        Node::Link(link) => {
            ctx.push_str(&format!(r#"<a href="{}">"#, escape_attr(&link.url)));
            for child in &link.children {
                render_node(child, ctx);
            }
            ctx.push_str("</a>");
        }

        // その他の要素...
        _ => {
            // デフォルトの処理またはログ
            eprintln!("Warning: Unhandled node type: {:?}", node);
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_html(s).replace('"', "&quot;")
}
```

### 3. エントリーポイント `to_html`

```rust
pub fn to_html(input: &str, options: &Options) -> Result<String, MarkflowError> {
    // 1. Parse to MDAST
    let mdast_options = build_mdast_options();
    let tree = markdown::to_mdast(input, &mdast_options)
        .map_err(|e| MarkflowError::MarkdownAdapter(e.to_string()))?;

    // 2. Traversal
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 3. Post-process: CSS injection if needed
    if ctx.needs_starlight_css {
        let css = include_str!("starlight_components.css");
        ctx.buf = format!("<style is:global>{}</style>\n{}", css, ctx.buf);
    }

    Ok(ctx.buf)
}
```

### 4. 段階的移行ステップ（推奨実装順序）

#### Phase 1: 基盤構築 (1-2日)
1. **新しいモジュール作成**: `crates/core/src/renderer/mdast.rs`
2. **Context 構造体の実装**: 上記のコードをそのまま実装
3. **最小限のテスト**: Text ノードだけをレンダリングするテストを書く

```rust
#[test]
fn test_render_text_only() {
    let node = Node::Text(Text { value: "Hello".to_string() });
    let mut ctx = Context::new(&Options::default());
    render_node(&node, &mut ctx);
    assert_eq!(ctx.buf, "Hello");
}
```

#### Phase 2: Asideの移行 (2-3日)
1. **Aside専用レンダラー**: `::: note` などのディレクティブを処理
2. **既存テストの複製**: `mdast_renderer.rs` の Aside テストを v2 用に複製
3. **並行動作確認**: 両方のレンダラーが同じ出力を生成することを確認

#### Phase 3: Card/CardGridの移行 (2-3日)
1. **Card レンダリング**: JSX タグ `<Card>` の処理
2. **ネストのテスト**: CardGrid > Card > Paragraph > Link の階層構造をテスト
3. **デバッグ**: スタックの状態遷移をログで確認

#### Phase 4: 全要素の移行 (1週間)
1. List, Table, Code Block などを順次移行
2. すべての insta スナップショットテストを v2 で通す
3. visual diff で withastro/docs の失敗率を 0% に

#### Phase 5: 切り替え (1日)
1. `render_to_html_mdast` の実装を v2 に差し替え
2. 古い `TagScanner` と `MdxProcessor` を削除
3. 最終的な統合テスト

### 5. テスト戦略

**ユニットテスト（必須）:**
```rust
#[test]
fn test_nested_aside_in_card() {
    let input = r#"
<Card title="Test">
::: note
This is a **note** with [link](/path).
:::
</Card>
    "#;
    let result = to_html(input, &Options::default());
    assert!(result.contains("<aside"));
    assert!(result.contains("<strong>note</strong>"));
    assert!(result.contains(r#"<a href="/path">link</a>"#));
}
```

**統合テスト（推奨）:**
- 既存の `tests/insta_tests.rs` をすべて v2 で実行
- withastro/docs の semantic diff で 0 differences を目指す

**パフォーマンステスト（オプション）:**
- 5000ページのビルド時間を計測（v1 vs v2）

## Consequences (結果と影響)

### Positive (期待される効果)
- ✅ **保守性の向上**: 状態管理が明確になり、バグ修正が容易
- ✅ **UTF-8対応**: バイト単位の操作を排除し、マルチバイト文字に完全対応
- ✅ **エラーメッセージ改善**: ソースマップを保持できるようになる
- ✅ **拡張性**: 新しいコンポーネント追加が簡単
- ✅ **CSS注入の安定性**: ページごとの Context で管理し、グローバル状態を排除

### Negative (トレードオフ)
- ❌ **初期投資の大きさ**: 2-3週間の実装期間が必要
- ❌ **一時的な重複**: v1 と v2 が並存する期間がある
- ❌ **学習コスト**: 新しいアーキテクチャの理解が必要

### Risks (リスク)
- ⚠️ **パフォーマンス劣化**: 再帰処理のオーバーヘッド（要計測）
- ⚠️ **移行の失敗**: 既存機能の破壊（段階的テストで緩和）

## References

- [Markdown AST (mdast) specification](https://github.com/syntax-tree/mdast)
- [micromark documentation](https://github.com/micromark/micromark)
- ADR-0001: Lean Architecture
- ADR-0002: Draft PR Submission

## Notes

**今回の調査で判明した重要な事実:**
1. デフォルトパイプラインは Multipass（env: `MARKFLOW_PIPELINE=mdast` で切り替え可能）
2. `render_jsx_inner_markdown` の UTF-8 バグにより、アラビア語ページでパニック
3. 正規表現アプローチは限界に達している
4. **グローバル状態（`AtomicBool`）による CSS 注入は複数ページで競合のリスク**

**次のアクション:**
- [ ] このADRを承認する
- [ ] `mdast.rs` モジュールを作成
- [ ] Context 構造体を実装
- [ ] 最初のテスト（Text ノードのみ）を書く
