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

pub fn resolve_curl_parts(args: Vec<String>) -> Result<Vec<String>> {
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

    bail!("need a URL (http/https) or --location/-L <url>")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_curl_like_with_url() {
        assert!(is_curl_like("http://localhost:3000/users"));
        assert!(is_curl_like("https://example.com/api"));
    }

    #[test]
    fn is_curl_like_with_curl_prefix() {
        assert!(is_curl_like("curl http://localhost:3000/users"));
        assert!(is_curl_like("curl"));
    }

    #[test]
    fn is_curl_like_with_flags() {
        assert!(is_curl_like("--location http://localhost:3000"));
        assert!(is_curl_like("-X POST http://localhost:3000"));
        assert!(is_curl_like("-H 'Content-Type: application/json'"));
    }

    #[test]
    fn is_curl_like_freetext() {
        assert!(!is_curl_like("ユーザー登録のフロー"));
        assert!(!is_curl_like("login flow"));
    }

    #[test]
    fn parse_curl_string_strips_curl() {
        let parts = parse_curl_string("curl -X POST http://localhost:3000/users");
        assert_eq!(parts, vec!["-X", "POST", "http://localhost:3000/users"]);
    }

    #[test]
    fn parse_curl_string_handles_backslash_continuation() {
        let input = "curl -X POST \\\nhttp://localhost:3000/users";
        let parts = parse_curl_string(input);
        assert_eq!(parts, vec!["-X", "POST", "http://localhost:3000/users"]);
    }

    #[test]
    fn resolve_curl_parts_with_url() {
        let args = vec!["-X".into(), "GET".into(), "http://localhost:3000/users".into()];
        let result = resolve_curl_parts(args).unwrap();
        assert_eq!(result, vec!["-X", "GET", "http://localhost:3000/users"]);
    }

    #[test]
    fn resolve_curl_parts_strips_curl() {
        let args = vec!["curl".into(), "http://localhost:3000".into()];
        let result = resolve_curl_parts(args).unwrap();
        assert_eq!(result, vec!["http://localhost:3000"]);
    }

    #[test]
    fn resolve_curl_parts_empty_args() {
        assert!(resolve_curl_parts(vec![]).is_err());
    }

    #[test]
    fn resolve_curl_parts_no_url() {
        assert!(resolve_curl_parts(vec!["hello".into()]).is_err());
    }

    #[test]
    fn extract_url_from_parts_found() {
        let parts = vec!["-X".into(), "POST".into(), "'http://localhost:3000/users'".into()];
        assert_eq!(
            extract_url_from_parts(&parts),
            Some("http://localhost:3000/users".into())
        );
    }

    #[test]
    fn extract_url_from_parts_none() {
        let parts = vec!["-X".into(), "POST".into()];
        assert_eq!(extract_url_from_parts(&parts), None);
    }

    #[test]
    fn extract_path_basic() {
        assert_eq!(extract_path("http://localhost:3000/users"), Some("/users".into()));
        assert_eq!(extract_path("https://example.com/api/v1?key=val"), Some("/api/v1".into()));
    }

    #[test]
    fn extract_path_no_path() {
        assert_eq!(extract_path("http://localhost:3000"), None);
    }

    #[test]
    fn path_to_slug_basic() {
        assert_eq!(path_to_slug("/users"), "users");
        assert_eq!(path_to_slug("/api/v1/articles"), "api_v1_articles");
    }

    #[test]
    fn path_to_slug_skips_numeric_segments() {
        assert_eq!(path_to_slug("/users/123/posts"), "users_posts");
    }

    #[test]
    fn path_to_slug_empty() {
        assert_eq!(path_to_slug("/"), "diagram");
    }

    #[test]
    fn detect_http_method_explicit() {
        let parts = vec!["-X".into(), "DELETE".into(), "http://localhost/users/1".into()];
        assert_eq!(detect_http_method(&parts), "delete");
    }

    #[test]
    fn detect_http_method_implicit_post() {
        let parts = vec!["-d".into(), "name=test".into(), "http://localhost/users".into()];
        assert_eq!(detect_http_method(&parts), "post");
    }

    #[test]
    fn detect_http_method_default_get() {
        let parts = vec!["http://localhost/users".into()];
        assert_eq!(detect_http_method(&parts), "get");
    }
}
