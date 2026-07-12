//! Server-rendered page shell for identity's non-SPA HTML pages (the
//! desktop consent and handoff pages). Mirrors the SPA's look -- the
//! dark token palette from `web-shared`'s `theme.css` and the card
//! layout of the sign-in view -- with inline CSS, because these pages
//! ship under CSP `default-src 'none'`: only the inline `<style>`
//! block and same-origin images (the `/chan-mark.png` logo mask) are
//! allowed to load.

use axum::http::{header, HeaderName, HeaderValue};

/// One page for [`render`]: the shell supplies the doctype, head,
/// shared styles, and the centered card; the caller supplies what
/// goes inside. Both string fields are raw markup the caller has
/// already escaped where needed.
pub struct Page<'a> {
    pub title: &'a str,
    /// Extra `<head>` markup (e.g. a meta refresh).
    pub head_extra: &'a str,
    /// Card-interior markup.
    pub body: &'a str,
}

/// The shared CSP for the server-rendered pages. `default-src 'none'`
/// blocks every external subresource; the carve-outs are the inline
/// `<style>` block, the same-origin logo mask (`img-src` governs CSS
/// `mask-image` loads), and the consent form's same-origin POST.
pub const CSP: &str = "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; \
                       form-action 'self'; frame-ancestors 'none'";

/// Security headers shared by every server-rendered page. Single
/// source so the consent and handoff pages cannot drift apart:
/// clickjacking (XFO + `frame-ancestors`), no caching (the pages
/// carry a CSRF nonce or a PAT secret), no referrer leakage, and no
/// MIME sniffing.
pub fn security_headers() -> [(HeaderName, HeaderValue); 5] {
    [
        (
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ),
        (
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CSP),
        ),
        (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        (
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ),
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
    ]
}

/// Minimal HTML escape: covers the five characters that matter for
/// attribute + text contexts. We never render unescaped user input
/// into a `<script>` or `style` block, so this list is sufficient.
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The inlined dark-theme values from the shared SPA stylesheet
/// (`web-shared`'s `theme.css`) plus the card/mark/button styles of
/// the SPA sign-in view (`profile`'s `Login.svelte`). Values are
/// duplicated here because the CSP forbids external stylesheets; when
/// the SPA palette changes, re-sync these.
const SHELL_CSS: &str = "\
html,body{height:100%;margin:0}\
body{display:flex;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;\
color:#f5f5f7;background:#1c1c1e}\
main{margin:auto;padding:1rem}\
.card{background:#232325;border:1px solid #3a3a3c;border-radius:12px;\
padding:2rem 2.25rem;width:min(360px,92vw);text-align:center;box-sizing:border-box}\
.mark{display:block;margin:0 auto .75rem;width:72px;height:72px;background-color:#f97316;\
-webkit-mask:url('/chan-mark.png') center/contain no-repeat;\
mask:url('/chan-mark.png') center/contain no-repeat}\
h1{color:#f5f5f7;margin:.25rem 0;font-size:18px;font-weight:600;letter-spacing:.01em}\
.muted{color:#98989d;font-size:14px}\
.small{font-size:13px}\
.details{text-align:left;margin:1rem 0 1.25rem;font-size:14px;border-top:1px solid #3a3a3c}\
.row{display:flex;justify-content:space-between;gap:1rem;padding:.45rem 0;\
border-bottom:1px solid #3a3a3c}\
.k{color:#98989d}\
.v{font-weight:600;word-break:break-word}\
form{display:flex;gap:.75rem}\
.btn{display:inline-flex;align-items:center;justify-content:center;flex:1;font:inherit;\
text-decoration:none;color:#f5f5f7;background:#2a2a2c;border:1px solid #3a3a3c;\
border-radius:8px;padding:.55rem 1rem;font-weight:500;cursor:pointer}\
.btn:hover{border-color:#98989d}\
.btn.primary{background:#f97316;border-color:#f97316;color:#1c1c1e}";

/// Render a page in the shared shell: dark background, centered card,
/// the SPA palette. `head_extra` and `body` are inserted verbatim.
pub fn render(page: &Page<'_>) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  {head_extra}<title>{title}</title>
  <style>{css}</style>
</head>
<body>
  <main><section class="card">{body}</section></main>
</body>
</html>
"#,
        head_extra = page.head_extra,
        title = html_escape(page.title),
        css = SHELL_CSS,
        body = page.body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_covers_attr_breakers() {
        assert_eq!(
            html_escape(r#"<a b="c" d='e'>&"#),
            "&lt;a b=&quot;c&quot; d=&#39;e&#39;&gt;&amp;"
        );
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn shell_wraps_title_head_extra_and_body() {
        let html = render(&Page {
            title: "T & T",
            head_extra: "<meta http-equiv=\"refresh\" content=\"0;url=x\">",
            body: "<h1>Hi</h1>",
        });
        assert!(html.starts_with("<!doctype html>"), "{html}");
        assert!(html.contains("<title>T &amp; T</title>"), "{html}");
        assert!(
            html.contains("<meta http-equiv=\"refresh\" content=\"0;url=x\">"),
            "{html}"
        );
        assert!(
            html.contains("<section class=\"card\"><h1>Hi</h1></section>"),
            "{html}"
        );
        assert!(html.contains(".card{background:#232325"), "{html}");
    }

    #[test]
    fn security_headers_pin_the_exact_csp() {
        assert_eq!(
            CSP,
            "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; \
             form-action 'self'; frame-ancestors 'none'"
        );
        let headers = security_headers();
        assert_eq!(headers.len(), 5);
        assert_eq!(headers[0].1, "DENY");
        assert_eq!(headers[1].1, HeaderValue::from_static(CSP));
        assert_eq!(headers[2].1, "no-store");
        assert_eq!(headers[3].1, "no-referrer");
        assert_eq!(headers[4].1, "nosniff");
    }
}
