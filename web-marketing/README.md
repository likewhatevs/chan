# web-marketing — chan.app static site source

Source for the public chan.app marketing site. Pure static HTML
plus a couple of PNG assets and the two `install.sh` /
`install.ps1` install scripts the homepage references via
`curl | sh` / `irm | iex`. No build step, no framework, no
node_modules.

Ported in `fullstack-b-23` from the historical chan.app source
that lived outside the main repo. Now in-tree so the Round-2
backlog Item 6 work (website + manual + first-launch UX + CI)
can build from a single source of truth.

## Layout

```
web-marketing/
├── README.md            this file
├── .gitignore           preview tool detritus (node_modules/, .DS_Store)
├── index.html           the homepage; single file, all CSS + JS inline
├── favicon.ico
├── chan-mark.png        ensō brushstroke, painted via CSS mask
├── install.sh           macOS + Linux install script (curl | sh target)
├── install.ps1          Windows install script (irm | iex target)
├── qr-donate.png        donation QR rendered into the §support section
└── assets/
    ├── editor-dark.png
    └── editor-recipes.png
```

`index.html` is the single visible page. Everything else is
either a referenced asset or one of the install-script payloads
the homepage links to.

## Preview locally

Pick whichever HTTP server is closest to hand. The site only
needs static file serving on a single port; no API, no SSR, no
hot reload.

```
# Python (no install needed on macOS / most Linux):
python3 -m http.server 8080

# Node (if you happen to have npm around):
npx serve

# Then point your browser at:
#   http://localhost:8080/
```

Opening `index.html` directly via `file://` mostly works but
breaks the absolute asset paths (`/favicon.ico`, `/chan-mark.png`,
`/assets/...`, `/qr-donate.png`). Use a local server for an
accurate preview.

## Theme + screenshot toggles

The page has two interactive bits, both driven by inline
`<script>` blocks at the bottom of `index.html`:

* **Site theme** — `bone` (warm light) vs `ink` (dark). Toggle
  in the header. Persisted in `localStorage` under the
  `chan-mode` key. First-visit default follows the OS via
  `prefers-color-scheme`.
* **Editor screenshot theme** — light vs dark for the editor
  shot. Independent of the site theme.

Edit the inline `<style>` block to tune colours; the `--accent`
custom property carries the brand orange and the `.brand .enso`
mask + section-tag glyphs both pick it up.

## Donation QR (§support section)

`qr-donate.png` is mirrored from `web/public/qr-donate.png` in
the main repo (the same QR used inside the chan editor's
Settings about pane). Refresh the asset by re-copying from there
when it changes; the in-page styles size it down to 140 px and
add a white card behind it so a dark-mode scan still works.

## Deployment target

GitHub Pages with a custom domain (`chan.app`), per the locked
Round-2 Item 6 decisions in
`docs/journals/phase-8/architect/round-2-plan.md`. Static-only
shape means the publishable artifact is the source tree itself;
no `dist/` step is required.

CI publishing wiring is a follow-up `ci-N` task; this `-b-23`
task lands the source only. DNS cutover from the current
nginx-on-VPS host is a follow-up `systacean-N` task.

## Not bundled here

* `site.nginx.conf` from the historical source. Lives with the
  legacy nginx host and decommissions alongside it once the
  Pages cutover lands.
* No package manifest. The site has zero runtime dependencies
  and no build pipeline. Adding a framework (Astro / Eleventy /
  etc.) would be a much larger architectural change and is
  explicitly NOT in scope for this port.

## Workspace boundary

`web-marketing/` is intentionally outside the cargo workspace
and the main `web/` SPA. Nothing here is consumed by `cargo
build`, `cargo test`, or `web/` build pipelines. Pre-push checks
(`cargo fmt`, `cargo clippy`, `cargo test`, `svelte-check`, `npm
run build`) ignore this directory entirely.
