fn is_curl_flag(s: &str) -> bool {
    s.starts_with('-')
        || s.starts_with("http://")
        || s.starts_with("https://")
}

fn is_sensitive_header(name: &str) -> bool {
    let sensitive = [
        "authorization",
        "cookie",
        "set-cookie",
        "x-csrf-token",
        "x-api-key",
        "x-access-token",
    ];
    let lower = name.trim().trim_matches('"').trim_matches('\'').to_lowercase();
    sensitive.iter().any(|h| lower == *h)
}

pub fn redact_curl_line(parts: &[String]) -> String {
    let sensitive_flags = ["-b", "--cookie", "-u", "--user"];

    let mut result = Vec::new();
    let mut omitted = false;
    let mut i = 0;
    while i < parts.len() {
        let arg = &parts[i];
        if (arg == "-H" || arg == "--header") && i + 1 < parts.len() {
            let header = &parts[i + 1];
            let header_name = if let Some(colon) = header.find(':') {
                &header[..colon]
            } else {
                header.as_str()
            };

            if is_sensitive_header(header_name) {
                omitted = true;
                let has_value = header.contains(':') && !header[header.find(':').unwrap() + 1..].trim().is_empty();
                i += 2;
                if !has_value {
                    while i < parts.len() && !is_curl_flag(&parts[i]) {
                        i += 1;
                    }
                }
                continue;
            }

            // 非秘匿ヘッダー: そのまま保持
            result.push(arg.clone());
            result.push(header.clone());
            let has_value = header.contains(':') && !header[header.find(':').unwrap() + 1..].trim().is_empty();
            i += 2;
            if !has_value {
                while i < parts.len() && !is_curl_flag(&parts[i]) {
                    result.push(parts[i].clone());
                    i += 1;
                }
            }
        } else if sensitive_flags.iter().any(|f| arg == *f) && i + 1 < parts.len() {
            omitted = true;
            i += 2;
        } else if arg.starts_with("http://") || arg.starts_with("https://") {
            result.push(redact_url_params(arg));
            i += 1;
        } else {
            result.push(arg.clone());
            i += 1;
        }
    }
    let line = format!("curl {}", result.join(" "));
    if omitted {
        format!("{line}\n（※ 秘匿情報は省略）")
    } else {
        line
    }
}

fn redact_url_params(url: &str) -> String {
    let sensitive_keys = [
        "token",
        "access_token",
        "api_key",
        "apikey",
        "secret",
        "password",
        "key",
        "auth",
    ];
    if let Some(q) = url.find('?') {
        let (base, query) = url.split_at(q + 1);
        let redacted: Vec<String> = query
            .split('&')
            .map(|pair| {
                if let Some(eq) = pair.find('=') {
                    let k = &pair[..eq].to_lowercase();
                    if sensitive_keys.iter().any(|s| k.contains(s)) {
                        return format!("{}=****", &pair[..eq]);
                    }
                }
                pair.to_string()
            })
            .collect();
        format!("{}{}", base, redacted.join("&"))
    } else {
        url.to_string()
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
