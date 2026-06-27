// Per-surface launcher capabilities, read once at boot from the index <meta>
// tags the server injects.
//
// The tunnel-trust devserver/gateway surface sets
// `<meta name="chan-launcher-readonly">`: workspace mutation (add / on-off /
// remove) is gated out there — a grantee can't be told from the owner, so the
// server answers those calls 403 — and the SPA hides the mutation controls and
// shows a "manage elsewhere" hint instead of buttons that fail. The desktop
// loopback omits the tag and gets the full mutable surface.
export const readOnly =
  typeof document !== "undefined" &&
  document.querySelector('meta[name="chan-launcher-readonly"]') !== null;

// The launcher host's OS family, injected by the server as a <meta> tag so the
// LOCAL machine card shows the same OS icon as a remote devserver does. One of
// `macos | windows | linux | other`, or "" when the tag is absent (no icon).
export const hostOs =
  typeof document === "undefined"
    ? ""
    : (document
        .querySelector('meta[name="chan-launcher-host-os"]')
        ?.getAttribute("content") ?? "");
