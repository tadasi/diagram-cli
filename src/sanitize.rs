pub fn redact_curl_line(parts: &[String]) -> String {
    let sensitive_headers = [
        "authorization",
        "cookie",
        "set-cookie",
        "x-csrf-token",
        "x-api-key",
        "x-access-token",
    ];
    let sensitive_flags = ["-b", "--cookie", "-u", "--user"];

    let mut result = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let arg = &parts[i];
        if (arg == "-H" || arg == "--header") && i + 1 < parts.len() {
            let header = &parts[i + 1];
            if let Some(colon) = header.find(':') {
                let name = header[..colon].trim().to_lowercase();
                if sensitive_headers.iter().any(|h| name == *h) {
                    result.push(arg.clone());
                    result.push(format!("{}: ****", &header[..colon]));
                    i += 2;
                    continue;
                }
            }
            result.push(arg.clone());
            result.push(parts[i + 1].clone());
            i += 2;
        } else if sensitive_flags.iter().any(|f| arg == *f) && i + 1 < parts.len() {
            result.push(arg.clone());
            result.push("****".to_string());
            i += 2;
        } else if arg.starts_with("http://") || arg.starts_with("https://") {
            result.push(redact_url_params(arg));
            i += 1;
        } else {
            result.push(arg.clone());
            i += 1;
        }
    }
    format!("curl {}", result.join(" "))
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
