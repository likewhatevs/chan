// Language-aware keyword counter.
//
// Cheap, deterministic, comparable within a language and roughly
// across closely-related languages. Not cyclomatic complexity;
// callers must treat the score as a heuristic, not a defect
// signal. Documented as such in design.md section 7.

/// Default keyword list. Mirrors scc's set. Per-language overrides
/// stay in this file; v0 uses the default for every language.
const DEFAULT_KEYWORDS: &[&str] = &[
    "if", "else", "elsif", "elif", "for", "while", "switch", "case", "match", "do", "goto",
    "continue", "break", "try", "catch", "except", "&&", "||", "and", "or",
];

fn keywords_for(_language: &str) -> &'static [&'static str] {
    DEFAULT_KEYWORDS
}

/// Score `content` for `language` by counting keyword occurrences.
/// Alphabetic keywords are matched on word boundaries; symbolic
/// operators (`&&`, `||`) are substring matches.
pub(crate) fn score(language: &str, content: &str) -> u64 {
    let keywords = keywords_for(language);
    let mut count = 0u64;
    for kw in keywords {
        if kw.chars().all(|c| c.is_ascii_alphabetic()) {
            count += count_word(content, kw);
        } else {
            count += content.matches(kw).count() as u64;
        }
    }
    count
}

fn count_word(haystack: &str, needle: &str) -> u64 {
    let bytes = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() || nb.len() > bytes.len() {
        return 0;
    }
    let mut count = 0u64;
    let mut i = 0;
    while i + nb.len() <= bytes.len() {
        if &bytes[i..i + nb.len()] == nb {
            let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
            let after_ok = i + nb.len() == bytes.len() || !is_word_byte(bytes[i + nb.len()]);
            if before_ok && after_ok {
                count += 1;
                i += nb.len();
                continue;
            }
        }
        i += 1;
    }
    count
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_branches() {
        // 2 ifs, 1 else, 1 for, 1 while, 1 &&, 1 || = 7
        let s = "if a { } else { } if b { } for x in y { } while z { } && || ";
        assert_eq!(score("Rust", s), 7);
    }

    #[test]
    fn ignores_substrings_inside_identifiers() {
        // `notify`, `forgive`, `withered` should not contribute.
        let s = "notify forgive withered";
        assert_eq!(score("Rust", s), 0);
    }

    #[test]
    fn empty_content() {
        assert_eq!(score("Rust", ""), 0);
    }
}
