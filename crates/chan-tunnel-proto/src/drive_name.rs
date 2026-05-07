//! Drive-name validation and sanitization.
//!
//! Drive names appear in public URLs as `/{user}/{drive}/...`, so
//! they have to be URL-safe. The constraints are intentionally
//! tight to keep paths predictable and to leave room for future
//! routing rules: lowercase ASCII letters, digits, and ASCII
//! hyphens; length 1..=32; cannot start or end with a hyphen.
//!
//! The `chan` CLI derives an initial drive name from the directory
//! basename; that derivation should call `sanitize_drive_name`
//! before saving so the user never holds a name that the tunnel
//! server would reject. The tunnel client and server both call
//! `is_valid_drive_name` on the wire as a defense-in-depth check.

/// Maximum drive-name length (inclusive). Picked to leave headroom
/// for the rest of a typical path; bump deliberately if needed.
pub const MAX_DRIVE_NAME_LEN: usize = 32;

/// Returns true if `s` is a syntactically valid drive name.
///
/// Rules:
/// - 1..=32 ASCII bytes
/// - characters are `[a-z0-9-]`
/// - first and last character are alphanumeric (no leading/trailing
///   hyphen)
pub fn is_valid_drive_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_DRIVE_NAME_LEN {
        return false;
    }
    let valid = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    let alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    if !alnum(bytes[0]) || !alnum(bytes[bytes.len() - 1]) {
        return false;
    }
    bytes.iter().all(|&b| valid(b))
}

/// Best-effort transform of an arbitrary string into a valid drive
/// name:
/// - lowercases ASCII letters
/// - replaces every other byte with `-`
/// - collapses runs of `-`
/// - trims leading/trailing `-`
/// - truncates to `MAX_DRIVE_NAME_LEN`
///
/// Returns `None` when the result would be empty (e.g. input was
/// all whitespace or punctuation). Callers should propagate that
/// as a "please provide a name explicitly" error rather than
/// silently inventing one.
pub fn sanitize_drive_name(input: &str) -> Option<String> {
    let mut out = String::with_capacity(input.len());
    let mut last_was_dash = true;
    for ch in input.chars() {
        let b = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if b == '-' {
            if last_was_dash {
                continue;
            }
            last_was_dash = true;
        } else {
            last_was_dash = false;
        }
        out.push(b);
        if out.len() >= MAX_DRIVE_NAME_LEN {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_canonical_examples() {
        assert!(is_valid_drive_name("notes"));
        assert!(is_valid_drive_name("a"));
        assert!(is_valid_drive_name("d-1"));
        assert!(is_valid_drive_name("123"));
        assert!(is_valid_drive_name(&"a".repeat(MAX_DRIVE_NAME_LEN)));
    }

    #[test]
    fn rejects_invalid_examples() {
        assert!(!is_valid_drive_name(""));
        assert!(!is_valid_drive_name("-leading"));
        assert!(!is_valid_drive_name("trailing-"));
        assert!(!is_valid_drive_name("UpperCase"));
        assert!(!is_valid_drive_name("with space"));
        assert!(!is_valid_drive_name("punct!"));
        assert!(!is_valid_drive_name(&"a".repeat(MAX_DRIVE_NAME_LEN + 1)));
    }

    #[test]
    fn sanitize_typical_inputs() {
        assert_eq!(sanitize_drive_name("My Notes"), Some("my-notes".into()));
        assert_eq!(
            sanitize_drive_name("  Daily Journal  "),
            Some("daily-journal".into())
        );
        assert_eq!(
            sanitize_drive_name("notes/2026-Q2"),
            Some("notes-2026-q2".into())
        );
        assert_eq!(sanitize_drive_name("---"), None);
        assert_eq!(sanitize_drive_name(""), None);
        let long = "x".repeat(100);
        let sanitized = sanitize_drive_name(&long).unwrap();
        assert!(sanitized.len() <= MAX_DRIVE_NAME_LEN);
    }

    #[test]
    fn sanitize_output_is_always_valid() {
        for s in [
            "Hello, World!",
            "résumé",
            "drive_name",
            "100%",
            "____",
            "a-b-c",
        ] {
            if let Some(n) = sanitize_drive_name(s) {
                assert!(
                    is_valid_drive_name(&n),
                    "sanitized {s:?} -> {n:?} not valid"
                );
            }
        }
    }
}
