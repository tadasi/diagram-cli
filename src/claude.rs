use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn resolve_claude_cli() -> PathBuf {
    if let Ok(p) = env::var("CLAUDE_CLI") {
        let pb = PathBuf::from(p.trim());
        if pb.exists() {
            return pb;
        }
    }
    PathBuf::from("claude")
}

fn build_agent_prompt(curl_line: &str, diagram_type: &str) -> String {
    let (directive, type_desc) = match diagram_type {
        "sequence" => (
            "sequenceDiagram",
            "クライアント・コントローラ・モデル・DB 間の処理の流れをシーケンス図で表現する。\
             参加者（participant）には役割名を付け、`rect` でまとまりを囲んで日本語の注釈を付ける。",
        ),
        "activity" => (
            "flowchart TD",
            "処理のアクティビティをフローチャートで表現する。\
             開始は `([開始])` 、終了は `([終了])` の丸角ノードにし、\
             分岐には菱形 `{条件}` を使う。",
        ),
        "component" => (
            "graph TD",
            "システムのコンポーネント構成と依存関係をコンポーネント図で表現する。\
             コンポーネントは `[コンポーネント名]` で表し、依存を矢印で結ぶ。",
        ),
        "state" => (
            "stateDiagram-v2",
            "リソースの状態遷移を状態遷移図で表現する。\
             `[*]` を開始・終了に使い、各状態間のイベント／条件をラベルに書く。",
        ),
        _ => (
            "flowchart TD",
            "処理の流れをフローチャートで表現する。",
        ),
    };

    let subgraph_rule = if diagram_type == "sequence" {
        "処理のまとまりごとに `rect rgb(240,248,255)` で囲み、\
         直前に `Note over ...: まとまりの説明` を入れる。"
    } else if diagram_type == "state" {
        "関連する状態を `state \"説明\" as グループ名` でまとめる。"
    } else {
        "処理のまとまりごとに `subgraph` で囲み、簡潔な日本語で名前を付ける\
         （例: `subgraph 認証チェック`）。各 subgraph の直後に `%% ...` で一行の補足説明を入れる。"
    };

    format!(
        r#"あなたはこのワークスペース内の Rails アプリを読む AI Agent です。

次の HTTP リクエスト（ユーザーが入力した curl 相当の文字列全体）を解釈してください。

1. `config/routes.rb` から該当するルートと `Controller#action` を特定する（GET/POST 等はリクエストから推測）。
2. 該当コントローラと、そこから呼ばれる主要なモデル/スコープ/関連をコードに基づいて要約する。
3. {type_desc}

出力ルール（厳守）:
- 応答は **```mermaid で始まるフェンス付きコードブロック 1 つだけ**。その前後に説明文・見出し・箇条書きを書かない。
- 図は **`{directive}`** で始める。
- Mermaid は v11 でパース可能な記法にする。ノードラベルに `()` `:` `#` など記号が多い場合は `["..."]` 形式のラベルを使う。
- ルートが特定できない場合は「ルート不明」として分岐を書く。
- {subgraph_rule}
- 図中にトークン・Cookie・セッション ID・API キー・パスワード等の秘匿情報を一切含めない。ヘッダー値やパラメータ値を表示する必要がある場合は `****` に置き換える。

ユーザー入力（curl 全体）:
{curl_line}
"#
    )
}

pub fn run_claude_agent(
    workspace: &Path,
    curl_line: &str,
    diagram_type: &str,
) -> Result<String, String> {
    let claude = resolve_claude_cli();
    let prompt = build_agent_prompt(curl_line, diagram_type);

    let model = env::var("DG_CLAUDE_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "claude-sonnet-4-6".to_string());

    let output = Command::new(&claude)
        .args([
            "-p",
            "--dangerously-skip-permissions",
            "--model",
            &model,
            &prompt,
        ])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to spawn Claude CLI ({claude:?}): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "claude exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
