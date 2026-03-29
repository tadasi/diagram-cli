use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

// =============================================================================
// 設定 (Config)
// =============================================================================

const DIAGRAM_TYPES: &[(&str, &str)] = &[
    ("flowchart", "フローチャート"),
    ("sequence", "シーケンス図"),
    ("activity", "アクティビティ図"),
    ("component", "コンポーネント図"),
    ("state", "状態遷移図"),
];

struct DgConfig {
    workspace: String,
    diagram_type: String,
    output_dir: String,
}

impl DgConfig {
    fn config_path() -> Option<PathBuf> {
        let home = env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/dg/config.json"))
    }

    fn load() -> Option<DgConfig> {
        let path = Self::config_path()?;
        let text = fs::read_to_string(path).ok()?;
        Self::from_json(&text)
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("could not resolve config path")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        fs::write(&path, self.to_json()).map_err(|e| format!("write: {e}"))
    }

    fn workspace_abs(&self) -> PathBuf {
        home_dir().join(&self.workspace)
    }

    fn output_dir_abs(&self) -> PathBuf {
        home_dir().join(&self.output_dir)
    }

    fn diagram_type_label(&self) -> &str {
        DIAGRAM_TYPES
            .iter()
            .find(|(k, _)| *k == self.diagram_type)
            .map(|(_, v)| *v)
            .unwrap_or("フローチャート")
    }

    fn to_json(&self) -> String {
        format!(
            "{{\n  \"workspace\": \"{}\",\n  \"diagram_type\": \"{}\",\n  \"output_dir\": \"{}\"\n}}\n",
            self.workspace, self.diagram_type, self.output_dir
        )
    }

    fn from_json(text: &str) -> Option<DgConfig> {
        let ws = json_string_value(text, "workspace")?;
        let dt = json_string_value(text, "diagram_type")?;
        let od = json_string_value(text, "output_dir")?;
        Some(DgConfig { workspace: ws, diagram_type: dt, output_dir: od })
    }
}

fn json_string_value(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = json.find(&needle)? + needle.len();
    let rest = &json[i..];
    let colon = rest.find(':')?;
    let after = &rest[colon + 1..];
    let q1 = after.find('"')? + 1;
    let after_q = &after[q1..];
    let q2 = after_q.find('"')?;
    Some(after_q[..q2].to_string())
}

fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_default())
}

// =============================================================================
// 対話プロンプト
// =============================================================================

fn prompt_line(msg: &str) -> String {
    eprint!("{msg}");
    io::stderr().flush().ok();
    let mut buf = String::new();
    io::stdin().lock().read_line(&mut buf).ok();
    buf.trim().to_string()
}

fn prompt_yn(msg: &str) -> bool {
    loop {
        let ans = prompt_line(msg);
        match ans.to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => eprintln!("  Y または N で回答してください。"),
        }
    }
}

fn run_setup() -> DgConfig {
    let prev = DgConfig::load();
    eprintln!("=== dg: 初期設定 ===\n");

    // 1. ワークスペース
    let default_ws = prev.as_ref().map(|c| c.workspace.as_str()).unwrap_or("");
    eprintln!("対象プロジェクトディレクトリ（ルートからの相対パス）");
    eprintln!("  例: Projects/your-project");
    let workspace = loop {
        let prompt = "> ";
        let input = prompt_line(&prompt);
        if input.is_empty() {
            if !default_ws.is_empty() {
                break default_ws.to_string();
            }
            eprintln!("  パスを入力してください。");
            continue;
        }
        let abs = home_dir().join(&input);
        if !abs.is_dir() {
            eprintln!("  警告: ~/{input} は存在しません。このまま設定します。");
        }
        break input;
    };

    // 2. 図の種類
    let default_dt = prev.as_ref().map(|c| c.diagram_type.as_str()).unwrap_or("flowchart");
    let default_idx = DIAGRAM_TYPES
        .iter()
        .position(|(k, _)| *k == default_dt)
        .unwrap_or(0);
    eprintln!("\nシステム図の種類:");
    for (i, (_, label)) in DIAGRAM_TYPES.iter().enumerate() {
        eprintln!("  {}: {label}", i + 1);
    }
    let diagram_type = loop {
        let input = prompt_line("番号を選択: ");
        if input.is_empty() {
            break DIAGRAM_TYPES[default_idx].0.to_string();
        }
        if let Ok(n) = input.parse::<usize>() {
            if n >= 1 && n <= DIAGRAM_TYPES.len() {
                break DIAGRAM_TYPES[n - 1].0.to_string();
            }
        }
        eprintln!("  1〜{} の番号を入力してください。", DIAGRAM_TYPES.len());
    };

    // 3. 出力先
    let default_od = prev.as_ref().map(|c| c.output_dir.as_str()).unwrap_or("Desktop");
    eprintln!("\nファイルの出力先（ルートからの相対パス）");
    let output_dir = {
        let input = prompt_line(&format!("出力先 [{default_od}]: "));
        if input.is_empty() { default_od.to_string() } else { input }
    };

    let config = DgConfig { workspace, diagram_type, output_dir };
    if let Err(e) = config.save() {
        eprintln!("\ndg: 設定の保存に失敗: {e}");
        std::process::exit(1);
    }
    eprintln!("\n設定を保存しました。");
    config
}

fn print_config(config: &DgConfig) {
    eprintln!("--- 現在の設定 ---");
    eprintln!("  対象ディレクトリ : ~/{}", config.workspace);
    eprintln!("  図の種類         : {}", config.diagram_type_label());
    eprintln!("  出力先           : ~/{}", config.output_dir);
    eprintln!("------------------");
}

// =============================================================================
// main
// =============================================================================

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }

    // --- dg init ---
    if args[0] == "init" {
        run_setup();
        return;
    }

    // --- 設定の読み込み / 未初期化なら対話セットアップ → 確認ループ ---
    let config = loop {
        let c = match DgConfig::load() {
            Some(c) => c,
            None => {
                eprintln!("dg: 初期設定が見つかりません。セットアップを開始します。\n");
                run_setup()
            }
        };
        print_config(&c);
        if prompt_yn("このまま実行しますか？ (Y/N)（設定を変更する場合は N を選択してください）: ") {
            eprintln!();
            break c;
        }
        run_setup();
    };

    let workspace = config.workspace_abs();

    // --- 引数パース: CLI 引数を curl 形式に正規化し、URL・パスを抽出する ---
    let curl_parts = match resolve_curl_parts(args, &workspace) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("dg: {e}");
            print_usage();
            std::process::exit(2);
        }
    };

    if curl_parts.is_empty() {
        eprintln!("dg: need a URL or curl-style args");
        std::process::exit(2);
    }

    let curl_line = curl_parts.join(" ");
    let url = extract_url_from_parts(&curl_parts).unwrap_or_else(|| {
        eprintln!("dg: URL not found in args");
        std::process::exit(2);
    });

    let path = extract_path(&url).unwrap_or_else(|| {
        eprintln!("dg: could not parse URL: {url}");
        std::process::exit(2);
    });

    if !workspace.exists() {
        eprintln!("dg: workspace does not exist: {}", workspace.display());
        std::process::exit(2);
    }

    // --- コード解析: Claude CLI で Rails コードを読み、Mermaid 図を生成する ---
    let agent_out = run_claude_agent(&workspace, &curl_line, &config.diagram_type)
        .unwrap_or_else(|e| {
            eprintln!("dg: {e}");
            std::process::exit(1);
        });

    let mermaid = extract_mermaid_block(&agent_out).unwrap_or_else(|| {
        eprintln!(
            "dg: no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---"
        );
        std::process::exit(1);
    });

    // --- ファイル出力: .mmd と可視化用 HTML を出力先に書き出す ---
    let redacted = redact_curl_line(&curl_parts);
    let slug = path_to_slug(&path);
    let method = detect_http_method(&curl_parts);
    let ts = timestamp_suffix();
    let base_name = format!("dg_{slug}_{method}_{ts}");
    let title = format!("{slug} ({method})");

    let out_dir = config.output_dir_abs();
    if !out_dir.exists() {
        if let Err(e) = fs::create_dir_all(&out_dir) {
            eprintln!("dg: could not create output dir {}: {e}", out_dir.display());
            std::process::exit(2);
        }
    }

    let mmd_path = out_dir.join(format!("{base_name}.mmd"));
    if let Err(e) = fs::write(&mmd_path, &mermaid) {
        eprintln!("dg: failed to write {}: {e}", mmd_path.display());
        std::process::exit(1);
    }

    let html_path = out_dir.join(format!("{base_name}.html"));
    let html = mermaid_html_page(&title, &mermaid, &redacted);
    if let Err(e) = fs::write(&html_path, html) {
        eprintln!("dg: failed to write {}: {e}", html_path.display());
        std::process::exit(1);
    }

    // --- ブラウザ表示: HTML を open コマンドで開く ---
    let status = Command::new("open").arg(&html_path).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("dg: open failed with exit code: {s}"),
        Err(e) => eprintln!("dg: failed to run open: {e}"),
    }

    println!("{}", html_path.display());
}

// =============================================================================
// Usage
// =============================================================================

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  dg init                     # 初期設定（対象ディレクトリ・図の種類・出力先）");
    eprintln!("  dg [curl args...] <url>     # システム図を生成（先頭の curl は省略可）");
    eprintln!("  dg <https?://...>           # 単一 URL の省略形");
    eprintln!("  dg /path                    # DG_BASE_URL / .dg-base-url + パス");
    eprintln!("  dg resource_name            # 同上（先頭に / が付与される）");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  DG_BASE_URL     パスだけ渡すときのオリジン（例: http://localhost:3000）");
    eprintln!("  DG_CLAUDE_MODEL claude CLI の --model（未設定時は claude-sonnet-4-6）");
    eprintln!("  CLAUDE_CLI      claude 実行ファイルのパス（既定: PATH から解決）");
}

// =============================================================================
// curl 引数の解決
// =============================================================================

fn resolve_curl_parts(
    args: Vec<String>,
    workspace: &std::path::Path,
) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Err("no arguments".to_string());
    }

    let mut rest = args;
    if rest[0] == "curl" {
        rest.remove(0);
        if rest.is_empty() {
            return Err("need URL or args after 'curl'".to_string());
        }
        return Ok(rest);
    }

    if rest
        .iter()
        .any(|a| a.starts_with("http://") || a.starts_with("https://"))
    {
        return Ok(rest);
    }

    if rest[0] == "--location" || rest[0] == "-L" {
        return Ok(rest);
    }

    let base = resolve_base_url(workspace);
    if let Some(base) = base {
        let base = base.trim_end_matches('/').to_string();
        if rest.len() == 1 {
            let p = rest[0].trim();
            let path = if p.starts_with('/') {
                p.to_string()
            } else {
                format!("/{}", p.trim_start_matches('/'))
            };
            let url = format!("{base}{path}");
            return Ok(vec!["--location".to_string(), url]);
        }
    }

    Err(
        "need a URL (http/https), or --location/-L <url>, or one path with DG_BASE_URL / .dg-base-url"
            .to_string(),
    )
}

fn resolve_base_url(workspace: &std::path::Path) -> Option<String> {
    if let Ok(v) = env::var("DG_BASE_URL") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let p = workspace.join(".dg-base-url");
    let s = fs::read_to_string(p).ok()?;
    let line = s.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    Some(line.to_string())
}

// =============================================================================
// URL / パスユーティリティ
// =============================================================================

fn extract_url_from_parts(parts: &[String]) -> Option<String> {
    parts
        .iter()
        .find(|a| a.starts_with("http://") || a.starts_with("https://"))
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
}

fn extract_path(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let slash = after_scheme.find('/')?;
    let path_and_more = &after_scheme[slash..];
    let path = path_and_more.split('?').next().unwrap_or(path_and_more);
    Some(path.to_string())
}

fn path_to_slug(path: &str) -> String {
    let slug: String = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if slug.is_empty() {
        "diagram".to_string()
    } else {
        slug
    }
}

fn detect_http_method(parts: &[String]) -> String {
    let mut i = 0;
    while i < parts.len() {
        if (parts[i] == "-X" || parts[i] == "--request") && i + 1 < parts.len() {
            return parts[i + 1].to_lowercase();
        }
        i += 1;
    }
    if parts.iter().any(|a| {
        a == "-d"
            || a == "--data"
            || a == "--data-raw"
            || a == "--data-binary"
            || a == "--data-urlencode"
    }) {
        return "post".to_string();
    }
    "get".to_string()
}

fn timestamp_suffix() -> String {
    Command::new("date")
        .arg("+%Y%m%d_%H%M%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "00000000_000000".to_string())
}

// =============================================================================
// 秘匿処理
// =============================================================================

fn redact_curl_line(parts: &[String]) -> String {
    let sensitive_headers = [
        "authorization",
        "cookie",
        "set-cookie",
        "x-csrf-token",
        "x-api-key",
        "x-access-token",
    ];
    let sensitive_flags = ["-b", "--cookie", "-u", "--user"];

    let mut result = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let arg = &parts[i];
        if (arg == "-H" || arg == "--header") && i + 1 < parts.len() {
            let header = &parts[i + 1];
            if let Some(colon) = header.find(':') {
                let name = header[..colon].trim().to_lowercase();
                if sensitive_headers.iter().any(|h| name == *h) {
                    result.push(arg.clone());
                    result.push(format!("{}: ****", &header[..colon]));
                    i += 2;
                    continue;
                }
            }
            result.push(arg.clone());
            result.push(parts[i + 1].clone());
            i += 2;
        } else if sensitive_flags.iter().any(|f| arg == *f) && i + 1 < parts.len() {
            result.push(arg.clone());
            result.push("****".to_string());
            i += 2;
        } else if arg.starts_with("http://") || arg.starts_with("https://") {
            result.push(redact_url_params(arg));
            i += 1;
        } else {
            result.push(arg.clone());
            i += 1;
        }
    }
    format!("curl {}", result.join(" "))
}

fn redact_url_params(url: &str) -> String {
    let sensitive_keys = [
        "token",
        "access_token",
        "api_key",
        "apikey",
        "secret",
        "password",
        "key",
        "auth",
    ];
    if let Some(q) = url.find('?') {
        let (base, query) = url.split_at(q + 1);
        let redacted: Vec<String> = query
            .split('&')
            .map(|pair| {
                if let Some(eq) = pair.find('=') {
                    let k = &pair[..eq].to_lowercase();
                    if sensitive_keys.iter().any(|s| k.contains(s)) {
                        return format!("{}=****", &pair[..eq]);
                    }
                }
                pair.to_string()
            })
            .collect();
        format!("{}{}", base, redacted.join("&"))
    } else {
        url.to_string()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// =============================================================================
// Claude CLI 連携
// =============================================================================

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

fn run_claude_agent(
    workspace: &std::path::Path,
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

// =============================================================================
// Mermaid 抽出 / HTML 生成
// =============================================================================

fn extract_mermaid_block(text: &str) -> Option<String> {
    let markers = ["```mermaid\n", "```mermaid\r\n", "```mermaid\r"];
    for m in markers {
        if let Some(i) = text.find(m) {
            let rest = &text[i + m.len()..];
            if let Some(end) = rest.find("```") {
                let body = rest[..end].trim();
                if !body.is_empty() {
                    return Some(body.to_string());
                }
            }
        }
    }
    if let Some(i) = text.find("```mermaid") {
        let rest = &text[i + "```mermaid".len()..];
        let rest = rest.trim_start_matches(['\r', '\n', ' ']);
        if let Some(end) = rest.find("```") {
            let body = rest[..end].trim();
            if !body.is_empty() {
                return Some(body.to_string());
            }
        }
    }
    None
}

fn mermaid_html_page(title: &str, mermaid: &str, curl_line: &str) -> String {
    let escaped_curl = html_escape(curl_line);
    let escaped_title = html_escape(title);
    format!(
        r###"<!DOCTYPE html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
    <script src="https://cdn.tailwindcss.com"></script>
  </head>
  <body class="min-w-[1280px] min-h-[720px] m-0 p-0 bg-white">
    <div class="w-full flex flex-col gap-6 p-10">
      <div class="border border-gray-300 rounded-lg px-5 py-4 bg-gray-50">
        <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-2">Request</h3>
        <code class="text-sm text-gray-800 break-all">{escaped_curl}</code>
      </div>
      <div class="border-2 border-blue-300 rounded-lg p-6 bg-blue-50">
        <h2 class="text-xl font-bold text-blue-900 mb-4">{escaped_title}</h2>
        <div class="mermaid">
{mermaid}
        </div>
      </div>
    </div>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({{
        startOnLoad: true,
        theme: "base",
        themeVariables: {{
          primaryColor: "#dbeafe",
          primaryBorderColor: "#3b82f6",
          lineColor: "#6b7280",
          fontFamily: "system-ui, sans-serif"
        }}
      }});
    </script>
  </body>
</html>
"###,
    )
}
