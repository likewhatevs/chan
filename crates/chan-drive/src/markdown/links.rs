// Link extraction. Two link kinds:
//   - Standard markdown: `[label](target)`
//   - Wiki-style:        `[[target]]` and `[[target|label]]`
//
// Resolving the target to an actual file is the caller's concern.
// Relative paths are resolved against the source file's directory;
// absolute URLs are filtered with `is_internal()`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

/// Byte ranges in the source where wiki-link syntax should be
/// ignored: code blocks (fenced + indented), inline code, raw HTML.
/// Wiki links inside these would render as literal text in any
/// markdown viewer, so storing them as graph edges is a phantom.
fn skip_ranges(markdown: &str) -> Vec<std::ops::Range<usize>> {
    let parser = Parser::new_ext(markdown, Options::all()).into_offset_iter();
    let mut out = Vec::new();
    let mut code_depth = 0usize;
    let mut code_start: Option<usize> = None;
    for (event, range) in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                if code_depth == 0 {
                    code_start = Some(range.start);
                }
                code_depth += 1;
            }
            Event::End(TagEnd::CodeBlock) => {
                code_depth = code_depth.saturating_sub(1);
                if code_depth == 0 {
                    if let Some(s) = code_start.take() {
                        out.push(s..range.end);
                    }
                }
            }
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => {
                out.push(range);
            }
            _ => {}
        }
    }
    out
}

fn in_skip(ranges: &[std::ops::Range<usize>], pos: usize) -> bool {
    ranges.iter().any(|r| r.start <= pos && pos < r.end)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// Target as written. Not resolved against any base path.
    pub target: String,
    /// Visible label, if any.
    pub label: Option<String>,
    /// True for `[[...]]`, false for standard markdown.
    pub wiki: bool,
}

impl Link {
    /// True for links that point inside the drive (relative paths,
    /// fragments, query strings). Filters `http://`, `mailto:`, etc.
    pub fn is_internal(&self) -> bool {
        let bytes = self.target.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            match b {
                b'/' | b'#' | b'?' => return true,
                b':' if i > 0 => return false,
                _ => continue,
            }
        }
        true
    }
}

/// Extract all links in document order. Wiki links first, then
/// standard markdown links; duplicates are not deduplicated.
pub fn extract_links(markdown: &str) -> Vec<Link> {
    let mut out = wiki_links(markdown);
    out.extend(standard_links(markdown));
    out
}

fn standard_links(markdown: &str) -> Vec<Link> {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut links = Vec::new();
    let mut current_label = String::new();
    let mut in_link: Option<String> = None;

    for event in parser {
        match event {
            // `[label](dest)` and `![alt](src)` both contribute an
            // edge from the source file to the target. Image embeds
            // were previously skipped, so an inspector view of an
            // image showed a misleading "0 backlinks" even when the
            // image was embedded in N markdown files. Treating both
            // events as the same link kind matches the wiki-side
            // behavior (`![[img]]` already produced an edge because
            // the wiki scanner ignores the leading `!`).
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => {
                in_link = Some(dest_url.into_string());
                current_label.clear();
            }
            Event::Text(t) if in_link.is_some() => current_label.push_str(&t),
            Event::Code(t) if in_link.is_some() => current_label.push_str(&t),
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => {
                if let Some(target) = in_link.take() {
                    // Empty-target links/images (`[label]()`,
                    // `![alt]()`) carry no graph signal; downstream
                    // we'd build a ghost node with an empty id and
                    // Cytoscape rejects those at render time.
                    // Matches the wiki_links scanner which already
                    // requires `!target.is_empty()`.
                    if target.is_empty() {
                        current_label.clear();
                        continue;
                    }
                    let label = if current_label.is_empty() {
                        None
                    } else {
                        Some(std::mem::take(&mut current_label))
                    };
                    links.push(Link {
                        target,
                        label,
                        wiki: false,
                    });
                }
            }
            _ => {}
        }
    }
    links
}

/// Hand-rolled scanner for `[[...]]` because pulldown-cmark doesn't
/// know about wiki links. We don't nest, and a newline inside the
/// brackets aborts the match. Skips matches that fall inside ranges
/// pulldown-cmark identified as code or raw HTML, where the syntax
/// would render literally and storing it as a graph edge is a
/// phantom.
fn wiki_links(markdown: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let skips = skip_ranges(markdown);
    let bytes = markdown.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if in_skip(&skips, i) {
                i += 1;
                continue;
            }
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() {
                if bytes[j] == b']' && bytes[j + 1] == b']' {
                    break;
                }
                if bytes[j] == b'\n' {
                    j = bytes.len();
                    break;
                }
                j += 1;
            }
            if j + 1 < bytes.len() && bytes[j] == b']' && bytes[j + 1] == b']' {
                let inner = &markdown[start..j];
                let (target, label) = match inner.split_once('|') {
                    Some((t, l)) => (t.trim().to_string(), Some(l.trim().to_string())),
                    None => (inner.trim().to_string(), None),
                };
                if !target.is_empty() {
                    out.push(Link {
                        target,
                        label,
                        wiki: true,
                    });
                }
                i = j + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_link_extracted() {
        let links = extract_links("see [home](./index.md) for more");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "./index.md");
        assert_eq!(links[0].label.as_deref(), Some("home"));
        assert!(!links[0].wiki);
    }

    #[test]
    fn standard_image_embed_extracted() {
        // `![alt](src)` is a Tag::Image in pulldown-cmark and used to
        // be silently dropped, leaving images with empty backlinks
        // in the inspector. Treat it as a link so the graph picks up
        // the image as an edge target.
        let links = extract_links("see ![cat](images/cat.jpg) please");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "images/cat.jpg");
        assert_eq!(links[0].label.as_deref(), Some("cat"));
        assert!(!links[0].wiki);
    }

    #[test]
    fn standard_link_and_image_both_collected() {
        let links = extract_links("[home](./i.md) and ![pic](./p.jpg)");
        assert_eq!(links.len(), 2);
        let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
        assert_eq!(targets, vec!["./i.md", "./p.jpg"]);
    }

    #[test]
    fn wiki_link_extracted() {
        let links = extract_links("see [[index]] now");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "index");
        assert!(links[0].wiki);
        assert!(links[0].label.is_none());
    }

    #[test]
    fn wiki_link_with_label() {
        let links = extract_links("ref [[notes/x|exhibit X]]");
        assert_eq!(links[0].target, "notes/x");
        assert_eq!(links[0].label.as_deref(), Some("exhibit X"));
    }

    #[test]
    fn wiki_link_no_newline_span() {
        let links = extract_links("[[bad\nlink]]");
        assert!(links.is_empty());
    }

    #[test]
    fn external_filtered() {
        let l = Link {
            target: "https://example.com".into(),
            label: None,
            wiki: false,
        };
        assert!(!l.is_internal());
    }

    #[test]
    fn relative_kept() {
        let l = Link {
            target: "./a.md".into(),
            label: None,
            wiki: false,
        };
        assert!(l.is_internal());
    }

    #[test]
    fn fragment_only_kept() {
        let l = Link {
            target: "#section".into(),
            label: None,
            wiki: false,
        };
        assert!(l.is_internal());
    }

    #[test]
    fn mixed_document() {
        let md = "Hello [a](./a.md) and [[b]] and <https://x.com>";
        let links = extract_links(md);
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn wiki_link_in_fenced_code_is_skipped() {
        let md = "before\n\n```\n[[ignored]]\n```\n\nafter [[real]] tail";
        let links = extract_links(md);
        let wiki: Vec<&Link> = links.iter().filter(|l| l.wiki).collect();
        assert_eq!(wiki.len(), 1, "got: {wiki:?}");
        assert_eq!(wiki[0].target, "real");
    }

    #[test]
    fn wiki_link_in_indented_code_is_skipped() {
        let md = "para\n\n    [[ignored]]\n\nafter";
        let wiki: Vec<Link> = extract_links(md).into_iter().filter(|l| l.wiki).collect();
        assert!(wiki.is_empty(), "got: {wiki:?}");
    }

    #[test]
    fn wiki_link_in_inline_code_is_skipped() {
        let md = "use `[[example]]` and write [[real]]";
        let wiki: Vec<Link> = extract_links(md).into_iter().filter(|l| l.wiki).collect();
        assert_eq!(wiki.len(), 1);
        assert_eq!(wiki[0].target, "real");
    }

    #[test]
    fn wiki_link_in_raw_html_block_is_skipped() {
        let md = "<div>\n[[ignored]]\n</div>\n\nafter [[real]]";
        let wiki: Vec<Link> = extract_links(md).into_iter().filter(|l| l.wiki).collect();
        assert_eq!(wiki.len(), 1);
        assert_eq!(wiki[0].target, "real");
    }
}
