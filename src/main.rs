mod claude;
mod config;
mod curl;
mod mermaid;
mod prompt;
mod sanitize;

use std::env;
use std::fs;
use std::process::Command;

use anyhow::{bail, Context, Result};
use config::DgConfig;
use curl::{
    detect_http_method, extract_path, extract_url_from_parts, is_curl_like, parse_curl_string,
    path_to_slug, resolve_curl_parts, timestamp_suffix,
};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(|s| s.as_str()) {
        Some("init") => {
            prompt::run_setup()?;
            return Ok(());
        }
        Some("--help" | "-h") => {
            print_usage();
            return Ok(());
        }
        _ => {}
    }

    let config = load_config_interactive()?;
    let workspace = config.workspace_abs();

    if !workspace.exists() {
        bail!("workspace does not exist: {}", workspace.display());
    }

    let input = if args.is_empty() {
        let text = prompt::prompt_input();
        if text.is_empty() {
            bail!("入力がありません");
        }
        text
    } else {
        args.join(" ")
    };

    let is_curl = is_curl_like(&input);

    eprintln!("\nコード分析中……");

    let agent_out = claude::run_claude_agent(&workspace, &input, &config.diagram_type, is_curl)?;

    let mermaid_src = mermaid::extract_mermaid_block(&agent_out).context(format!(
        "no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---"
    ))?;

    let ts = timestamp_suffix();

    let (title, base_name, display_input, input_label) = if is_curl {
        let parts = if args.is_empty() {
            parse_curl_string(&input)
        } else {
            resolve_curl_parts(args, &workspace).unwrap_or_else(|_| parse_curl_string(&input))
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
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("could not create output dir {}", out_dir.display()))?;

    let mmd_path = out_dir.join(format!("{base_name}.mmd"));
    fs::write(&mmd_path, &mermaid_src)?;

    let html_path = out_dir.join(format!("{base_name}.html"));
    let html = mermaid::mermaid_html_page(&title, &mermaid_src, &display_input, input_label);
    fs::write(&html_path, html)?;

    eprintln!("実行が完了しました。出力内容を確認してください。");

    let _ = Command::new("open").arg(&html_path).status();

    println!("出力ファイル: {}", html_path.display());
    Ok(())
}

fn load_config_interactive() -> Result<DgConfig> {
    loop {
        let c = match DgConfig::load() {
            Some(c) => c,
            None => {
                eprintln!("dg: 初期設定が見つかりません。セットアップを開始します。\n");
                prompt::run_setup()?
            }
        };
        eprintln!("指定のコードを分析し、システム図を出力します。");
        eprintln!("設定を確認してください。\n");
        prompt::print_config(&c);
        if prompt::prompt_yn("設定を変更しますか？ (Y/N): ") {
            prompt::run_setup()?;
        } else {
            eprintln!();
            return Ok(c);
        }
    }
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
