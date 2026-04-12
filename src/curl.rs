use std::env;
use std::fs;
use std::path::Path;

use anyhow::{bail, Result};
use chrono::Local;

pub fn is_curl_like(input: &str) -> bool {
    let first = input.trim().lines().next().unwrap_or("").trim();
    // 先頭の "dg" / "curl" を読み飛ばして実質的なトークンで判定
    let mut s = first;
    if let Some(rest) = s.strip_prefix("dg ") {
        s = rest.trim_start();
    }
    if let Some(rest) = s.strip_prefix("curl ") {
        s = rest.trim_start();
    } else if s == "curl" {
        return true;
    }
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("--location")
        || s.starts_with("-L ")
        || s.starts_with("-X ")
        || s.starts_with("-H ")
}

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

pub fn resolve_curl_parts(args: Vec<String>, workspace: &Path) -> Result<Vec<String>> {
    if args.is_empty() {
        bail!("no arguments");
    }

    let mut rest = args;
    if rest[0] == "curl" {
        rest.remove(0);
        if rest.is_empty() {
            bail!("need URL or args after 'curl'");
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

    if let Some(base) = resolve_base_url(workspace) {
        let base = base.trim_end_matches('/');
        if rest.len() == 1 {
            let p = rest[0].trim();
            let path = if p.starts_with('/') {
                p.to_string()
            } else {
                format!("/{}", p.trim_start_matches('/'))
            };
            return Ok(vec!["--location".to_string(), format!("{base}{path}")]);
        }
    }

    bail!("need a URL (http/https), or --location/-L <url>, or one path with DG_BASE_URL / .dg-base-url")
}

fn resolve_base_url(workspace: &Path) -> Option<String> {
    if let Ok(v) = env::var("DG_BASE_URL") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let s = fs::read_to_string(workspace.join(".dg-base-url")).ok()?;
    let line = s.lines().next()?.trim();
    if line.is_empty() { None } else { Some(line.to_string()) }
}

pub fn extract_url_from_parts(parts: &[String]) -> Option<String> {
    parts
        .iter()
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
        .find(|a| a.starts_with("http://") || a.starts_with("https://"))
}

pub fn extract_path(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let slash = after_scheme.find('/')?;
    let path_and_more = &after_scheme[slash..];
    Some(path_and_more.split('?').next().unwrap_or(path_and_more).to_string())
}

pub fn path_to_slug(path: &str) -> String {
    let slug: String = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.chars().all(|c| c.is_ascii_digit()))
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if slug.is_empty() { "diagram".to_string() } else { slug }
}

pub fn detect_http_method(parts: &[String]) -> String {
    for pair in parts.windows(2) {
        if pair[0] == "-X" || pair[0] == "--request" {
            return pair[1].to_lowercase();
        }
    }
    if parts.iter().any(|a| {
        matches!(
            a.as_str(),
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode"
        )
    }) {
        return "post".to_string();
    }
    "get".to_string()
}

pub fn timestamp_suffix() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}
