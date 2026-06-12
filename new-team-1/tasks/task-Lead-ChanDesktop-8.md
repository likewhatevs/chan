# task-Lead-ChanDesktop-8 — HOT: integrated gate RED on 79de0e95

From: @@Lead. To: @@ChanDesktop. Priority over everything on your
plate (your other work is all accepted — task-3 half, task-7 fixes
7da761de verified — this is the only open item).

## The failure

My isolated full gate (`make pre-push` at 79de0e95, fresh worktree,
fresh target dir) fails:

```
error: use of deprecated static `objc2_app_kit::NSFilenamesPboardType`:
  Create multiple pasteboard items with NSPasteboardTypeFileURL or
  kUTTypeFileURL instead
  (x2 → could not compile `chan-desktop` bin + test)
```

The pre-push gate builds with `RUSTFLAGS="-D warnings"` (and clippy
`-- -D warnings`), so the deprecation in dropped_paths.rs is a hard
error there. Your bare `cargo clippy -p chan-desktop` doesn't
elevate it — that's how your gate read green while the real gate is
red. Likely you saw no warning at all due to incremental-cache
replay; either way the flags differ.

## The fix (your call between)

a) Keep the wry-parity read, add `#[allow(deprecated)]` ON the
   specific use with a comment carrying the justification you
   already wrote (parity with wry's own collect_paths so we parse
   exactly what the native layer would report — wry itself still
   reads this type); or
b) Move to the non-deprecated NSPasteboardTypeFileURL enumeration
   (per-item readObjects path) if it's contract-identical (raw
   absolute paths, pasteboard order).

(a) is smaller and keeps the parity argument literal; (b) is
future-proof. Either way the contract and pins must hold unchanged.

## Gate discipline (new, also going in the round plan)

Re-verify with the REAL flags before reporting:

```
RUSTFLAGS="-D warnings" cargo clippy -p chan-desktop --all-targets -- -D warnings
RUSTFLAGS="-D warnings" cargo test -p chan-desktop
```

Then poke me — I re-run the full isolated gate on the new HEAD.

Completion: append to task-ChanDesktop-Lead-3.md or a new file +
poke.
