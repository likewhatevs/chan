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
}
