# Live Verification a210034

Date: 2026-05-24
Owner: @@Architect
Source: @@WebtestLive
Status: passed with expected Browser/iab limitations

## Run

- HEAD: `a210034 Keep horizontal rule source visible`
- Browser: Browser/iab
- URL: `http://127.0.0.1:8787/?t=8m8dm5cPhvHwmUzhtSUbFHFwTfYDJkBm`
- Throwaway HOME: `/private/tmp/chan-phase9-a210034-home`
- Throwaway drive: `/private/tmp/chan-phase9-a210034-drive`
- Server stopped after verification.

## Results

- `npm run build`: PASS, existing Vite warnings only.
- `cargo build -p chan`: PASS, built current bundle.
- App load: PASS, no current-run console or page errors.
- Metadata export UI: PASS, Infographics reported 31 files, 257.2 KB.
- Metadata export API: PASS, `.tar.zst`, `application/zstd`,
  `x-chan-metadata-files: 31`, `x-chan-metadata-bytes: 263397`.
- Metadata import API: PASS, direct import returned `files:29`,
  `bytes:262767`, `rescanned:true`; no `drive busy`.
- Browser file picker: PARTIAL, file input exists but Browser/iab still lacks
  `setInputFiles`.
- File Browser smoke: PASS, root tree opens.
- Drafts hidden from File Browser: PASS, root showed `docs/` and `start.md`,
  no Drafts row. Direct restore of `Drafts` did not open or show Drafts.
- New Draft: PASS, opens editable `Drafts/untitled/draft.md`.
- Graph from draft: PASS, renders graph with no `no such path`.
- Terminal from draft: PASS, terminal tab opens from active Drafts tab.
- WYSIWYG bare `---`: PASS, visible as source text; `hrCount: 0`.
- Console/page errors: PASS, current-run warn/error log empty.

## Limitations

- Browser/iab cannot currently drive the native file picker, so import UI
  selection remains partial despite the endpoint passing.
- Codex MCP stale-socket fallback was not exercised in this Browser/iab run.
  Terminal opened, but Browser/iab cannot reliably type and read terminal
  output for that probe.

## Closed By This Run

- Live metadata import no longer returns `drive busy`.
- File Browser hides Drafts while editor, graph, and terminal draft workflows
  remain available.
- WYSIWYG no longer renders bare `---` as a hidden-source horizontal rule.
