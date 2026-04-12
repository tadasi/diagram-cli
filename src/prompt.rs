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

pub fn should_change_settings() -> bool {
    loop {
        let ans = prompt_line("設定を変更しますか？ (y/n): ");
        match ans.to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => eprintln!("  y(/yes) または n(/no) で回答してください。"),
        }
    }
}

// セットアップ実行
pub fn run_setup() -> Result<DgConfig> {
    let prev_setting = DgConfig::load(); // 既存設定の読み込み
    eprintln!("=== dg: 初期設定 ===\n");

    //
    // 入力値に基づいた、「分析対象ディレクトリパス」の設定
    //
    //   as_ref: 元の参照用設定オブジェクトを消費せず（所有権を移動せず）、元の参照を含む Result 型のオブジェクトを返す
    //   unwrap_or: Result 型の値が Ok または Some であればその値を取り出し、そうでなければデフォルト値（空文字）を返す
    let default_ws = prev_setting.as_ref().map(|c| c.workspace.as_str()).unwrap_or("");
    eprintln!("分析対象プロジェクトディレクトリを、ルートからの相対パスで指定（変更が不要な場合は、Enter キーを押下）してください。");
    eprintln!("  例: Projects/your-project");
    let workspace = loop {
        let input = prompt_line("> ");
        if input.is_empty() {
            if !default_ws.is_empty() {
                break default_ws.to_string();
            }
            eprintln!("  ディレクトリパスを入力してください。");
            continue;
        }
        let abs = home_dir().join(&input);
        if !abs.is_dir() {
            eprintln!("  ~/{input} ディレクトリは存在しません。正確なパスを指定してください。");
            continue;
        }
        break input;
    };

    //
    // 入力値に基づいた、「出力システム図の種類」設定
    //
    let default_dt = prev_setting.as_ref().map(|c| c.diagram_type.as_str()).unwrap_or("flowchart");
    let default_idx = DIAGRAM_TYPES
        .iter()
        .position(|(k, _)| *k == default_dt)
        .unwrap_or(0);
    eprintln!("\nシステム図の種類を選択（変更が不要な場合は、Enter キー押下）してください:");
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

    //
    // 入力値に基づいた、「ファイル出力先」設定
    //
    let default_od = prev_setting.as_ref().map(|c| c.output_dir.as_str()).unwrap_or("Desktop");
    eprintln!("\nファイルの出力先を、ルートからの相対パスで指定（変更が不要な場合は、Enter キー押下）してください。");
    eprintln!("  デフォルト: Desktop");
    let output_dir = {
        let input = prompt_line("> ");
        if input.is_empty() { default_od.to_string() } else { input }
    };

    //
    // 設定保存
    //
    let config = DgConfig { workspace, diagram_type, output_dir };
    config.save()?;
    eprintln!("\n設定を保存しました。");
    Ok(config)
}

// 
pub fn prompt_input() -> String {
    eprintln!("分析対象の詳細を指定してください:");
    eprintln!("  API 単位の分析 → curl コマンドを入力");
    eprintln!("  包括的な分析   → 画面操作手順や機能説明を自由テキストで入力");
    eprintln!("  （行末に \\ を加えると、継続行の入力が可能）");

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
            Ok(0) | Err(_) => break, // stdin が閉じた場合は入力終了
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
            // 末尾のバックスラッシュを取り除き、スペースに置換（複数行を連結）する
            result.push_str(&line[..line.len() - 1]);
            result.push(' ');
        } else {
            result.push_str(line);
            if !in_single_quote { // シングルクォートが閉じられているなら、入力完了とみなしループを抜ける
                break;
            }
            result.push('\n'); // シングルクォートが閉じられていないなら、改行を入れた上で入力受付を続行する
        }
    }
    result.trim().to_string()
}

// 設定情報の出力
pub fn print_config(config: &DgConfig) {
    eprintln!("--------------現在の設定--------------");
    eprintln!("  コードの分析対象ディレクトリ : ~/{}", config.workspace);
    eprintln!("  システム図の種類             : {}", config.diagram_type_label());
    eprintln!("  出力先                       : ~/{}", config.output_dir);
    eprintln!("------------------------------------\n");
}
