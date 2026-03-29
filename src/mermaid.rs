use crate::sanitize::html_escape;

pub fn extract_mermaid_block(text: &str) -> Option<String> {
    let markers = ["```mermaid\n", "```mermaid\r\n", "```mermaid\r"];
    for m in markers {
        if let Some(i) = text.find(m) {
            let rest = &text[i + m.len()..];
            if let Some(end) = rest.find("```") {
                let body = rest[..end].trim();
                if !body.is_empty() {
                    return Some(body.to_string());
                }
            }
        }
    }
    if let Some(i) = text.find("```mermaid") {
        let rest = &text[i + "```mermaid".len()..];
        let rest = rest.trim_start_matches(['\r', '\n', ' ']);
        if let Some(end) = rest.find("```") {
            let body = rest[..end].trim();
            if !body.is_empty() {
                return Some(body.to_string());
            }
        }
    }
    None
}

pub fn mermaid_html_page(title: &str, mermaid: &str, curl_line: &str) -> String {
    let escaped_curl = html_escape(curl_line);
    let escaped_title = html_escape(title);
    format!(
        r###"<!DOCTYPE html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
    <script src="https://cdn.tailwindcss.com"></script>
  </head>
  <body class="min-w-[1280px] min-h-[720px] m-0 p-0 bg-white">
    <div class="w-full flex flex-col gap-6 p-10">
      <div class="border border-gray-300 rounded-lg px-5 py-4 bg-gray-50">
        <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-2">Request</h3>
        <code class="text-sm text-gray-800 break-all">{escaped_curl}</code>
      </div>
      <div class="border-2 border-blue-300 rounded-lg p-6 bg-blue-50">
        <h2 class="text-xl font-bold text-blue-900 mb-4">{escaped_title}</h2>
        <div class="mermaid">
{mermaid}
        </div>
      </div>
    </div>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({{
        startOnLoad: true,
        theme: "base",
        themeVariables: {{
          primaryColor: "#dbeafe",
          primaryBorderColor: "#3b82f6",
          lineColor: "#6b7280",
          fontFamily: "system-ui, sans-serif"
        }}
      }});
    </script>
  </body>
</html>
"###,
    )
}
