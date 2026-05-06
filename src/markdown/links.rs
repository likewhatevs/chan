// Link extraction. Two link kinds:
//   - Standard markdown: `[label](target)`
//   - Wiki-style:        `[[target]]` and `[[target|label]]`
//
// Resolving the target to an actual file is the caller's concern.
// Relative paths are resolved against the source file's directory;
// absolute URLs are filtered with `is_internal()`.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

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
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = Some(dest_url.into_string());
                current_label.clear();
            }
            Event::Text(t) if in_link.is_some() => current_label.push_str(&t),
            Event::Code(t) if in_link.is_some() => current_label.push_str(&t),
            Event::End(TagEnd::Link) => {
                if let Some(target) = in_link.take() {
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
/// brackets aborts the match.
fn wiki_links(markdown: &str) -> Vec<Link> {
    let mut out = Vec::new();
    let bytes = markdown.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
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
}
