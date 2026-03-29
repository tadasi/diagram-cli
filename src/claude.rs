use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

fn resolve_claude_cli() -> PathBuf {
    env::var("CLAUDE_CLI")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("claude"))
}

fn diagram_type_info(diagram_type: &str) -> (&'static str, &'static str, &'static str) {
    let (directive, type_desc) = match diagram_type {
        "sequence" => (
            "sequenceDiagram",
            "クライアント・コントローラ・モデル・DB 間の処理の流れをシーケンス図で表現する。\
             参加者（participant）には役割名を付け、`rect` でまとまりを囲んで日本語の注釈を付ける。",
        ),
        _ => (
            "flowchart TD",
            "処理の流れをフローチャートで表現する。",
        ),
    };

    let subgraph_rule = match diagram_type {
        "sequence" => {
            "処理のまとまりごとに `rect rgb(240,248,255)` で囲み、\
             直前に `Note over ...: まとまりの説明` を入れる。"
        }
        _ => {
            "処理のまとまりごとに `subgraph` で囲み、簡潔な日本語で名前を付ける\
             （例: `subgraph 認証チェック`）。各 subgraph の直後に `%% ...` で一行の補足説明を入れる。"
        }
    };

    (directive, type_desc, subgraph_rule)
}

fn build_prompt(input: &str, diagram_type: &str, is_curl: bool) -> String {
    let (directive, type_desc, subgraph_rule) = diagram_type_info(diagram_type);

    let filename_rule = if is_curl {
        String::new()
    } else {
        "\n- Mermaid コードブロック内の **最初の行** に `%% filename: <slug>` の形式でファイル名スラグを出力する。スラグは図の内容を端的に表す半角小文字英字とアンダースコアのみの文字列にする（例: `%% filename: user_registration_flow`）。".to_string()
    };

    let rules = format!(
        r#"出力ルール（厳守）:
- 応答は **```mermaid で始まるフェンス付きコードブロック 1 つだけ**。その前後に説明文・見出し・箇条書きを書かない。
- 図は **`{directive}`** で始める。
- Mermaid は v11 でパース可能な記法にする。ノードラベルに `()` `:` `#` など記号が多い場合は `["..."]` 形式のラベルを使う。
- ルートが特定できない場合は「ルート不明」として分岐を書く。
- {subgraph_rule}
- 図中にトークン・Cookie・セッション ID・API キー・パスワード等の秘匿情報を一切含めない。ヘッダー値やパラメータ値を表示する必要がある場合は `****` に置き換える。{filename_rule}"#
    );

    if is_curl {
        format!(
            r#"あなたはこのワークスペース内の Rails アプリを読む AI Agent です。

次の HTTP リクエスト（ユーザーが入力した curl 相当の文字列全体）を解釈してください。

1. `config/routes.rb` から該当するルートと `Controller#action` を特定する（GET/POST 等はリクエストから推測）。
2. 該当コントローラと、そこから呼ばれる主要なモデル/スコープ/関連をコードに基づいて要約する。
3. {type_desc}

{rules}

ユーザー入力（curl 全体）:
{input}
"#
        )
    } else {
        format!(
            r#"あなたはこのワークスペース内の Rails アプリを読む AI Agent です。

以下のユーザーの説明を解釈し、関連するコードを特定してシステム図を生成してください。

1. 説明に含まれる画面操作・機能・処理を特定し、関連するルート・コントローラ・モデル・ビューをコードに基づいて特定する。
2. 特定した処理の流れを、コードの実装に基づいて正確に把握する。
3. {type_desc}

{rules}

ユーザーの説明:
{input}
"#
        )
    }
}

pub fn run_claude_agent(
    workspace: &Path,
    input: &str,
    diagram_type: &str,
    is_curl: bool,
) -> Result<String> {
    let claude = resolve_claude_cli();
    let prompt = build_prompt(input, diagram_type, is_curl);

    let model = env::var("DG_CLAUDE_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "claude-sonnet-4-6".to_string());

    let max_turns = env::var("DG_MAX_TURNS")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(20)
        .to_string();

    let output = Command::new(&claude)
        .args([
            "-p",
            "--dangerously-skip-permissions",
            "--model",
            &model,
            "--max-turns",
            &max_turns,
            &prompt,
        ])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to spawn Claude CLI ({claude:?})"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("claude exited with {}: {}", output.status, stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
