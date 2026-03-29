use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// 入力テキストが curl コマンドかどうかを判定する
pub fn is_curl_like(input: &str) -> bool {
    let first = input.trim().lines().next().unwrap_or("").trim();
    first.starts_with("curl ")
        || first.starts_with("http://")
        || first.starts_with("https://")
        || first.starts_with("--location")
        || first.starts_with("-L ")
        || first.starts_with("-X ")
        || first.starts_with("-H ")
}

/// 複数行の curl 文字列をパーツに分割する
pub fn parse_curl_string(input: &str) -> Vec<String> {
    let normalized = input.replace("\\\n", " ").replace("\\\r\n", " ");
    let mut parts: Vec<String> = normalized
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if parts.first().map(|s| s.as_str()) == Some("curl") {
        parts.remove(0);
    }
    parts
}

pub fn resolve_curl_parts(
    args: Vec<String>,
    workspace: &Path,
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

fn resolve_base_url(workspace: &Path) -> Option<String> {
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

pub fn extract_url_from_parts(parts: &[String]) -> Option<String> {
    parts
        .iter()
        .find(|a| a.starts_with("http://") || a.starts_with("https://"))
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
}

pub fn extract_path(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let slash = after_scheme.find('/')?;
    let path_and_more = &after_scheme[slash..];
    let path = path_and_more.split('?').next().unwrap_or(path_and_more);
    Some(path.to_string())
}

pub fn path_to_slug(path: &str) -> String {
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

pub fn detect_http_method(parts: &[String]) -> String {
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

pub fn timestamp_suffix() -> String {
    Command::new("date")
        .arg("+%Y%m%d_%H%M%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "00000000_000000".to_string())
}
