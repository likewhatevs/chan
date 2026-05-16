// Reciprocal Rank Fusion (RRF). Combines results from N rankers
// (BM25 + semantic) into a single list without per-ranker score
// normalization. Tuning-free: pick `k`, ignore everything else.
//
//   score(d) = sum_r  1 / (k + rank_r(d))
//
// where `rank_r(d)` is the 1-based rank of d in ranker r's list,
// or the doc is simply absent from that ranker's contribution.
//
// `k = 60` is the value from the original RRF paper. Lower values
// give more weight to top hits; the canonical 60 is a well-balanced
// default that doesn't punish documents that just barely missed
// either ranker's top spot.

use std::collections::HashMap;

use super::facade::Hit;

const RRF_K: f32 = 60.0;

/// Fuse `lists` (one per ranker) into a single list ranked by RRF.
/// Returns up to `limit` entries. The fused score replaces the
/// per-ranker score on each `Hit`.
///
/// Snippet selection: when a chunk appears in multiple rankers we
/// prefer the snippet that has BM25 highlight markup (`<b>...</b>`),
/// since the semantic snippet is a plain head-of-body fallback.
pub fn rrf(lists: &[Vec<Hit>], limit: usize) -> Vec<Hit> {
    if limit == 0 {
        return Vec::new();
    }
    let mut by_key: HashMap<(String, String), (f32, Hit)> = HashMap::new();
    for hits in lists {
        for (rank0, h) in hits.iter().enumerate() {
            let key = (h.path.clone(), h.chunk_id.clone());
            let contrib = 1.0 / (RRF_K + rank0 as f32 + 1.0);
            match by_key.get_mut(&key) {
                Some((score, existing)) => {
                    *score += contrib;
                    if !existing.snippet.contains("<b>") && h.snippet.contains("<b>") {
                        existing.snippet = h.snippet.clone();
                    }
                }
                None => {
                    let mut seed = h.clone();
                    seed.score = 0.0;
                    by_key.insert(key, (contrib, seed));
                }
            }
        }
    }
    let mut out: Vec<Hit> = by_key
        .into_values()
        .map(|(score, mut h)| {
            h.score = score;
            h
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(path: &str, chunk: &str, score: f32, snippet: &str) -> Hit {
        Hit {
            path: path.to_owned(),
            chunk_id: chunk.to_owned(),
            heading: String::new(),
            start_line: 0,
            snippet: snippet.to_owned(),
            score,
        }
    }

    #[test]
    fn empty_lists_return_empty() {
        let r = rrf(&[vec![], vec![]], 5);
        assert!(r.is_empty());
    }

    #[test]
    fn single_ranker_passthrough_order() {
        let bm25 = vec![
            h("a", "1", 1.0, ""),
            h("b", "1", 0.5, ""),
            h("c", "1", 0.1, ""),
        ];
        let r = rrf(&[bm25], 10);
        let paths: Vec<&str> = r.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, vec!["a", "b", "c"]);
    }

    #[test]
    fn document_in_both_lists_outranks_singletons() {
        let bm25 = vec![h("a", "1", 1.0, ""), h("b", "1", 0.5, "")];
        let sem = vec![h("x", "1", 0.9, ""), h("b", "1", 0.4, "")];
        let r = rrf(&[bm25, sem], 10);
        assert_eq!(r[0].path, "b");
        let rest: std::collections::HashSet<_> = r[1..].iter().map(|h| h.path.as_str()).collect();
        assert!(rest.contains("a"));
        assert!(rest.contains("x"));
    }

    #[test]
    fn keyed_by_path_and_chunk() {
        let bm25 = vec![h("a", "h-0", 1.0, ""), h("a", "h-1", 0.5, "")];
        let sem = vec![h("a", "h-0", 0.9, "")];
        let r = rrf(&[bm25, sem], 10);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].chunk_id, "h-0");
    }

    #[test]
    fn upgrades_to_highlighted_snippet() {
        let sem = vec![h("a", "1", 0.9, "plain text")];
        let bm25 = vec![h("a", "1", 0.5, "matched <b>token</b>")];
        let r = rrf(&[sem, bm25], 5);
        assert_eq!(r.len(), 1);
        assert!(r[0].snippet.contains("<b>token</b>"));
    }

    #[test]
    fn limit_truncates() {
        let bm25 = (0..50).map(|i| h(&format!("p{i}"), "1", 1.0, "")).collect();
        let r = rrf(&[bm25], 5);
        assert_eq!(r.len(), 5);
    }
}
