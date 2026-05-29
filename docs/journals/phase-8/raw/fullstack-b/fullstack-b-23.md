# fullstack-b-23 — Port chan.app source into web-marketing/ (Item 6 sub-piece)

Owner: @@FullStackB
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Port the existing chan.app marketing site source into
a new `web-marketing/` tree inside the chan repo, so
the website + manual + first-launch UX pipeline (Item
6 of the Round-2 backlog) can build from in-repo
sources.

## Background

Round-2 backlog Item 6 — chan.app website + manual +
first-launch UX + CI — has several sub-pieces across
lanes:

* **This task** (`-b-23`): port chan.app's existing
  source into `web-marketing/`. Static site source
  in-repo so subsequent pieces have a build target.
* `-architect-2` (future): `docs/manual/` content
  (markdown source for the manual; rendered by item 6
  website pipeline).
* `-systacean-N` (future): DNS cutover + TLS story +
  VPS decommission timeline.
* `-ci-N` (future): CI pipelines for marketing site +
  manual + release-tag manual-bundle.

Locked decisions from
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Decisions (all locked 2026-05-20)":

* Item-6 hosting: GitHub Pages with custom domain
  (apex-domain TLS via Pages' built-in cert).
* Manual home: `docs/manual/` (markdown source in main
  repo; rendered by the website pipeline at
  `chan.app/manual/`).
* No external infra dependency beyond GitHub.

Donation QR companion: per the bug-list entry "Backlog
item 6 — companion website QR" (added 2026-05-21), the
chan.app site should embed `web/public/qr-donate.png`
(or its `web-marketing/` mirror) somewhere visible —
footer Support block, inline on the download page, or a
dedicated Support page. Implementer picks at fan-out.

## Acceptance criteria

* `web-marketing/` directory exists at repo root with
  the ported chan.app source.
* `web-marketing/README.md` documents the build +
  preview commands (likely npm-based; align with
  whatever the existing chan.app source uses).
* Local preview works (`npm run dev` or equivalent
  inside `web-marketing/`).
* Donation QR placed at one of footer / download
  page / Support page (implementer picks; surface the
  choice in the task tail).
* `web-marketing/` static-site source builds into a
  publishable artifact directory (`dist/` or
  equivalent).
* `.gitignore` excludes build artifacts (`dist/`,
  `node_modules/`, etc.).
* No CI wire yet — that's a follow-up `ci-N` task
  per the Item 6 mosaic.
* No DNS cutover yet — that's a follow-up
  `systacean-N` task.
* Pre-push gate green (only relevant pieces: any test
  the new tree adds + the workspace pre-push gate
  must stay green; `web-marketing/` should be ignored
  by `cargo` since it's not a workspace member).

## How to start

1. Locate the current chan.app source (likely external
   to this repo right now; ask @@Alex via a permission
   event if you can't find it). Likely shape: a small
   static-site framework (Astro / Eleventy / vanilla)
   with a few markdown pages.
2. Port the source into `web-marketing/`. Keep the
   existing structure where reasonable; rename only if
   it conflicts with workspace conventions.
3. Add `web-marketing/README.md` with build + preview
   commands.
4. Confirm `web-marketing/` is ignored by the cargo
   workspace (root `Cargo.toml` `workspace.members`
   should not pick it up implicitly).
5. Place the donation QR in one of the proposed
   locations.
6. Local preview verification (run `npm run dev` and
   load in browser).

## Coordination

* @@FullStackB lane.
* Pure additive change to the repo tree.
* Pre-push gate green before commit clearance.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Where chan.app source lives today

Unknown from the architect side. Possibilities:

1. Separate repo (e.g. `chan-writer/chan-website`) —
   clone + port files in.
2. Local-only on @@Alex's machine — fire a permission
   event to access.
3. Lives at the chan.app live site — scrape / mirror
   via `wget`.

Fire a scope question / permission event if (1)/(2)/(3)
can't be resolved without @@Alex's input.

### Shared-infra authorization

* **Authorization: yes** for this task to create
  `web-marketing/` + a `web-marketing/README.md`.
* **Authorization NOT pre-granted** for
  `.github/workflows/` edits — CI wiring is a
  follow-up `ci-N`.
* **Authorization NOT pre-granted** for DNS / TLS
  changes — those are out-of-band and gated by
  @@Alex.

## Numbering

Highest committed `-b-N` is `-b-21`; `-b-22` is in
commit (orphan-sidecar reap). This is `-b-23`.

## 2026-05-21 — @@FullStackB: implementation note

Ported chan.app source from
`/Users/fiorix/dev/github.com/chan-writer/chan-prod-setup/etc/chan-site`
(local-only, confirmed with @@Alex via in-session question)
into `web-marketing/` at the repo root.

### Source shape

The chan.app source turned out to be **pure static HTML** — no
framework, no node_modules, no build pipeline. One single
`index.html` (370 lines, all CSS + JS inline) plus a handful of
PNG assets and the two install scripts the homepage references.

This is the most additive shape possible: no workspace member,
no npm/cargo footprint, no shared toolchain coupling. The
publishable artifact is the source tree itself.

### Ported files

From `chan-site/`:

| Source                          | Destination                              |
|---------------------------------|------------------------------------------|
| `index.html`                    | `web-marketing/index.html` (edited)      |
| `favicon.ico`                   | `web-marketing/favicon.ico`              |
| `chan-mark.png`                 | `web-marketing/chan-mark.png`            |
| `install.sh`                    | `web-marketing/install.sh` (exec mode)   |
| `install.ps1`                   | `web-marketing/install.ps1`              |
| `assets/editor-dark.png`        | `web-marketing/assets/editor-dark.png`   |
| `assets/editor-recipes.png`     | `web-marketing/assets/editor-recipes.png`|

From the main repo:

| Source                          | Destination                              |
|---------------------------------|------------------------------------------|
| `web/public/qr-donate.png`      | `web-marketing/qr-donate.png`            |

### Deliberately NOT ported

* `chan-site/site.nginx.conf` — nginx config for the legacy
  host. Per round-2-plan §"Decisions (all locked 2026-05-20)",
  Item-6 hosting is GitHub Pages with a custom domain. The
  nginx config decommissions alongside the legacy host once
  the Pages cutover lands (the follow-up `systacean-N` DNS
  task).

### Donation QR placement — §support section

Per the task body's "footer Support block, inline on the
download page, or a dedicated Support page" — I chose **a
dedicated §support section just above the footer**. Reasoning:

1. The site is single-page; "dedicated Support page" would
   mean a second HTML file, more URL surface, more split
   attention.
2. The footer is already busy with the email + github link;
   adding a QR there would dwarf the existing copy.
3. Slotting between §status and `<footer>` keeps the product
   story (install → vibe → features → editor → status) intact,
   then closes with the soft ask. Reads as "by the way, if you
   want to support this" rather than as a paywall.

The section uses the same `.section-tag` divider as the rest
of the page; the QR sits on a small white card (so a dark-mode
scan still works) with a caption pointing to `mailto:hello@chan.app`
as the alternative "say hi" channel. New CSS lives in a `/*
support / donation QR */` block right above the existing `/*
footer */` block; ~12 lines of style, mobile-flexbox-collapses
under 520px.

### Added files

| Path                            | Purpose                                  |
|---------------------------------|------------------------------------------|
| `web-marketing/README.md`       | Build + preview + deployment docs        |
| `web-marketing/.gitignore`      | `node_modules/`, `.DS_Store`, `dist/`, `build/` (forward-compat) |

### Workspace boundary

* `web-marketing/` has no `Cargo.toml`. The workspace's
  `members` array (root `Cargo.toml`) is explicit (not a glob),
  so cargo entirely ignores the new directory. Verified by a
  clean `cargo test --workspace` post-port.
* Not consumed by `web/` either — no shared package.json, no
  shared assets imported the other direction.
* Pre-push checks (fmt, clippy, workspace test, svelte-check,
  npm build, vitest) all ignore this directory.

### Preview verification

```
cd web-marketing
python3 -m http.server 8989
```

`curl` against the live preview:

| URL                          | Result   |
|------------------------------|----------|
| `/index.html`                | 200, title `chan markdown editor` |
| `/qr-donate.png`             | 200      |
| `/install.sh`                | 200      |
| `/favicon.ico`               | (served) |

§support section + QR image reference confirmed present in
the served HTML.

### Pre-push gate

| Surface                                                 | State                                                        |
|---------------------------------------------------------|--------------------------------------------------------------|
| `cargo fmt --check`                                     | Clean.                                                       |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean.                                                       |
| `cargo test --workspace`                                | All pass.                                                    |
| `cargo build --workspace --no-default-features`         | Clean.                                                       |
| `web/` `npx svelte-check`                               | 3987 / 0 / 0.                                                |
| `web/` `npx vitest run`                                 | 58 / 588 tests pass.                                         |
| `web/` `npm run build`                                  | Clean (only pre-existing chunk-size warnings).               |
| Local site preview                                      | `python3 -m http.server` + curl checks all 200; QR served.   |

### Acceptance criteria — verification

| Criterion                                                                                  | State                                                                            |
|--------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------|
| `web-marketing/` exists at repo root with the ported chan.app source                        | Yes; 7 source files + assets/ subdir.                                            |
| `web-marketing/README.md` documents the build + preview commands                            | Yes; documents `python3 -m http.server` + `npx serve` paths.                     |
| Local preview works                                                                         | Verified via `python3 -m http.server 8989` + curl smoke checks.                  |
| Donation QR placed at footer / download / Support page                                      | Dedicated §support section above the footer (rationale in this note).            |
| Static-site source builds into a publishable artifact directory                             | No build step — the source IS the artifact (pure static HTML).                   |
| `.gitignore` excludes build artifacts                                                       | Yes (`node_modules/`, `.DS_Store`, `dist/`, `build/`).                           |
| No CI wire yet (`ci-N` follow-up)                                                           | Confirmed; no `.github/workflows/` change in this commit.                        |
| No DNS cutover yet (`systacean-N` follow-up)                                                | Confirmed; nginx config not ported (lives with legacy host).                     |
| Pre-push gate green; cargo workspace ignores `web-marketing/`                               | Verified.                                                                        |

### Coordination footprint

* Pure additive — new top-level `web-marketing/` directory only.
* No file overlap with any in-flight FullStackA / Systacean /
  CI / WebtestA / WebtestB work in the shared tree.
* No edits to any existing file in this commit (only adds).

### Suggested commit subject

```
web-marketing: port chan.app static site source + donation QR section (fullstack-b-23)
```

Touches (all NEW files):

* `web-marketing/README.md`
* `web-marketing/.gitignore`
* `web-marketing/index.html`
* `web-marketing/favicon.ico`
* `web-marketing/chan-mark.png`
* `web-marketing/qr-donate.png`
* `web-marketing/install.sh`
* `web-marketing/install.ps1`
* `web-marketing/assets/editor-dark.png`
* `web-marketing/assets/editor-recipes.png`

Plus this task file: `docs/journals/phase-8/fullstack-b/fullstack-b-23.md`.

Standing by for @@Architect clearance.

## 2026-05-21 — committed as `bc9e1f8`

Cleared per @@Architect's `## 2026-05-21 — @@Architect: approved
+ commit clearance (fullstack-b-23)` heading in
[`../alex/event-architect-fullstack-b.md`](../alex/event-architect-fullstack-b.md).

Commit subject (accepted verbatim from the suggestion above):

```
web-marketing: port chan.app static site source + donation QR section (fullstack-b-23)
```

Files committed (explicit per-path `git add`, 11 paths total):

* `web-marketing/README.md`
* `web-marketing/.gitignore`
* `web-marketing/index.html`
* `web-marketing/favicon.ico`
* `web-marketing/chan-mark.png`
* `web-marketing/qr-donate.png`
* `web-marketing/install.sh`
* `web-marketing/install.ps1`
* `web-marketing/assets/editor-dark.png`
* `web-marketing/assets/editor-recipes.png`
* `docs/journals/phase-8/fullstack-b/fullstack-b-23.md`

Pre-commit `git diff --staged --stat`: 11 files, 944 insertions
— pure additive, no `deletions(-)` line. Post-commit
`git show --stat HEAD` matches the staged stat exactly. Push held
per release discipline.

Architect noted this is my final task this session before
recycle. Standing by for recycle.
