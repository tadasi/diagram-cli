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
    let args: Vec<String> = env::args().skip(1).collect(); // dg 以降の引数を全てコレクションに変換

    match args.first().map(|s| s.as_str()) {
        // 初期設定
        Some("init") => {
            prompt::run_setup()?;
            return Ok(());
        }
        // ヘルプ
        Some("--help" | "-h") => {
            print_usage();
            return Ok(());
        }
        // 上記以外が指定された場合は、以降の処理に進む
        _ => {}
    }

    // 設定確認
    let config = setup_config_interactive()?;

    // 設定済みの「分析対象ディレクトリ」が現在も存在するか確認
    let workspace = config.workspace_full_path();
    if !workspace.exists() {
        bail!("分析対象のディレクトリが存在しません。再設定をお願いします。: {}", workspace.display());
    }

    let input = if args.is_empty() {
        let text = prompt::prompt_input();
        if text.is_empty() {
            bail!("入力がありません"); // bail!: return Err(anyhow!(...)) の簡略化
        }
        text
    } else {
        args.join(" ")
    };

    // 引数に curl コマンドが指定された場合は、秘匿情報をサニタイズしてからプロンプトに渡す
    let is_curl = is_curl_like(&input);
    let prompt_input = if is_curl {
        let parts = parse_curl_string(&input);
        sanitize::redact_curl_line(&parts)
    } else {
        input.clone()
    };

    eprintln!("\nコード分析中……");

    let agent_out =
        claude::run_claude_agent(&workspace, &prompt_input, &config.diagram_type, is_curl)?;

    let raw_mermaid = mermaid::extract_mermaid_block(&agent_out).context(format!(
        "no Mermaid code block in Claude output. Raw output follows:\n---\n{agent_out}\n---"
    ))?;

    let ts = timestamp_suffix();

    let (title, base_name, mermaid_src, display_input, input_label) = if is_curl {
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
            raw_mermaid,
            redacted,
            "Request",
        )
    } else {
        let (slug_opt, body) = mermaid::extract_filename_slug(&raw_mermaid);
        let slug = slug_opt.unwrap_or_else(|| "freetext".to_string());
        (
            "システム図".to_string(),
            format!("dg_{slug}_{ts}"),
            body,
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
    let diagram_type_label = config.diagram_type_label();
    let html = mermaid::mermaid_html_page(&title, &mermaid_src, &display_input, input_label, diagram_type_label);
    fs::write(&html_path, html)?;

    eprintln!("実行が完了しました。出力内容を確認してください。");

    let _ = Command::new("open").arg(&html_path).status();

    println!("出力ファイル: {}", html_path.display());
    Ok(())
}

// インタラクティブに設定を行う
fn setup_config_interactive() -> Result<DgConfig> {
    loop {
        let c = match DgConfig::load() {
            Some(c) => c,
            None => {
                eprintln!("dg: 初期設定が見つかりません。セットアップを開始します。\n");
                prompt::run_setup()?
            }
        };
        eprintln!("指定のソースコードを分析し、システム図を出力します。");
        eprintln!("設定を確認してください。\n");
        prompt::print_config(&c);

        // 必要に応じて、設定変更
        if prompt::should_change_settings() {
            prompt::run_setup()?;
        } else {
            eprintln!();
            return Ok(c);
        }
    }
}

// ヘルプ情報
fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  dg                          # 対話形式で入力（curl / 自由テキスト）");
    eprintln!("  dg init                     # 初期設定（分析対象ディレクトリ指定・システム図の種類選択・出力先指定）");
    eprintln!("  dg [curl args...] <url>     # API 単位のシステム図を生成");
    eprintln!("  dg <自由テキスト>           # 画面操作手順等から包括的なシステム図を生成");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  DG_BASE_URL     パスだけ渡すときのオリジン（例: http://localhost:3000）");
    eprintln!("  DG_CLAUDE_MODEL claude CLI の --model（未設定時は claude-sonnet-4-6）");
    eprintln!("  CLAUDE_CLI      claude 実行ファイルのパス（既定: PATH から解決）");
}
