use std::io::{self, Write};

use anyhow::Result;

use crate::config::{home_dir, DgConfig, DIAGRAM_TYPES};

pub fn prompt_line(msg: &str) -> String {
    eprint!("{msg}");
    io::stderr().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim().to_string()
}

pub fn prompt_yn(msg: &str) -> bool {
    loop {
        let ans = prompt_line(msg);
        match ans.to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => eprintln!("  Y または N で回答してください。"),
        }
    }
}

pub fn run_setup() -> Result<DgConfig> {
    let prev = DgConfig::load();
    eprintln!("=== dg: 初期設定 ===\n");

    let default_ws = prev.as_ref().map(|c| c.workspace.as_str()).unwrap_or("");
    eprintln!("対象プロジェクトディレクトリ（ルートからの相対パス）");
    eprintln!("  例: Projects/your-project");
    let workspace = loop {
        let input = prompt_line("> ");
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

    let default_od = prev.as_ref().map(|c| c.output_dir.as_str()).unwrap_or("Desktop");
    eprintln!("\nファイルの出力先（ルートからの相対パス）");
    let output_dir = {
        let input = prompt_line(&format!("出力先 [{default_od}]: "));
        if input.is_empty() { default_od.to_string() } else { input }
    };

    let config = DgConfig { workspace, diagram_type, output_dir };
    config.save()?;
    eprintln!("\n設定を保存しました。");
    Ok(config)
}

pub fn prompt_input() -> String {
    eprintln!("分析対象の詳細を指定してください:");
    eprintln!("  API 単位の分析 → curl コマンドを入力");
    eprintln!("  包括的な分析   → 画面操作手順や機能説明を自由テキストで入力");
    eprintln!("  （行末に \\ で継続入力 / 複数行入力は Ctrl+D で送信）");

    let mut result = String::new();
    let mut in_single_quote = false;
    let mut first = true;

    let stdin = io::stdin();
    loop {
        let prompt = if first { "> " } else { "  " };
        first = false;
        eprint!("{prompt}");
        io::stderr().flush().ok();

        let mut buf = String::new();
        match stdin.read_line(&mut buf) {
            Ok(0) | Err(_) => break, // EOF (Ctrl+D)
            _ => {}
        }
        let line = buf.trim_end_matches('\n').trim_end_matches('\r');

        // シングルクォートの開閉を追跡
        for ch in line.chars() {
            if ch == '\'' {
                in_single_quote = !in_single_quote;
            }
        }

        if line.ends_with('\\') && !in_single_quote {
            result.push_str(&line[..line.len() - 1]);
            result.push(' ');
        } else {
            result.push_str(line);
            if !in_single_quote {
                break;
            }
            result.push('\n');
        }
    }
    result.trim().to_string()
}

pub fn print_config(config: &DgConfig) {
    eprintln!("--------------現在の設定--------------");
    eprintln!("  コードの分析対象ディレクトリ : ~/{}", config.workspace);
    eprintln!("  システム図の種類             : {}", config.diagram_type_label());
    eprintln!("  出力先                       : ~/{}", config.output_dir);
    eprintln!("------------------------------------\n");
}
