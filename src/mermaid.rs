use crate::sanitize::html_escape;

pub fn extract_mermaid_block(text: &str) -> Option<String> {
    let start = text.find("```mermaid")?;
    let rest = &text[start + "```mermaid".len()..];
    let rest = rest.trim_start_matches(['\r', '\n', ' ']);
    let end = rest.find("```")?;
    let body = rest[..end].trim();
    if body.is_empty() { None } else { Some(body.to_string()) }
}

/// Mermaid ソースの先頭にある `%% filename: <slug>` 行からスラグを抽出し、
/// その行を除いた本体を返す。
pub fn extract_filename_slug(mermaid: &str) -> (Option<String>, String) {
    if let Some(first_line) = mermaid.lines().next() {
        let trimmed = first_line.trim();
        if let Some(rest) = trimmed.strip_prefix("%% filename:") {
            let slug: String = rest
                .trim()
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            let body = mermaid[first_line.len()..].trim_start_matches(['\r', '\n']).to_string();
            if slug.is_empty() {
                return (None, body);
            }
            return (Some(slug), body);
        }
    }
    (None, mermaid.to_string())
}

pub fn mermaid_html_page(
    title: &str,
    mermaid: &str,
    input_text: &str,
    input_label: &str,
    diagram_type_label: &str,
) -> String {
    let escaped_input = html_escape(input_text);
    let escaped_title = html_escape(title);
    let escaped_label = html_escape(input_label);
    let escaped_diagram_type = html_escape(diagram_type_label);
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
        <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-2">{escaped_label}</h3>
        <pre class="text-sm text-gray-800 break-all whitespace-pre-wrap">{escaped_input}</pre>
      </div>
      <div class="border-2 border-blue-300 rounded-lg p-6 bg-blue-50">
        <div class="flex items-center gap-3 mb-4">
          <h2 class="text-xl font-bold text-blue-900">{escaped_title}</h2>
          <span class="text-xs font-medium text-blue-700 bg-blue-200 rounded px-2 py-0.5">{escaped_diagram_type}</span>
        </div>
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
