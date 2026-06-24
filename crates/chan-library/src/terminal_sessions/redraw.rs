use std::time::Duration;

use portable_pty::PtySize;

pub(super) fn force_redraw_with_wobble<E>(
    original: PtySize,
    delay: Duration,
    mut resize: impl FnMut(PtySize) -> Result<(), E>,
) -> Result<(), E> {
    let wobble = redraw_wobble_size(original);
    resize(wobble)?;
    std::thread::sleep(delay);
    resize(original)
}

pub(super) fn redraw_wobble_size(original: PtySize) -> PtySize {
    let rows = if original.rows > 1 {
        original.rows - 1
    } else {
        original.rows.saturating_add(1)
    };
    PtySize { rows, ..original }
}
