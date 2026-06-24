pub(super) fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

pub(super) fn visible_activity_bytes(bytes: &[u8]) -> u64 {
    let mut visible = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x1b => i = skip_ansi_escape(bytes, i + 1),
            0x00..=0x1f | 0x7f => i += 1,
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                visible += 1;
                i += 1;
            }
        }
    }
    visible
}

fn skip_ansi_escape(bytes: &[u8], mut i: usize) -> usize {
    if i >= bytes.len() {
        return i;
    }
    match bytes[i] {
        b'[' => {
            i += 1;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7e).contains(&b) {
                    break;
                }
            }
            i
        }
        b']' => {
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    0x07 => return i + 1,
                    0x1b if i + 1 < bytes.len() && bytes[i + 1] == b'\\' => return i + 2,
                    _ => i += 1,
                }
            }
            i
        }
        _ => i + 1,
    }
}
