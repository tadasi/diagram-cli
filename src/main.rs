mod claude;
mod config;
mod curl;
mod mermaid;
mod prompt;
mod sanitize;

use std::env;
use std::fs;
use std::process::Command;

use config::DgConfig;
use curl::{
    detect_http_method, extract_path, extract_url_from_parts, path_to_slug, resolve_curl_parts,
    timestamp_suffix,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }

    // --- dg init ---
    if args[0] == "init" {
        prompt::run_setup();
        return;
    }

    // --- 設定の読み込み / 未初期化なら対話セットアップ → 確認ループ ---
    let config = loop {
        let c = match DgConfig::load() {
            Some(c) => c,
            None => {
                eprintln!("dg: 初期設定が見つかりません。セットアップを開始します。\n");
                prompt::run_setup()
            }
        };
        prompt::print_config(&c);
        if prompt::prompt_yn(
            "このまま実行しますか？ (Y/N)（設定を変更する場合は N を選択してください）: ",
        ) {
            eprintln!();
            break c;
        }
        prompt::run_setup();
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
    let agent_out =
        claude::run_claude_agent(&workspace, &curl_line, &config.diagram_type).unwrap_or_else(
            |e| {
                eprintln!("dg: {e}");
                std::process::exit(1);
            },
        );

    let mermaid_src = mermaid::extract_mermaid_block(&agent_out).unwrap_or_else(|| {
        eprintln!(
            "dg: no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---"
        );
        std::process::exit(1);
    });

    // --- ファイル出力: .mmd と可視化用 HTML を出力先に書き出す ---
    let redacted = sanitize::redact_curl_line(&curl_parts);
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
    if let Err(e) = fs::write(&mmd_path, &mermaid_src) {
        eprintln!("dg: failed to write {}: {e}", mmd_path.display());
        std::process::exit(1);
    }

    let html_path = out_dir.join(format!("{base_name}.html"));
    let html = mermaid::mermaid_html_page(&title, &mermaid_src, &redacted);
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
