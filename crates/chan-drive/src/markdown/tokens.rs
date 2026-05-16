// Reference-token extraction for the graph view:
//
//   #tag         folksonomy tags. ASCII alpha first; `[A-Za-z0-9_/-]*`
//   @@mention    `@@` prefix avoids collision with editor smart-node
//                `@today` / `@date` syntax that serializes to a date.
//   YYYY-MM-DD   ISO date. Surfaces dates the editor produces from
//                `@today` so the graph can group files that share one.
//
// Tokens come from text content only. Code spans, fenced code blocks,
// and HTML blocks are skipped so example syntax in documentation
// doesn't pollute the index. Link targets are also skipped (they
// come from `Tag::Link::dest_url`, not as text).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Token {
    Tag { name: String },
    Mention { name: String },
    Date { iso: String },
}

pub fn extract_tokens(markdown: &str) -> Vec<Token> {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut out = Vec::new();
    let mut in_code_block: u32 = 0;
    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block += 1,
            Event::End(TagEnd::CodeBlock) => in_code_block = in_code_block.saturating_sub(1),
            Event::Code(_) => {}
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::Text(t) if in_code_block == 0 => extract_from_text(&t, &mut out),
            _ => {}
        }
    }
    out
}

fn extract_from_text(buf: &str, out: &mut Vec<Token>) {
    let bytes = buf.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let prev = if i == 0 { None } else { Some(bytes[i - 1]) };
        let prev_is_word = prev.is_some_and(is_word_byte);
        if c == b'#' && !prev_is_word {
            if let Some(end) = scan_name(bytes, i + 1, true) {
                out.push(Token::Tag {
                    name: buf[i + 1..end].to_owned(),
                });
                i = end;
                continue;
            }
        } else if c == b'@' && i + 1 < bytes.len() && bytes[i + 1] == b'@' && !prev_is_word {
            if let Some(end) = scan_name(bytes, i + 2, false) {
                out.push(Token::Mention {
                    name: buf[i + 2..end].to_owned(),
                });
                i = end;
                continue;
            }
        } else if c.is_ascii_digit() && !prev_is_word && bytes_match_date(bytes, i) {
            // Reject if followed by an alphanumeric or '-' so
            // `2026-04-28-notes.md` doesn't capture as a date.
            let after = bytes.get(i + 10).copied();
            let after_ok = match after {
                None => true,
                Some(b) => !b.is_ascii_alphanumeric() && b != b'-',
            };
            if after_ok {
                out.push(Token::Date {
                    iso: buf[i..i + 10].to_owned(),
                });
                i += 10;
                continue;
            }
        }
        i += 1;
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'@'
}

fn scan_name(bytes: &[u8], start: usize, allow_slash: bool) -> Option<usize> {
    if start >= bytes.len() || !bytes[start].is_ascii_alphabetic() {
        return None;
    }
    let mut j = start + 1;
    while j < bytes.len() {
        let b = bytes[j];
        let ok = b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || (allow_slash && b == b'/');
        if !ok {
            break;
        }
        j += 1;
    }
    Some(j)
}

fn bytes_match_date(bytes: &[u8], i: usize) -> bool {
    if i + 10 > bytes.len() {
        return false;
    }
    let s = &bytes[i..i + 10];
    s[0].is_ascii_digit()
        && s[1].is_ascii_digit()
        && s[2].is_ascii_digit()
        && s[3].is_ascii_digit()
        && s[4] == b'-'
        && s[5].is_ascii_digit()
        && s[6].is_ascii_digit()
        && s[7] == b'-'
        && s[8].is_ascii_digit()
        && s[9].is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(md: &str) -> Vec<Token> {
        extract_tokens(md)
    }

    #[test]
    fn extracts_basic_tag() {
        assert_eq!(
            tokens("see #rust here"),
            vec![Token::Tag {
                name: "rust".into()
            }]
        );
    }

    #[test]
    fn tag_supports_nested_path() {
        assert_eq!(
            tokens("filed under #projects/chan today"),
            vec![Token::Tag {
                name: "projects/chan".into()
            }]
        );
    }

    #[test]
    fn tag_must_have_word_boundary() {
        assert!(tokens("language c#").is_empty());
    }

    #[test]
    fn extracts_mention() {
        assert_eq!(
            tokens("hi @@bob"),
            vec![Token::Mention { name: "bob".into() }]
        );
    }

    #[test]
    fn single_at_is_not_a_mention() {
        assert!(tokens("@today").is_empty());
    }

    #[test]
    fn email_is_not_a_mention() {
        assert!(tokens("write me at me@host.com").is_empty());
    }

    #[test]
    fn mention_does_not_take_slash() {
        assert_eq!(
            tokens("@@bob/v2"),
            vec![Token::Mention { name: "bob".into() }]
        );
    }

    #[test]
    fn extracts_date() {
        assert_eq!(
            tokens("met on 2026-05-01."),
            vec![Token::Date {
                iso: "2026-05-01".into()
            }]
        );
    }

    #[test]
    fn date_rejects_extension_segment() {
        assert!(tokens("see 2026-04-28-notes.md").is_empty());
    }

    #[test]
    fn skips_code_block() {
        let md = "```\n#nope @@nope 2026-01-01\n```\n";
        assert!(tokens(md).is_empty());
    }

    #[test]
    fn skips_inline_code() {
        let md = "use `#tag` syntax";
        assert!(tokens(md).is_empty());
    }

    #[test]
    fn extracts_inside_link_label() {
        let md = "see [the #rust note](./rust.md)";
        assert_eq!(
            tokens(md),
            vec![Token::Tag {
                name: "rust".into()
            }]
        );
    }

    #[test]
    fn mixed_document() {
        let md = "
# Notes for #project/chan

met @@alice on 2026-05-01 to discuss #graph-view.

```rust
// #not-a-tag
```
";
        let toks = tokens(md);
        assert_eq!(
            toks,
            vec![
                Token::Tag {
                    name: "project/chan".into()
                },
                Token::Mention {
                    name: "alice".into()
                },
                Token::Date {
                    iso: "2026-05-01".into()
                },
                Token::Tag {
                    name: "graph-view".into()
                },
            ]
        );
    }
}
