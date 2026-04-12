# diagram-cli (`dg`)

Rails コードを Claude Code Agent で分析し、Mermaid ベースのシステム図を生成する CLI ツールです。

> [!CAUTION]
> **【分析対象のソースコードについて】** 本 CLI コマンドを実行すると、内部的に Claude コマンドを実行し、指定されたプロジェクトファイルのソースコードを読み取ります。秘匿情報を読み取らないようフラグ指定してはいますが、**秘匿情報の扱いには十分気を付けてください**。万が一問題が発生しても、**当方は一切責任を負いません**。

> [!CAUTION]
> **【生成されるシステム図の正確性について】** 本ツールが出力するシステム図は、実行環境の **Claude Code CLI が Rails コードを解釈した結果**に基づいて生成されます。指定されたモデルの AI Agent による推論を元にしているため、**図の内容が実際のコードの振る舞いと一致することを保証するものではありません。** 生成結果を鵜呑みにせず、設計の理解や議論の出発点としてご活用ください。

## インストール

### 方法 1: インストールスクリプト（推奨）

Rust 不要。GitHub Releases からビルド済みバイナリをダウンロードして配置します。

```bash
curl -fsSL https://raw.githubusercontent.com/tadasi/diagram-cli/main/install.sh | bash
```

インストール先を変更する場合:

```bash
DG_INSTALL_DIR="$HOME/.local/bin" curl -fsSL https://raw.githubusercontent.com/tadasi/diagram-cli/main/install.sh | bash
```

### 方法 2: cargo install（Rust ツールチェインがある場合）

```bash
cargo install --git https://github.com/tadasi/diagram-cli.git
```

### 前提条件

- **Claude Code CLI** (`claude`) がインストール済みで PATH に通っていること
- 解析対象の Rails リポジトリがローカルに存在すること

## セットアップ

初回実行時に自動でセットアップが始まります。手動で設定するには:

```bash
dg init
```

対話形式で以下の 3 項目を設定します。設定は `~/.config/dg/config.json` に保存されます。

| 設定項目 | 説明 |
|---|---|
| 対象ディレクトリ | 解析する Rails リポジトリのパス（`~` からの相対パス） |
| 図の種類 | フローチャート / シーケンス図 |
| 出力先 | 生成ファイルの出力先ディレクトリ（`~` からの相対パス、既定: `Desktop`） |

## 使い方

```bash
dg                          # 対話形式で入力（curl / 自由テキスト）
dg init                     # 設定を変更
dg [curl args...] <url>     # curl 形式で API 単位のシステム図を生成
dg <自由テキスト>           # 自由テキストで包括的なシステム図を生成
```

実行すると設定の確認画面が表示され、そのまま進むか設定を変更するか選択できます。
分析完了後、`.mmd`（Mermaid ソース）と `.html`（ブラウザ表示用）が出力先に生成され、HTML がブラウザで自動的に開きます。

### 例: curl 形式（API 単位の分析）

```bash
# フル URL を指定
dg curl --location 'http://localhost:3000/users'

# URL だけでも可
dg 'http://localhost:3000/articles'

# DG_BASE_URL を設定するとパスだけで指定可能
export DG_BASE_URL='http://localhost:3000'
dg /comments
```

リポジトリ直下に `.dg-base-url` ファイル（1行でオリジンを記載）を置いても同様に機能します。

### 例: 自由テキスト（包括的な分析）

```bash
dg "ユーザーがログインしてから記事を投稿するまでのフロー"
```

引数なしで `dg` を実行すると、対話形式で複数行の入力が可能です。

## 環境変数

| 変数 | 説明 | 既定値 |
|---|---|---|
| `DG_BASE_URL` | パスだけ渡すときのオリジン | なし |
| `DG_CLAUDE_MODEL` | Claude CLI に渡す `--model` | `claude-sonnet-4-6` |
| `DG_MAX_TURNS` | Claude CLI の `--max-turns` | `20` |
| `CLAUDE_CLI` | `claude` 実行ファイルのパス | PATH から解決 |

## メンテナ向け

### 開発環境

#### Rust インストール

参照

- https://rust-lang.org/tools/install/
- https://github.com/rust-lang/rustup?tab=readme-ov-file#rustup-the-rust-toolchain-installer

### ビルド（コンパイル）のみ

```
cargo build
```

### コンパイル & コマンド実行

```
cargo run <自由テキスト>
```

### リリース手順

```bash
git tag v0.1.0
git push origin v0.1.0
```

タグを push すると GitHub Actions が macOS (Intel / Apple Silicon) と Linux 向けにビルドし、GitHub Release を自動作成。

## ライセンス

[MIT](LICENSE)
