// ATX heading parser. Skips fenced code blocks (``` and ~~~) so
// `# this is a comment` inside a code block doesn't pollute the
// outline. Setext headings (===, ---) are not recognized; they
// don't fit a per-line scan and are rare in personal notes.

use serde::{Deserialize, Serialize};

/// One ATX heading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    /// Position in document order, 0-indexed.
    pub ord: u32,
    /// 0-indexed line number in the source.
    pub line: u32,
    /// Heading depth (1..=6).
    pub level: u8,
    /// Display text (no leading or trailing `#`s).
    pub text: String,
}

/// Parse all ATX headings from `source`.
pub fn parse(source: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = "";
    for (line_idx, line) in source.split('\n').enumerate() {
        if let Some(marker) = leading_fence(line) {
            if !in_fence {
                in_fence = true;
                fence_marker = marker;
            } else if line.starts_with(fence_marker) {
                in_fence = false;
                fence_marker = "";
            }
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((depth, text)) = atx_heading(line) {
            out.push(Heading {
                ord: out.len() as u32,
                line: line_idx as u32,
                level: depth,
                text,
            });
        }
    }
    out
}

fn leading_fence(line: &str) -> Option<&'static str> {
    if line.starts_with("```") {
        Some("```")
    } else if line.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn atx_heading(line: &str) -> Option<(u8, String)> {
    let bytes = line.as_bytes();
    let mut depth = 0;
    while depth < bytes.len() && bytes[depth] == b'#' && depth < 6 {
        depth += 1;
    }
    if depth == 0 {
        return None;
    }
    // CommonMark: whitespace (or end of line) required after the #s.
    if depth >= bytes.len() {
        return None;
    }
    if bytes[depth] != b' ' && bytes[depth] != b'\t' {
        return None;
    }
    let rest = &line[depth + 1..];
    let trimmed = rest.trim();
    let trimmed = trimmed.trim_end_matches('#');
    let text = trimmed.trim_end().to_owned();
    if text.is_empty() {
        None
    } else {
        Some((depth as u8, text))
    }
}

/// Walk the heading stack at index `i` to produce the breadcrumb
/// chain leading to `headings[i]`. Each element is the closest
/// preceding heading at a shallower level. Useful for search
/// snippets and graph anchors.
pub fn heading_stack(headings: &[Heading], i: usize) -> Vec<&Heading> {
    let mut stack: Vec<&Heading> = Vec::new();
    for h in &headings[..=i] {
        while stack.last().is_some_and(|t| t.level >= h.level) {
            stack.pop();
        }
        stack.push(h);
    }
    stack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_h1_h2() {
        let h = parse("# top\n\nsome text\n\n## second\n");
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].level, 1);
        assert_eq!(h[0].text, "top");
        assert_eq!(h[0].line, 0);
        assert_eq!(h[1].level, 2);
        assert_eq!(h[1].text, "second");
        assert_eq!(h[1].line, 4);
    }

    #[test]
    fn skips_fenced_code() {
        let h = parse("# real\n\n```\n# not a heading\n```\n\n## also real\n");
        let texts: Vec<_> = h.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, vec!["real", "also real"]);
    }

    #[test]
    fn requires_space_after_hash() {
        let h = parse("#nope\n# yes\n");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].text, "yes");
    }

    #[test]
    fn caps_at_h6() {
        let h = parse("####### too deep\n");
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn strips_trailing_hashes() {
        let h = parse("## title ## \n");
        assert_eq!(h[0].text, "title");
    }

    #[test]
    fn ignores_setext() {
        let h = parse("Title\n=====\n");
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn nested_fences() {
        let h = parse("# real\n```\n~~~\n# masked\n~~~\n```\n## also real\n");
        let texts: Vec<_> = h.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, vec!["real", "also real"]);
    }

    #[test]
    fn stack_walks_ancestors() {
        let h = parse("# A\n## B\n### C\n## D\n# E\n");
        let stack: Vec<_> = heading_stack(&h, 2)
            .iter()
            .map(|x| x.text.as_str())
            .collect();
        assert_eq!(stack, vec!["A", "B", "C"]);
        let stack: Vec<_> = heading_stack(&h, 3)
            .iter()
            .map(|x| x.text.as_str())
            .collect();
        assert_eq!(stack, vec!["A", "D"]);
        let stack: Vec<_> = heading_stack(&h, 4)
            .iter()
            .map(|x| x.text.as_str())
            .collect();
        assert_eq!(stack, vec!["E"]);
    }
}
