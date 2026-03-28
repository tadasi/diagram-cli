use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }

    let workspace = resolve_workspace();
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
        eprintln!("dg: DG_WORKSPACE does not exist: {}", workspace.display());
        std::process::exit(2);
    }

    let agent_out = run_claude_agent(&workspace, &curl_line).unwrap_or_else(|e| {
        eprintln!("dg: {e}");
        std::process::exit(1);
    });

    let mermaid = extract_mermaid_block(&agent_out).unwrap_or_else(|| {
        eprintln!("dg: no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---");
        std::process::exit(1);
    });

    let base = path_to_base_name(&path);
    let title = format!("{base} (from curl)");
    let mmd_name = format!("dg_{base}.mmd");
    let html_name = format!("dg_{base}.html");

    let mmd_path = desktop_path(&mmd_name).unwrap_or_else(|| {
        eprintln!("dg: could not resolve ~/Desktop");
        std::process::exit(2);
    });
    if let Err(e) = fs::write(&mmd_path, &mermaid) {
        eprintln!("dg: failed to write {}: {e}", mmd_path.display());
        std::process::exit(1);
    }

    let html_path = desktop_path(&html_name).unwrap_or_else(|| {
        eprintln!("dg: could not resolve ~/Desktop");
        std::process::exit(2);
    });
    let html = mermaid_html_page(&title, &mermaid);
    if let Err(e) = fs::write(&html_path, html) {
        eprintln!("dg: failed to write {}: {e}", html_path.display());
        std::process::exit(1);
    }

    let status = Command::new("open").arg(&html_path).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("dg: open failed with exit code: {s}"),
        Err(e) => eprintln!("dg: failed to run open: {e}"),
    }

    println!("{}", html_path.display());
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  dg [curl args...] <url>     # --location / -L や URL をそのまま指定可（先頭の curl は省略可）");
    eprintln!("  dg <https?://...>           # 単一 URL の省略形");
    eprintln!("  dg /path                    # DG_BASE_URL または DG_WORKSPACE/.dg-base-url が必要");
    eprintln!("  dg resource_name            # 同上（先頭に / が付与される）");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  DG_WORKSPACE    Rails アプリのルート（既定: ~/Projects/tech-index があればそれ、なければカレント）");
    eprintln!("  DG_BASE_URL     パスだけ渡すときのオリジン（例: http://localhost:3000）。未設定時は .dg-base-url を参照");
    eprintln!("  DG_CLAUDE_MODEL claude CLI の --model（未設定時は claude-sonnet-4-6）");
    eprintln!("  CLAUDE_CLI      claude 実行ファイルのパス（既定: PATH から解決）");
}

/// `dg curl ...` に加え、URL 直指定・`-L` のみ・ベース URL + パスを受け付ける。
fn resolve_curl_parts(args: Vec<String>, workspace: &std::path::Path) -> Result<Vec<String>, String> {
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

    if rest.iter().any(|a| a.starts_with("http://") || a.starts_with("https://")) {
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
    read_workspace_base_url(workspace)
}

fn read_workspace_base_url(workspace: &std::path::Path) -> Option<String> {
    let p = workspace.join(".dg-base-url");
    let s = fs::read_to_string(p).ok()?;
    let line = s.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    Some(line.to_string())
}

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

fn path_to_base_name(path: &str) -> String {
    let p = path.trim_end_matches('/');
    let s = p.rsplit('/').next().unwrap_or("diagram");
    if s.is_empty() {
        "diagram".to_string()
    } else {
        s.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }
}

fn resolve_workspace() -> PathBuf {
    if let Ok(p) = env::var("DG_WORKSPACE") {
        return PathBuf::from(p);
    }
    if let Ok(home) = env::var("HOME") {
        let tech = PathBuf::from(home).join("Projects/tech-index");
        if tech.is_dir() {
            return tech;
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_claude_cli() -> PathBuf {
    if let Ok(p) = env::var("CLAUDE_CLI") {
        let pb = PathBuf::from(p.trim());
        if pb.exists() {
            return pb;
        }
    }
    PathBuf::from("claude")
}

fn build_agent_prompt(curl_line: &str) -> String {
    format!(
        r#"あなたはこのワークスペース内の Rails アプリを読む AI Agent です。

次の HTTP リクエスト（ユーザーが入力した curl 相当の文字列全体）を解釈してください。

1. `config/routes.rb` から該当するルートと `Controller#action` を特定する（GET/POST 等はリクエストから推測）。
2. 該当コントローラと、そこから呼ばれる主要なモデル/スコープ/関連をコードに基づいて要約する。
3. 処理の流れを **Mermaid の flowchart（`flowchart TD`）** で表現する。

出力ルール（厳守）:
- 応答は **```mermaid で始まるフェンス付きコードブロック 1 つだけ**。その前後に説明文・見出し・箇条書きを書かない。
- Mermaid は v11 でパース可能な記法にする。ノードラベルに `()` `:` `#` など記号が多い場合は `["..."]` 形式のラベルを使う。
- ルートが特定できない場合は「ルート不明」として分岐を書く。

ユーザー入力（curl 全体）:
{curl_line}
"#
    )
}

fn run_claude_agent(workspace: &std::path::Path, curl_line: &str) -> Result<String, String> {
    let claude = resolve_claude_cli();
    let prompt = build_agent_prompt(curl_line);

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

fn desktop_path(filename: &str) -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    p.push("Desktop");
    p.push(filename);
    Some(p)
}

fn mermaid_html_page(title: &str, mermaid: &str) -> String {
    format!(
        r###"<!DOCTYPE html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{}</title>
    <script src="https://cdn.tailwindcss.com"></script>
  </head>
  <body class="w-[1280px] h-[720px] m-0 p-0 overflow-hidden bg-white">
    <div class="w-full h-full flex flex-col gap-6 p-10">
      <div class="border-2 border-blue-300 rounded-lg p-6 bg-blue-50">
        <h2 class="text-xl font-bold text-blue-900 mb-4">{}</h2>
        <div class="mermaid">
{}
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
        title, title, mermaid
    )
}
