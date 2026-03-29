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
    detect_http_method, extract_path, extract_url_from_parts, is_curl_like, parse_curl_string,
    path_to_slug, resolve_curl_parts, timestamp_suffix,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // --- dg init / --help ---
    match args.first().map(|s| s.as_str()) {
        Some("init") => {
            prompt::run_setup();
            return;
        }
        Some("--help" | "-h") => {
            print_usage();
            return;
        }
        _ => {}
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
        eprintln!("指定のコードを分析し、システム図を出力します。");
        eprintln!("設定を確認してください。\n");
        prompt::print_config(&c);
        if prompt::prompt_yn(
            "設定を変更しますか？ (Y/N): ",
         ) {
            prompt::run_setup();
        } else {
            eprintln!();
            break c;
        }
    };

    let workspace = config.workspace_abs();

    if !workspace.exists() {
        eprintln!("dg: workspace does not exist: {}", workspace.display());
        std::process::exit(2);
    }

    // --- 入力: CLI 引数 or 対話入力 ---
    let input = if args.is_empty() {
        let text = prompt::read_multiline_input();
        if text.is_empty() {
            eprintln!("dg: 入力がありません");
            std::process::exit(2);
        }
        text
    } else {
        args.join(" ")
    };

    let is_curl = is_curl_like(&input);

    eprintln!("コード分析中……");

    // --- コード解析: Claude CLI で Mermaid 図を生成する ---
    let agent_out = if is_curl {
        claude::run_claude_agent(&workspace, &input, &config.diagram_type)
    } else {
        claude::run_claude_agent_freetext(&workspace, &input, &config.diagram_type)
    }
    .unwrap_or_else(|e| {
        eprintln!("dg: {e}");
        std::process::exit(1);
    });

    let mermaid_src = mermaid::extract_mermaid_block(&agent_out).unwrap_or_else(|| {
        eprintln!(
            "dg: no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---"
        );
        std::process::exit(1);
    });

    // --- ファイル出力 ---
    let ts = timestamp_suffix();

    let (title, base_name, display_input, input_label) = if is_curl {
        let parts = if args.is_empty() {
            parse_curl_string(&input)
        } else {
            match resolve_curl_parts(args, &workspace) {
                Ok(p) => p,
                Err(_) => parse_curl_string(&input),
            }
        };
        let url = extract_url_from_parts(&parts).unwrap_or_default();
        let path = extract_path(&url).unwrap_or_else(|| "/".to_string());
        let slug = path_to_slug(&path);
        let method = detect_http_method(&parts);
        let redacted = sanitize::redact_curl_line(&parts);
        (
            format!("{slug} ({method})"),
            format!("dg_{slug}_{method}_{ts}"),
            redacted,
            "Request",
        )
    } else {
        (
            "システム図".to_string(),
            format!("dg_freetext_{ts}"),
            input.clone(),
            "Description",
        )
    };

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
    let html = mermaid::mermaid_html_page(&title, &mermaid_src, &display_input, input_label);
    if let Err(e) = fs::write(&html_path, html) {
        eprintln!("dg: failed to write {}: {e}", html_path.display());
        std::process::exit(1);
    }

    eprintln!("完了しました。出力内容を確認してください。");

    // --- ブラウザ表示 ---
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
    eprintln!("  dg                          # 対話形式で入力（curl / 自由テキスト）");
    eprintln!("  dg init                     # 初期設定（対象ディレクトリ・図の種類・出力先）");
    eprintln!("  dg [curl args...] <url>     # API 単位のシステム図を生成");
    eprintln!("  dg <自由テキスト>           # 画面操作手順等から包括的なシステム図を生成");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  DG_BASE_URL     パスだけ渡すときのオリジン（例: http://localhost:3000）");
    eprintln!("  DG_CLAUDE_MODEL claude CLI の --model（未設定時は claude-sonnet-4-6）");
    eprintln!("  CLAUDE_CLI      claude 実行ファイルのパス（既定: PATH から解決）");
}
