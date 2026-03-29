# diagram-cli

AI 連携（Claude Code）で Rails コードを分析し、Mermaid ベースのシステム図を生成する CLI ツール。

## インストール

### 方法 1: インストールスクリプト（推奨）

Rust 不要。ビルド済みバイナリをダウンロードして配置します。

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

```bash
dg init   # 対話形式で対象ディレクトリ・図の種類・出力先を設定
```

## 使い方

```bash
dg                          # 対話形式で入力（curl / 自由テキスト）
dg init                     # 初期設定（対象ディレクトリ・図の種類・出力先）
dg [curl args...] <url>     # API 単位のシステム図を生成
dg <自由テキスト>           # 画面操作手順等から包括的なシステム図を生成
```

### 例

```bash
# curl 形式で API のシステム図を生成
dg curl --location 'http://localhost:3000/tech_book_terms'

# URL だけでも可
dg 'http://localhost:3000/tech_reference_terms'

# ベース URL を環境変数で設定するとパスだけで指定可能
export DG_BASE_URL='http://localhost:3000'
dg /tech_articles

# 自由テキストで包括的なシステム図を生成
dg "ユーザーがログインしてから記事を投稿するまでのフロー"
```

## 環境変数

| 変数 | 説明 |
|---|---|
| `DG_BASE_URL` | パスだけ渡すときのオリジン（例: `http://localhost:3000`） |
| `DG_CLAUDE_MODEL` | Claude CLI の `--model`（既定: `claude-sonnet-4-6`） |
| `CLAUDE_CLI` | `claude` 実行ファイルのパス（既定: PATH から解決） |

## リリース手順（メンテナ向け）

```bash
git tag v0.1.0
git push origin v0.1.0
```

タグを push すると GitHub Actions が macOS (Intel/Apple Silicon) と Linux 向けにビルドし、GitHub Release を自動作成します。
