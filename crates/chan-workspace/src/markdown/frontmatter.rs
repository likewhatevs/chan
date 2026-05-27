// YAML frontmatter detection and parsing.
//
//     ---
//     title: My note
//     tags: [a, b]
//     ---
//
//     # body...
//
// Returns the parsed YAML as a serde_json::Value plus the byte
// offset where the body starts. Callers can pass `&source[fm.body_offset..]`
// to skip the frontmatter when feeding the body to other parsers.

use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::{Deserialize, Serialize};

use crate::graph::NodeKind;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Parsed key/value data. `null` if the document had no frontmatter.
    pub data: serde_json::Value,
    /// Byte offset in the source where the body begins (after the
    /// closing `---\n`). 0 if no frontmatter was present.
    pub body_offset: usize,
}

pub fn parse(source: &str) -> Frontmatter {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(source);
    let data = parsed
        .data
        .and_then(|p| p.deserialize::<serde_json::Value>().ok())
        .unwrap_or(serde_json::Value::Null);
    let body_offset = if data.is_null() {
        0
    } else {
        find_body_offset(source).unwrap_or(0)
    };
    Frontmatter { data, body_offset }
}

/// One supported `chan.kind` frontmatter value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChanKindSpec {
    /// Canonical lowercase `chan.kind` value.
    pub name: &'static str,
    /// Graph node kind stamped by the indexer.
    pub node_kind: NodeKind,
    /// Renderer hint surfaced to HTTP/web consumers.
    pub renderer: &'static str,
}

/// Registry of supported `chan.kind` values. Adding a new kind should
/// be a table entry first; callers consume the returned spec rather
/// than branching directly on frontmatter strings.
pub const CHAN_KIND_REGISTRY: &[ChanKindSpec] = &[ChanKindSpec {
    name: "contact",
    node_kind: NodeKind::Contact,
    renderer: "contact",
}];

pub fn chan_kind(data: &serde_json::Value) -> Option<ChanKindSpec> {
    let raw = data
        .get("chan")
        .and_then(|v| v.get("kind"))
        .and_then(|v| v.as_str())?;
    CHAN_KIND_REGISTRY
        .iter()
        .copied()
        .find(|spec| spec.name.eq_ignore_ascii_case(raw.trim()))
}

fn find_body_offset(source: &str) -> Option<usize> {
    if !source.starts_with("---") {
        return None;
    }
    let after_opening = source.find('\n').map(|n| n + 1)?;
    let rest = &source[after_opening..];
    let mut idx = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            return Some(after_opening + idx + line.len());
        }
        idx += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter() {
        let fm = parse("# title\n\nbody\n");
        assert!(fm.data.is_null());
        assert_eq!(fm.body_offset, 0);
    }

    #[test]
    fn with_frontmatter() {
        let src = "---\ntitle: hello\n---\n\nbody here\n";
        let fm = parse(src);
        assert_eq!(fm.data["title"], "hello");
        assert!(fm.body_offset > 0);
        assert!(
            src[fm.body_offset..].starts_with('\n') || src[fm.body_offset..].starts_with("body")
        );
    }

    #[test]
    fn array_value() {
        let src = "---\ntags:\n  - a\n  - b\n---\nx\n";
        let fm = parse(src);
        assert_eq!(fm.data["tags"][0], "a");
        assert_eq!(fm.data["tags"][1], "b");
    }

    #[test]
    fn chan_kind_registry_resolves_contact_case_insensitively() {
        let src = "---\nchan:\n  kind: Contact\n---\n# Alice\n";
        let fm = parse(src);
        let spec = chan_kind(&fm.data).expect("contact kind");
        assert_eq!(spec.name, "contact");
        assert_eq!(spec.node_kind, NodeKind::Contact);
        assert_eq!(spec.renderer, "contact");
    }

    #[test]
    fn chan_kind_registry_ignores_unknown_kinds() {
        let src = "---\nchan:\n  kind: task\n---\n# Todo\n";
        let fm = parse(src);
        assert!(chan_kind(&fm.data).is_none());
    }
}
