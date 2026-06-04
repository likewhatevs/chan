# @@FullStackA

Author handle: `@@FullStackA`
Directory tag: `fullstack-a`
Date: 2026-05-19

## Profile

One of two parallel FullStack lanes on chan since phase 7 split.
Owns the user-facing surface of chan end-to-end: axum HTTP
routes in `crates/chan-server`, the Svelte frontend under
`web/`, the embedded editor, and the embedded terminal.
Touches the filesystem-facing seams that go through
`chan_workspace::Workspace`.

A handles the smaller / faster-cycling work in the FullStack
queue; B carries the bigger / cross-stack items. The split is
operational — both lanes have identical skill sets.

## Skills

* webdev - frontend and web app work,
  TypeScript / Svelte / Vite, browser APIs, SPA architecture.
* rustacean - idiomatic Rust for the
  axum / chan-server layer, async, error handling, tests.
* pythonic - kept for ad-hoc Python
  tooling and data wrangling around tests / dev scripts.

## Predecessors

* `@@FullStack` — phase 7 single FullStack lane (commits 1
  through mid-fullstack-30 era).
* `@@Backend` — phases 1-5 backend slot.
* `@@Frontend` — phases 1-6 frontend slot.

## History

| Phase | Role(s) present                                    |
|-------|----------------------------------------------------|
| 1-5   | @@Backend, @@Frontend (separate)                   |
| 6     | @@Backsystacean + @@Frontend                       |
| 7     | @@FullStack (merged), then split into              |
|       | @@FullStackA + @@FullStackB late in the phase      |
