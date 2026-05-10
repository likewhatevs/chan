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

/// Resolve a link href to a clean drive-relative POSIX path.
///
/// `href` is the literal target as written in the markdown (or the
/// inner text of a wiki link). `source_dir` is the directory of the
/// file the href appears in, expressed as a drive-relative POSIX
/// path with no leading slash (use `""` for files at the drive
/// root).
///
/// Returns `None` for hrefs that don't address a graph-targetable
/// file: external schemes, intra-document fragments, empty hrefs,
/// and relative paths that escape the drive root.
///
/// The graph builder and the editor's click handler both call this
/// (the editor side via a hand-port in TS) so the on-disk edges and
/// the in-editor navigation agree on the same target string.
pub fn normalize_href(href: &str, source_dir: &str) -> Option<String> {
    if href.is_empty() || href.as_bytes().contains(&0) {
        return None;
    }
    // Fragment-only refs (`#anchor`) stay intra-document and don't
    // produce a graph edge.
    if href.starts_with('#') {
        return None;
    }
    // URL scheme detection: a `:` that appears before any `/`, `#`,
    // or `?` marks the href as external (`https:`, `mailto:`, etc.).
    // Mirrors `Link::is_internal` so the two stay in sync.
    for (i, b) in href.bytes().enumerate() {
        match b {
            b':' if i > 0 => return None,
            b'/' | b'#' | b'?' => break,
            _ => continue,
        }
    }
    // Strip the trailing `?query` and `#anchor` portions; the graph
    // already records anchor on its own column, so the path-only
    // view is what's resolved against the drive.
    let path_only = {
        let q = href.find('?').unwrap_or(href.len());
        let h = href.find('#').unwrap_or(href.len());
        &href[..q.min(h)]
    };
    if path_only.is_empty() {
        return None;
    }
    let combined = if let Some(rest) = path_only.strip_prefix('/') {
        rest.to_string()
    } else if source_dir.is_empty() {
        path_only.to_string()
    } else {
        format!("{}/{}", source_dir.trim_end_matches('/'), path_only)
    };
    // Lexical `.` / `..` collapse. A `..` that pops past the drive
    // root is rejected; lexical-only matches chan-drive's no-symlink
    // sandbox philosophy.
    let mut stack: Vec<&str> = Vec::new();
    for part in combined.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                stack.pop()?;
            }
            _ => stack.push(part),
        }
    }
    if stack.is_empty() {
        return None;
    }
    Some(stack.join("/"))
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

    #[test]
    fn normalize_drive_rooted_strips_leading_slash() {
        assert_eq!(normalize_href("/x.md", "notes").as_deref(), Some("x.md"));
        assert_eq!(
            normalize_href("/images/x.png", "deep/nested").as_deref(),
            Some("images/x.png"),
        );
    }

    #[test]
    fn normalize_parent_relative_walks_up() {
        assert_eq!(normalize_href("../x.md", "notes").as_deref(), Some("x.md"));
        assert_eq!(normalize_href("../../x.md", "a/b").as_deref(), Some("x.md"),);
    }

    #[test]
    fn normalize_parent_relative_escapes_drive() {
        assert!(normalize_href("../../../x.md", "a/b").is_none());
        assert!(normalize_href("../x.md", "").is_none());
    }

    #[test]
    fn normalize_dot_relative_resolves_to_source_dir() {
        assert_eq!(
            normalize_href("./x.md", "notes").as_deref(),
            Some("notes/x.md"),
        );
    }

    #[test]
    fn normalize_bare_relative_resolves_to_source_dir() {
        assert_eq!(
            normalize_href("x.md", "notes").as_deref(),
            Some("notes/x.md"),
        );
        assert_eq!(normalize_href("x.md", "").as_deref(), Some("x.md"));
    }

    #[test]
    fn normalize_external_schemes_rejected() {
        assert!(normalize_href("https://x.com/", "notes").is_none());
        assert!(normalize_href("mailto:a@b", "notes").is_none());
        assert!(normalize_href("tel:+15551234", "notes").is_none());
    }

    #[test]
    fn normalize_fragment_only_rejected() {
        assert!(normalize_href("#section", "notes").is_none());
    }

    #[test]
    fn normalize_strips_anchor_from_relative() {
        assert_eq!(
            normalize_href("a.md#sec", "notes").as_deref(),
            Some("notes/a.md"),
        );
    }

    #[test]
    fn normalize_strips_anchor_from_absolute() {
        assert_eq!(
            normalize_href("/a.md#sec", "notes").as_deref(),
            Some("a.md"),
        );
    }

    #[test]
    fn normalize_strips_query() {
        assert_eq!(
            normalize_href("a.md?q=1", "notes").as_deref(),
            Some("notes/a.md"),
        );
    }

    #[test]
    fn normalize_strips_query_and_anchor_together() {
        assert_eq!(
            normalize_href("a.md?q=1#sec", "notes").as_deref(),
            Some("notes/a.md"),
        );
        assert_eq!(
            normalize_href("a.md#sec?q=1", "notes").as_deref(),
            Some("notes/a.md"),
        );
    }

    #[test]
    fn normalize_empty_rejected() {
        assert!(normalize_href("", "notes").is_none());
    }

    #[test]
    fn normalize_root_only_rejected() {
        assert!(normalize_href("/", "notes").is_none());
        assert!(normalize_href("/#frag", "notes").is_none());
        assert!(normalize_href("/?q=1", "notes").is_none());
    }

    #[test]
    fn normalize_preserves_spaces() {
        assert_eq!(
            normalize_href("/contacts/Jane Doe.md", "notes").as_deref(),
            Some("contacts/Jane Doe.md"),
        );
    }

    #[test]
    fn normalize_collapses_interior_dot() {
        assert_eq!(
            normalize_href("a/./b.md", "notes").as_deref(),
            Some("notes/a/b.md"),
        );
    }

    #[test]
    fn normalize_collapses_interior_double_dot() {
        assert_eq!(
            normalize_href("a/b/../c.md", "notes").as_deref(),
            Some("notes/a/c.md"),
        );
    }

    #[test]
    fn normalize_rejects_null_byte() {
        assert!(normalize_href("a\0b.md", "notes").is_none());
    }
}
