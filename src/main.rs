use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        eprintln!("Usage:");
        eprintln!("  dg <text>");
        eprintln!("  dg curl [curl args...] <url>");
        std::process::exit(2);
    };

    if first == "curl" {
        let url = extract_url(args).unwrap_or_else(|| {
            eprintln!("dg curl: URL not found in args");
            std::process::exit(2);
        });

        let path = extract_path(&url).unwrap_or_else(|| {
            eprintln!("dg curl: could not parse URL: {url}");
            std::process::exit(2);
        });

        if path == "/tech_book_terms" {
            let mermaid = mermaid_tech_book_terms_index();
            let mmd_path = desktop_path("tech_book_terms.mmd").unwrap_or_else(|| {
                eprintln!("dg curl: could not resolve ~/Desktop");
                std::process::exit(2);
            });
            if let Err(e) = fs::write(&mmd_path, &mermaid) {
                eprintln!("dg curl: failed to write {}: {e}", mmd_path.display());
                std::process::exit(1);
            }

            let html_path = desktop_path("tech_book_terms.html").unwrap_or_else(|| {
                eprintln!("dg curl: could not resolve ~/Desktop");
                std::process::exit(2);
            });
            let html = mermaid_html_page("TechBookTerms#index", &mermaid);
            if let Err(e) = fs::write(&html_path, html) {
                eprintln!("dg curl: failed to write {}: {e}", html_path.display());
                std::process::exit(1);
            }

            // Open the rendered diagram in the default browser (macOS).
            let status = Command::new("open").arg(&html_path).status();
            match status {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    eprintln!("dg curl: open failed with exit code: {s}");
                }
                Err(e) => {
                    eprintln!("dg curl: failed to run open: {e}");
                }
            }

            println!("{}", html_path.display());
            return;
        }

        eprintln!("dg curl: unsupported path: {path}");
        std::process::exit(2);
    }

    println!("{first}");
}

fn extract_url(args: impl Iterator<Item = String>) -> Option<String> {
    args.filter(|a| a.starts_with("http://") || a.starts_with("https://"))
        .next()
        .map(|s| s.trim_matches('\'').trim_matches('"').to_string())
}

fn extract_path(url: &str) -> Option<String> {
    // Very small parser good enough for: http://localhost:3000/tech_book_terms?x=y
    let after_scheme = url.split("://").nth(1)?;
    let slash = after_scheme.find('/')?;
    let path_and_more = &after_scheme[slash..];
    let path = path_and_more.split('?').next().unwrap_or(path_and_more);
    Some(path.to_string())
}

fn desktop_path(filename: &str) -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    p.push("Desktop");
    p.push(filename);
    Some(p)
}

fn mermaid_tech_book_terms_index() -> String {
    // Based on tech-index/app/controllers/tech_book_terms_controller.rb#index
    // and related models.
    [
        "flowchart TD",
        "  A[Client] -->|GET /tech_book_terms| B[Rails Router]",
        "  B --> C[TechBookTermsController index]",
        "  C --> D[TechBookTerm.visible_to(current_user)]",
        "  D --> E[includes(:tech_book) order(created_at: :desc)]",
        "  E --> F{params[:q] present?}",
        "  F -->|yes| G[TechBookTerm.search(q) (joins :tech_book, searches columns + book name + related terms)]",
        "  F -->|no| H[skip search]",
        "  G --> I[page(params[:page]).per(100)]",
        "  H --> I",
        r#"  I --> J["TechBookTerm.preload_related_terms(records) (load RelatedTerm for related_term_ids)"]"#,
        r#"  C --> K["Sidebar: TechBook.order(LOWER(name)) page(params[:books_page]).per(100)"]"#,
        r#"  J --> L["Render HTML: tech_book_terms/index"]"#,
        "  K --> L",
    ]
    .join("\n")
}

fn mermaid_html_page(title: &str, mermaid: &str) -> String {
    // Follow Mermaid v11 official "simple full example":
    // put the graph definition directly inside <pre class="mermaid"> ... </pre>
    // and load mermaid as an ESM module. Avoid HTML-escaping `>` into `&gt;`.
    format!(
        r#"<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{}</title>
    <style>
      body {{ font-family: -apple-system, system-ui, sans-serif; margin: 24px; }}
      pre.mermaid {{ border: 1px solid #ddd; border-radius: 8px; padding: 16px; overflow: auto; }}
    </style>
  </head>
  <body>
    <h1>{}</h1>
    <pre class="mermaid">
{}
    </pre>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      // Default behavior renders all elements with class="mermaid" after load.
      // Explicit initialization is optional for this simple use-case.
    </script>
  </body>
</html>
"#,
        title, title, mermaid
    )
}
