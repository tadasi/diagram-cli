# diagram-cli
AI 連携でシステム図を生成する CLI

## `dg curl`（Cursor Agent）

受け取った curl 相当の文字列を **Cursor Agent**（`cursor agent --print`）に渡し、Rails ルートとコントローラを読んで **Mermaid** を生成します。生成結果は `~/Desktop/dg_<パス>.mmd` と `.html` に書き、HTML をブラウザで開きます。

前提:

- **Cursor CLI** が使えること（macOS では通常 `/Applications/Cursor.app/Contents/Resources/app/bin/cursor`）。別パスなら `CURSOR_CLI` を設定。
- **認証済み**であること（`cursor agent login` 等）。API キーは `CURSOR_API_KEY` でも可（Cursor のドキュメント参照）。
- 解析対象の Rails リポジトリを **`DG_WORKSPACE`** で指定（未設定時は `~/Projects/tech-index` が存在すればそれ、なければカレントディレクトリ）。
- **`dg` は `cursor agent` に `--model auto` を付けます**（無料プランの Auto と整合）。別モデルを使う場合は `DG_CURSOR_MODEL` を設定。

```bash
export DG_WORKSPACE="$HOME/Projects/tech-index"
# 従来どおり curl 形式
dg curl --location 'http://localhost:3000/tech_book_terms'

# 先頭の curl は省略可（任意のルートの URL で可）
dg --location 'http://localhost:3000/tech_articles'
dg 'http://localhost:3000/tech_reference_terms'

# ベース URL を環境変数またはリポジトリ直下の .dg-base-url（1 行）で指定するとパスだけで可変に指定できる
export DG_BASE_URL='http://localhost:3000'
dg /tech_articles
```
