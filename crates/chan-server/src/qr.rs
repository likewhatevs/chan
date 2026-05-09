//! Terminal QR rendering for the launch banner.
//!
//! The CLI prints the launch URL on stderr and follows it with a
//! Dense1x2 (half-block) QR so a phone camera can pick the URL up
//! straight off the terminal. ECC level M (~15% damage budget) keeps
//! the matrix as small as possible; nothing on top of it needs more.

use qrcode::{Color, EcLevel, QrCode};

/// Quiet-zone width in modules. The QR spec requires 4; phone
/// scanners are forgiving below that, but we have the cells.
const QUIET: usize = 4;

const RESET: &str = "\x1b[0m";
// 256-color palette: 255 (near-white) and 232 (near-black) avoid
// clashes with terminal themes that remap the basic 16-color slots
// and let the QR scan regardless of light/dark theme.
const FG_DARK_BG_LIGHT: &str = "\x1b[48;5;255m\x1b[38;5;232m";
const FG_LIGHT_BG_DARK: &str = "\x1b[48;5;232m\x1b[38;5;255m";

/// Render `url` as a half-block QR.
/// Returns `None` if the URL is too long to encode at ECC M.
pub fn render(url: &str) -> Option<String> {
    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::M).ok()?;
    let w = code.width();
    let colors = code.to_colors();
    let dark_at = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || (x as usize) >= w || (y as usize) >= w {
            return false;
        }
        colors[(y as usize) * w + (x as usize)] == Color::Dark
    };

    let total = w + QUIET * 2;
    let rows = total.div_ceil(2);

    let mut out = String::new();
    for cy in 0..rows {
        for cx in 0..total {
            // Each cell carries two vertically stacked modules. The
            // upper-half block '\u{2580}' paints its FG on the upper
            // module and the BG on the lower, so we map (upper, lower)
            // straight onto FG/BG colors.
            let mx = cx as i32 - QUIET as i32;
            let upper_y = (cy * 2) as i32 - QUIET as i32;
            let lower_y = upper_y + 1;
            let upper = dark_at(mx, upper_y);
            let lower = dark_at(mx, lower_y);
            match (upper, lower) {
                (false, false) => {
                    out.push_str(FG_DARK_BG_LIGHT);
                    out.push(' ');
                }
                (true, true) => {
                    out.push_str(FG_LIGHT_BG_DARK);
                    out.push(' ');
                }
                (true, false) => {
                    out.push_str(FG_DARK_BG_LIGHT);
                    out.push('\u{2580}');
                }
                (false, true) => {
                    out.push_str(FG_LIGHT_BG_DARK);
                    out.push('\u{2580}');
                }
            }
        }
        out.push_str(RESET);
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_short_url() {
        let s = render("http://127.0.0.1:8080/?t=abcdef").expect("encodes");
        assert!(s.contains('\u{2580}') || s.contains(' '));
        assert!(s.ends_with('\n'));
    }
}
