---
name: test-server
description: Spin up and tear down a local chan test server over a throwaway or
  existing workspace, including the rebuild cycle for frontend changes.
when_to_use: The user asks to "spin up a test server", "try this in the browser",
  or otherwise wants a running chan instance to verify a change.
---

# Test Server Workflow

When the user asks for a test server (e.g. "spin up a test
server", "let's try this in the browser"):

1. **Ask first**: new workspace under `/tmp/chan-test-<something>`,
   or reuse an existing registered one? `chan list` shows the
   options. For a new workspace, also ask what to seed it with
   (empty, a few sample notes, copy of an existing tree).
2. **Build + launch**: `cargo build -p chan` rebuilds the binary
   with the current `web/dist/` bundle, then
   `./target/debug/chan serve <path>` in the background. The URL
   with the per-launch bearer token lands on stderr.
3. **Reload on frontend changes**: rust-embed bakes the bundle
   in at compile time, so every web edit needs the full cycle:
   stop the server, `npm run build` in `web/`, `cargo build -p
   chan`, restart. There is no hot reload. A stale browser tab
   also needs a hard reload to pick up the new hashed bundle
   filenames.
4. **Tear down**: stop the server process, `rm -rf` the temp
   workspace directory if it was a throwaway, then `chan remove
   <path>` to drop the registry entry. `chan remove` takes the
   path, not the display name.

## Pitfalls (hard-won)

- **Stale `web/dist` gives a false bug.** When QA'ing a
  frontend-touching change, run `npm run build` in `web/` BEFORE
  `cargo build`, and grep the SERVED bundle for the handler before
  calling it broken. `web/dist` is gitignored; a stale embed gives a
  false-negative, not a product bug.
- **Re-walking a previously-failed test**: explicitly stop the old
  server, `cargo build`, verify the binary provenance, then restart.
  Stale-binary false-positives cost real round-trips.
- **Multi-agent runs**: a broad `pkill chan serve` kills every
  agent's server. When several lanes share a machine, serve from a
  renamed binary copy (e.g. `/tmp/docsrv`) and scope each pkill to
  your own workspace path or port.
