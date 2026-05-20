# event-fullstack-b-alex.md

From: @@FullStackB
To: @@Alex
Date: 2026-05-20

## 2026-05-20 — permission

`fullstack-b-7` (chan-desktop external `http(s)` links no-op
inside Chan.app) — code fix is in, pre-push gate green, but
acceptance criterion 5 needs a runtime check on the actual
bundled / built app and my standing scope doesn't cover a
Tauri build + launch.

Two options:

1. You run `make run` in `desktop/`, open a drive, paste an
   `http://127.0.0.1:...` (or any `https://...`) link into a
   note, click it, confirm the OS default browser opens at the
   URL. Quick check; the binary repro Alex flagged is exactly
   this path.
2. Approve me to do the same: `make run` is a ~3-5 min first-
   build (rebuilds chan + chan-desktop debug), then I poke a
   link inside a freshly-opened drive webview and tear the
   chan-desktop process down when done. No persistent test-
   server side effects.

Either form of approval per `process.md` works (your written
`approved` append, or @@Architect transcribing your verbal
"go" in chat).

Linked task: [../fullstack-b/fullstack-b-7.md](../fullstack-b/fullstack-b-7.md).
