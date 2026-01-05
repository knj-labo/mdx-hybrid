use markdown::mdast::Node;

/// レンダリング中の「現在の状態」を管理する構造体
pub struct Context<'a> {
    buf: String,
    stack: Vec<Scope>,
    #[allow(dead_code)]
    options: &'a Options,
    needs_starlight_css: bool,
}

/// スタックに積む「現在のスコープ」の種類
#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Root,
    Paragraph,
    List,
    Aside(AsideMeta),
    Card(CardMeta),
}

/// Aside固有の情報
#[derive(Debug, Clone, PartialEq)]
pub struct AsideMeta {
    pub kind: String,
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
            stack: vec![Scope::Root],
            options,
            needs_starlight_css: false,
        }
    }

    /// バッファに文字列をそのまま書き込む（タグなど安全な文字列用）
    pub fn push_raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// HTMLエスケープしながらバッファに書き込む（public API）
    pub fn push_text(&mut self, s: &str) {
        self.push_escaped(s);
    }

    /// 【内部用】HTMLエスケープしながらバッファに書き込む
    /// テキストノード用（<, >, & のみエスケープ）
    fn push_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.buf.push_str("&lt;"),
                '>' => self.buf.push_str("&gt;"),
                '&' => self.buf.push_str("&amp;"),
                _ => self.buf.push(c),
            }
        }
    }

    /// 【内部用】属性値用にエスケープして書き込む
    fn push_attr_value(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '<' => self.buf.push_str("&lt;"),
                '>' => self.buf.push_str("&gt;"),
                '&' => self.buf.push_str("&amp;"),
                '"' => self.buf.push_str("&quot;"),
                _ => self.buf.push(c),
            }
        }
    }

    /// 現在のスコープを取得
    pub fn current_scope(&self) -> &Scope {
        self.stack.last().unwrap_or(&Scope::Root)
    }

    /// スコープに入る
    pub fn enter(&mut self, scope: Scope) {
        self.stack.push(scope);
    }

    /// スコープから出る
    pub fn exit(&mut self) -> Option<Scope> {
        self.stack.pop()
    }

    /// Starlight コンポーネントが使用されたことをマーク
    pub fn mark_starlight_used(&mut self) {
        self.needs_starlight_css = true;
    }

    /// 処理完了後に結果を取り出す
    pub fn finish(self) -> String {
        self.buf
    }
}

/// 設定（ユーザーがいじるので pub でOK）
pub struct Options {
    pub inject_starlight_css: bool,
    pub enable_directives: bool,
}

/// ASTノードを受け取り、状態を遷移させながらHTMLを書き込む
fn render_node(node: &Node, ctx: &mut Context) {
    match node {
        // ルートノード
        Node::Root(root) => {
            // ルートノードは何もタグを出力せず、子要素を処理するだけ
            for child in &root.children {
                render_node(child, ctx);
            }
        }

        // テキストノード
        Node::Text(text) => {
            ctx.push_text(&text.value);
        }

        // 通常のパラグラフ
        Node::Paragraph(para) => {
            ctx.push_raw("<p>");
            ctx.enter(Scope::Paragraph);

            for child in &para.children {
                render_node(child, ctx);
            }

            ctx.exit();
            ctx.push_raw("</p>");
        }

        // リンク
        Node::Link(link) => {
            ctx.push_raw(r#"<a href=""#);
            ctx.push_attr_value(&link.url);
            ctx.push_raw(r#"">"#);

            for child in &link.children {
                render_node(child, ctx);
            }

            ctx.push_raw("</a>");
        }

        // その他の要素（未実装）
        _ => {
            eprintln!("Warning: Unhandled node type: {:?}", node);
        }
    }
}

/// Markdownを受け取り、HTMLに変換する（エントリーポイント）
pub fn to_html(input: &str, options: &Options) -> Result<String, String> {
    // 1. Parse to MDAST (設定を強化)
    // ディレクティブやフロントマターを有効にする設定
    let parse_options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            // フロントマター (--- ... ---) を認識させる
            frontmatter: true,
            // GitHub Flavored Markdownの機能を有効化
            gfm_autolink_literal: true,
            gfm_strikethrough: true,
            gfm_table: true,
            gfm_task_list_item: true,
            ..markdown::Constructs::default()
        },
        ..markdown::ParseOptions::default()
    };

    let tree = markdown::to_mdast(input, &parse_options)
        .map_err(|e| format!("Markdown parse error: {}", e))?;

    // 2. Traversal
    let mut ctx = Context::new(options);
    render_node(&tree, &mut ctx);

    // 3. Check CSS flag before consuming ctx
    // ここでフラグを退避させるのが正解です
    let needs_css = ctx.needs_starlight_css;
    let mut result = ctx.finish();

    // 4. Post-process: CSS injection if needed
    if needs_css && options.inject_starlight_css {
        // 将来的に assets/starlight.css を読み込む形にする
        // 現時点ではプレースホルダーでOK
        let style_tag = "<style is:global>/* Starlight CSS will be injected here */</style>\n";
        result = format!("{}{}", style_tag, result);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 基本的なテスト (Step #15) ---
    #[test]
    fn test_simple_text() {
        let input = "Hello, world!";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let result = to_html(input, &options).unwrap();
        assert!(result.contains("Hello, world!"));
    }

    #[test]
    fn test_paragraph() {
        let input = "This is a paragraph.";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let result = to_html(input, &options).unwrap();
        assert!(result.contains("<p>"));
        assert!(result.contains("</p>"));
        assert!(result.contains("This is a paragraph."));
    }

    #[test]
    fn test_link() {
        let input = "[Rust](https://www.rust-lang.org/)";
        let options = Options {
            inject_starlight_css: false,
            enable_directives: true,
        };

        let result = to_html(input, &options).unwrap();
        assert!(result.contains(r#"<a href="https://www.rust-lang.org/""#));
        assert!(result.contains("Rust</a>"));
    }

    // --- 【重要】AST構造を確認するためのデバッグ用テスト ---
    // これを実行すると、ターミナルにASTのツリー構造が表示されます
    #[test]
    fn debug_directive_ast() {
        let input = ":::note[My Title]\nContent\n:::";

        // パース設定 (本番と同じにする)
        // 注: markdown-rs クレートはディレクティブをネイティブサポートしていない
        // このテストは生のAST構造を確認するためのもの
        let parse_options = markdown::ParseOptions::default();

        // パース実行
        let tree = markdown::to_mdast(input, &parse_options).unwrap();

        // ここでAST構造を標準出力にダンプ！
        println!("=== AST DEBUG START ===");
        println!("{:#?}", tree);
        println!("=== AST DEBUG END ===");
    }
}
