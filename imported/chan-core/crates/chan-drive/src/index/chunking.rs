// Markdown -> Chunks. Three strategies (Headings, WholeDoc, Fixed)
// selected via `IndexConfig::chunking`.
//
// Headings is the default: it gives semantically coherent units,
// lines up with the editor inspector's outline view, and produces
// stable chunk ids the frontend can reuse for scrolling.
//
// All strategies share the `Chunk` shape so downstream BM25 +
// embeddings code is strategy-agnostic.

use crate::markdown::headings::{self, Heading};
use crate::markdown::parse_frontmatter;

use super::config::Chunking;

/// One indexable unit. The id is stable within a (file, strategy)
/// pair: re-chunking a file produces the same ids if the text didn't
/// change in a way that affects boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Stable id within the file. Format depends on strategy:
    ///   Headings: `h-<index>` (matches the frontend Outline pane)
    ///   WholeDoc: `whole`
    ///   Fixed:    `c-<index>`
    pub id: String,
    /// Heading text for the section this chunk belongs to. Empty
    /// when no heading applies (whole-doc / pre-first-heading
    /// content).
    pub heading: String,
    /// Heading depth (1..=6), or 0 if no heading applies.
    pub depth: u8,
    /// 0-indexed start line in the original source.
    pub start_line: usize,
    /// 0-indexed (exclusive) end line.
    pub end_line: usize,
    /// The chunk body, trimmed of leading/trailing blank lines.
    pub body: String,
}

/// Split a markdown document into chunks per the active strategy.
///
/// YAML frontmatter is stripped before chunking so the indexer never
/// tokenizes classifier metadata (`chan.kind: contact`, `provider:
/// google`, `imported_at: ...`). Without this, a search for `google`
/// would surface every imported contact note via the prelude chunk.
/// Chunk line numbers stay relative to the original source: we add
/// the frontmatter's line count back after running the strategy on
/// the body.
pub fn chunk(source: &str, strategy: &Chunking) -> Vec<Chunk> {
    let fm = parse_frontmatter(source);
    let body = &source[fm.body_offset..];
    let line_offset = source[..fm.body_offset].matches('\n').count();
    let mut chunks = match strategy {
        Chunking::Headings => chunk_by_headings(body),
        Chunking::WholeDoc => chunk_whole(body),
        Chunking::Fixed { chars } => chunk_fixed(body, *chars),
    };
    if line_offset > 0 {
        for c in &mut chunks {
            c.start_line += line_offset;
            c.end_line += line_offset;
        }
    }
    chunks
}

fn chunk_whole(source: &str) -> Vec<Chunk> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let lines = source.matches('\n').count() + 1;
    vec![Chunk {
        id: "whole".to_owned(),
        heading: String::new(),
        depth: 0,
        start_line: 0,
        end_line: lines,
        body: trimmed.to_owned(),
    }]
}

fn chunk_by_headings(source: &str) -> Vec<Chunk> {
    let headings = headings::parse(source);
    if headings.is_empty() {
        return chunk_whole(source);
    }
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out = Vec::with_capacity(headings.len() + 1);

    // Pre-heading prelude (text before the first heading). Common
    // pattern: top-of-file YAML frontmatter or a stray paragraph.
    let first_h_line = headings[0].line as usize;
    if first_h_line > 0 {
        let body = lines[..first_h_line].join("\n");
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            out.push(Chunk {
                id: "prelude".to_owned(),
                heading: String::new(),
                depth: 0,
                start_line: 0,
                end_line: first_h_line,
                body: trimmed.to_owned(),
            });
        }
    }

    for (i, h) in headings.iter().enumerate() {
        let start = h.line as usize;
        let end = headings
            .get(i + 1)
            .map(|n| n.line as usize)
            .unwrap_or(lines.len());
        let body = lines[start..end].join("\n");
        let trimmed = body.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(Chunk {
            id: chunk_id_for_heading(h),
            heading: h.text.clone(),
            depth: h.level,
            start_line: start,
            end_line: end,
            body: trimmed.to_owned(),
        });
    }
    out
}

/// `h-<index>` matches the data-heading-id attribute the WYSIWYG
/// editor stamps on rendered headings, so a search hit can ask the
/// editor to scroll to the same chunk.
fn chunk_id_for_heading(h: &Heading) -> String {
    format!("h-{}", h.ord)
}

fn chunk_fixed(source: &str, chars: usize) -> Vec<Chunk> {
    let chars = chars.max(1);
    if source.trim().is_empty() {
        return Vec::new();
    }
    // Pre-compute byte offsets of every char boundary so chunk windows
    // never split mid-utf8 and we can compute (start_line, end_line)
    // accurately by scanning newlines on either side.
    let boundaries: Vec<usize> = source
        .char_indices()
        .map(|(b, _)| b)
        .chain(std::iter::once(source.len()))
        .collect();
    let total_chars = boundaries.len() - 1;
    let mut out = Vec::new();
    let mut chunk_idx = 0usize;
    let mut chr = 0usize;
    while chr < total_chars {
        let end_chr = (chr + chars).min(total_chars);
        let start_byte = boundaries[chr];
        let end_byte = boundaries[end_chr];
        let body = &source[start_byte..end_byte];
        let trimmed_body = body.trim();
        if !trimmed_body.is_empty() {
            let start_line = source[..start_byte].matches('\n').count();
            let end_line = source[..end_byte].matches('\n').count() + 1;
            out.push(Chunk {
                id: format!("c-{chunk_idx}"),
                heading: String::new(),
                depth: 0,
                start_line,
                end_line,
                body: trimmed_body.to_owned(),
            });
            chunk_idx += 1;
        }
        chr = end_chr;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_doc_one_chunk() {
        let c = chunk("hello world\n", &Chunking::WholeDoc);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].id, "whole");
        assert_eq!(c[0].body, "hello world");
    }

    #[test]
    fn whole_doc_empty_returns_empty() {
        assert!(chunk("\n\n  \n", &Chunking::WholeDoc).is_empty());
    }

    #[test]
    fn headings_split() {
        let src = "# top\n\nintro line\n\n## sub\nbody of sub\n";
        let c = chunk(src, &Chunking::Headings);
        assert_eq!(c.len(), 2, "expected one chunk per heading: {c:?}");
        assert_eq!(c[0].id, "h-0");
        assert_eq!(c[0].heading, "top");
        assert!(c[0].body.starts_with("# top"));
        assert!(c[0].body.contains("intro line"));
        assert_eq!(c[1].id, "h-1");
        assert_eq!(c[1].heading, "sub");
        assert!(c[1].body.contains("body of sub"));
    }

    #[test]
    fn headings_with_prelude() {
        let src = "frontmatter or whatever\n\n# title\nbody\n";
        let c = chunk(src, &Chunking::Headings);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].id, "prelude");
        assert!(c[0].body.contains("frontmatter"));
        assert_eq!(c[1].id, "h-0");
    }

    #[test]
    fn headings_empty_falls_back_to_whole() {
        let c = chunk("just a paragraph\n", &Chunking::Headings);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].id, "whole");
    }

    #[test]
    fn fixed_splits_uniformly() {
        let src = "0123456789".repeat(10);
        let c = chunk(&src, &Chunking::Fixed { chars: 30 });
        assert_eq!(c.len(), 4);
        for (i, ch) in c.iter().enumerate() {
            assert_eq!(ch.id, format!("c-{i}"));
        }
    }

    #[test]
    fn fixed_empty_returns_empty() {
        assert!(chunk("", &Chunking::Fixed { chars: 10 }).is_empty());
    }

    #[test]
    fn fixed_handles_utf8_boundaries() {
        let src = "αβγδαβγδαβγδ";
        let c = chunk(src, &Chunking::Fixed { chars: 4 });
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn frontmatter_is_stripped_before_chunking() {
        // Without the strip, the YAML lines would land in a `prelude`
        // chunk and a search for "google" would match every contact.
        let src = "---\nchan:\n  kind: contact\n  provider: google\n---\n# Jane Doe\n\nbody\n";
        let c = chunk(src, &Chunking::Headings);
        assert_eq!(c.len(), 1, "no prelude chunk for the YAML: {c:?}");
        assert_eq!(c[0].id, "h-0");
        assert!(!c[0].body.contains("google"));
        assert!(!c[0].body.contains("kind: contact"));
    }

    #[test]
    fn frontmatter_strip_preserves_original_line_numbers() {
        // start_line must remain relative to the on-disk file so the
        // editor can scroll search hits back to the right line.
        let src = "---\na: 1\nb: 2\n---\n# title\nbody\n";
        let c = chunk(src, &Chunking::Headings);
        assert_eq!(c.len(), 1);
        // `# title` is on line 4 (0-indexed) in the original source.
        assert_eq!(c[0].start_line, 4);
    }
}
