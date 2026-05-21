# Round-2 open questions

Author: @@Architect (curating); @@Alex (replying)
Started: 2026-05-21

Curated index of architect-side open questions + blocking actions awaiting @@Alex's input. @@Alex drops replies inline under each entry; @@Architect picks up the answers + routes the resulting work into the canonical decision artifacts (`round-2-plan.md`, `journal.md`, task files).

Distinction:

* **Open questions** (§A) = plan / dispatch decisions @@Alex needs to confirm before fan-out. Architect-recommended option named in each.
* **Blocking actions** (§B) = hands-on tasks @@Alex needs to do (terminal commands, GitHub UI work, secret population). NOT decisions; gates on existing dispatched work.

---

## A — Open questions for plan + dispatch decisions

### A.1 — Hybrid back-side: per-surface scope of settings (incl. theme)

**Source**: [`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side revisited" Q1 + [`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md).

**Question**: are settings on the back per-type (one terminal config applies to every terminal) or per-tab (each tab gets its own)? Including the dark/light theme?

**Architect recommendation**: per-type. Hybrid-revisited spec already implies this ("settings impact all terminals, not just the current terminal").

**Reply from @@Alex 2026-05-21**:

> correct, per-type, including the dark/light switch.
> My case here is that I like my docs light, my terminals dark.

**Status**: ANSWERED. Per-type confirmed. Theme is part of the per-type config.

**Follow-up nuance for @@Architect to confirm with @@Alex before transcribing**: under per-type theme, "docs light + terminals dark" implies the theme value LIVES WITH the surface type globally (all editors render light; all terminals render dark) rather than living per-pane (per `-b-5`). Two interpretations:

* (i) **Global per-type theme**: theme is a property of the surface type. When a pane shows an editor, it renders with the global editor theme. Switching tabs within a pane (editor → terminal) auto-flips the visible theme. `-b-5`'s per-pane override goes away entirely.
* (ii) **Per-pane theme with per-type defaults**: theme is still per-pane (today's `-b-5` shape) but defaults per surface type ("when this pane first shows an editor, default to light; when it first shows a terminal, default to dark"). Hamburger toggle from `-a-27` still overrides per-pane.

(ii) keeps `-b-5`'s per-pane independence; (i) simplifies but loses the per-pane affordance. **Recommend (i)** since @@Alex's "docs light, terminals dark" framing reads more naturally as a global preference than a per-pane default. Confirm before Task E (drop front/back independent theme) cuts.

**Follow-up reply from @@Alex 2026-05-21**:

> global per-type

**Status**: ANSWERED. Global per-type theme — theme lives with the surface type, not the pane. `-b-5`'s per-pane override goes away entirely. All editors render with the editor theme; all terminals render with the terminal theme. Hamburger toggle from `-a-27` flips the GLOBAL theme for the current surface type (not per-pane). Task E (drop front/back independent theme) becomes "drop per-pane theme override entirely; move to global per-type". Update the round-2-plan "Hybrid back-side revisited" section + the Task E spec at fan-out.

---

### A.2 — Hybrid back-side: Hybrid File Browser back v1 content

**Source**: [`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side revisited" Q2.

**Question**: what goes on the back of a Hybrid File Browser?

**Architect recommendation** (original): empty placeholder ("reserved for future use").

**Reply from @@Alex 2026-05-21**:

> back of FB: we will include the data that is today in the
> inspector for the drive (screenshot referenced).

**Status**: ANSWERED. FB back-side = the drive inspector data (drive metadata: name, path, language breakdown, size, etc. — the existing inspector surface).

**Implementation note**: this is a richer affordance than my "empty placeholder" recommendation. Composes with `-a-33`'s ancestor breadcrumb work (the drive inspector surface in the Graph view shares the same data). Need to confirm whether the FB back-side renders the SAME inspector component the Graph view uses, OR a tailored variant. Recommend reusing the same component for now — single source of truth on what "drive inspector data" looks like.

---

### A.3 — Hybrid back-side: Search overlay's Hybrid future + Graph back

**Source**: [`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side revisited" Q3.

**Question**: does the search overlay become a 5th Hybrid surface with its own back?

**Architect recommendation**: stays out-of-Hybrid.

**Reply from @@Alex 2026-05-21**:

> there's no back of the hybrid search; sorry if i created
> confusion here. We need a back of the Graph tab, which I
> spec'd with the colours.

**Status**: ANSWERED. Two pieces:

* Search overlay stays out-of-Hybrid (matches recommendation). No back.
* Graph back = colour legend grid (`[Node] [Colour]` for Dir / File subtypes / Hashtag / Mention / Language). Already in the hybrid-revisited spec. Confirmed.

---

### A.4 — Hybrid back-side: Wave-2 vs Wave-3 split for the 5-task implementation

**Source**: [`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side revisited" Q4.

**Question**: 5 tasks (A: architecture, B: terminal migration, C: editor migration, D: graph legend, E: drop front/back theme split). All Wave 2, all Wave 3, or split?

**Architect recommendation**: Task A rides Wave 2 as a hard-prereq; Tasks B/C/D/E land in Wave 3.

**Reply from @@Alex 2026-05-21**:

> will take your recommendation here

**Status**: ANSWERED. Task A in Wave 2; B/C/D/E in Wave 3.

---

### A.5 — v0.11.2 patch scope: cut a patch wave or hold for v0.12.0?

**Source**: this session's bug filings (Alex-visible UX bugs from v0.11.1 dogfooding).

**Plain-language question**: do you want me to ship a small patch release (v0.11.2) NOW with the critical UX fixes, or do those bugs ride along into the next proper release (v0.12.0, end of Round 2 — weeks away)?

**Trade-off**:

* **Cut the patch**: fixes ship in days. Same shape as the v0.11.1 mini-wave (~5-7 commits, tag, push). Costs you another tag-cycle to coordinate; runs CI; produces another GitHub Release.
* **Hold**: fixes ship at v0.12.0 (Round-2 close). No extra tag cycle. You live with the bugs daily until the signed-DMG pipeline is green + tagged.

**Candidate patch scope** (updated 2026-05-21 — Cmd+F withdrawn, "Copied path" notification added):

| # | Bug | Severity |
|---|-----|----------|
| 1 | "File moved or deleted" false-positive (interrupts active writing) | CRITICAL |
| 2 | Tab right-click Reload + Open Inspector no-op on chan-desktop | DEV META-BLOCKER |
| 3 | Pre-flight bubble spinner stuck at `0:00` | Medium |
| 4 | FB expand/collapse state lost on tab switch | Medium |
| 5 | Source-mode editor list intervention (auto-continuation in raw mode) | Paper-cut |
| 6 | "Copied path" status-bar notification doesn't auto-dismiss | Paper-cut |

#1 + #2 alone justify a patch (CRITICAL + DEV META-BLOCKER). #3-6 ride along cheaply.

**Architect recommendation**: **cut the patch**. The CRITICAL "file moved or deleted" interrupts active writing TODAY; the DEV META-BLOCKER gates your ability to inspect / debug everything else on chan-desktop; the rest are cheap to bundle along.

**Your reply** (just say yes / no / something else): 

**Reply from @@Alex 2026-05-21**:

> 1. yes

**Status**: ANSWERED. **Cut v0.11.2.** Architect cuts task files + dispatches the mini-wave; @@Systacean cuts the tag once the wave lands green. **Expanded scope 2026-05-21 per @@Alex's follow-up** ("if there are more items that DO NOT need much spec and we have answers, we could bake all into this release too" + "the working agents have been mostly idle this session, mostly you") — packed maximally with anything well-defined + answered + small scope. Final v0.11.2 scope = 10 fixes across 9 tasks:

@@FullStackA (6 tasks):
* `-a-36` Tab Reload + Inspector SPA (paired with `-b-17`)
* `-a-37` File moved or deleted false-positive (CRITICAL)
* `-a-38` Notification surface polish (spinner gating + Copied path auto-dismiss)
* `-a-39` FB tab state polish (expand persistence + spawn-new chord)
* `-a-40` Wysiwyg outline-style dotted numbering (CSS counters per A.7 a)
* `-a-41` Source-mode editor list intervention

@@FullStackB (3 tasks):
* `-b-17` Tab Reload + Inspector Tauri IPC (paired with `-a-36`)
* `-b-18` Submit-mode persistence on reload + tooltip copy fix
* `-b-19` chan-desktop browser-style zoom (Cmd + / - / 0)

Plus already-landed Round-2 Wave-1 work absorbs into the patch tag:
* `-b-15` + `-b-16` (bundled chan binary + PATH-first probe)
* `ci-7` (signing workflow YAML)
* `systacean-11 + -12 + -13` (signing-key rotation + tauri-plugin-updater verify + Keychain-driven Makefile)

Full plan at [`commit-plan-v0.11.2.md`](commit-plan-v0.11.2.md). Tasks dispatched 2026-05-21 via outbound poke channels.

---

### A.6 — Markmap support: Round-2 wave-2 or Round-3?

**Source**: [`../phase-8-bugs.md`](../phase-8-bugs.md) markmap entry (filed 2026-05-20).

**Question**: where does the markmap support feature land?

**Architect recommendation** (original): Round-2 wave-2 alongside rich-prompt session evolution.

**Reply from @@Alex 2026-05-21**:

> whenever we can.. not very important; check if the license
> is compatible with embedding btw; i will want to be able to
> 1) edit the markdown in our rich editor, and 2) flip a
> switch to see the rendered mindmap instead; the switch can
> be created next to the frontformmat thing

**Status**: ANSWERED. Three sub-answers:

1. **Priority / sequencing**: low priority. Round-3 polish neighbourhood (not wave-2). Cut whenever a quieter slot opens.
2. **License compat check**: required before task-cut. Architect's existing markmap-entry assertion: markmap is MIT (Apache-2.0 compatible). Transitive deps: D3 (BSD-3-Clause, also Apache-2.0 compatible). Implementer verifies the FULL transitive dep tree at task-cut + adds attribution row to the Settings About section (alongside Source Code Pro's OFL.txt row from `-b-12`).
3. **UX requirements** (both confirmed):
   * Edit markdown in chan's rich editor (preserves the wysiwyg / source mode authoring path; markmap doesn't replace editing — it's a third VIEW).
   * Toggle switch to flip wysiwyg ↔ markmap. Switch placement: "next to the frontformmat thing" — interpreted as the StyleToolbar mode-toggle from `-a-26` (where the wysiwyg / source toggle lives). So the toolbar becomes a 3-way mode select: wysiwyg / source / markmap. Markmap is read-only (matches architect's original recommendation; reads "edit in wysiwyg, flip to markmap to view structure, flip back to keep editing").

**Architect follow-up**:

* Re-route the markmap entry in [`../phase-8-bugs.md`](../phase-8-bugs.md) from "Round-2 wave-2" → "Round-3 polish (whenever quiet slot)".
* Confirm @@Alex meant "next to the StyleToolbar mode toggle" — if they actually meant a different surface (the YAML frontmatter editor area, etc.), the placement changes.

**Follow-up reply from @@Alex (placement confirmation) 2026-05-21**:

> 5. yes

**Status**: ANSWERED. StyleToolbar mode-toggle confirmed. Markmap entry in `phase-8-bugs.md` re-routes from Round-2 wave-2 → Round-3 polish (whenever quiet slot).

---

### A.7 — Wysiwyg outline-style dotted numbering: implementation shape

**Source**: [`../phase-8-bugs.md`](../phase-8-bugs.md) wysiwyg outline-numbering entry (filed 2026-05-21).

**Question**: option (a) pure visual (CSS counters) vs option (b) source change (literal `1.1.` text)?

**Architect recommendation**: **(a) pure visual / CSS counters**. Source stays standard markdown; portability across other tools (GitHub, Obsidian, etc.) preserved; chan's distinctive display lives only in chan's renderer.

**Options recap**:

* (a) **RECOMMENDED**: CSS counters. Source: `1. / \t1. / 2.` (standard). Render: `1. / 1.1. / 2.`.
* (b) Source change. Source: `1. / 1.1. / 2.` (chan-specific). Render matches source. WYSIWYG-source view parity but breaks markdown standard.

**Reply from @@Alex 2026-05-21**:

> agree A

**Status**: ANSWERED. Option (a) pure visual / CSS counters. Update the wysiwyg outline-numbering bug entry in `phase-8-bugs.md` to lock the implementation direction.

---

## B — Blocking actions (hands-on @@Alex tasks)

### B.1 — systacean-11: provide `APPLE_SIGNING_IDENTITY` string

**Source**: [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) 2026-05-20.

**Context**: @@Systacean's `systacean-11` rotates `desktop/src-tauri/tauri.conf.json` from the DEV signing posture to the release Developer ID identity. Identity NAME is needed to land the JSON edit. @@Alex has the cert in their Keychain (confirmed 2026-05-20) but hasn't shared the identity string yet.

**Format needed**: `Developer ID Application: <Your Name> (<TEAMID>)` — e.g. `Developer ID Application: Alexandre Fiori (ABCD123456)`.

**Reply path**: append a `## 2026-05-21 — approved` section to [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) with the identity string. @@Systacean lands the JSON rotation commit that day.

**Reply from @@Alex 2026-05-21**:

> 2. you can get this from my machine already.. there are 2
> there, a new one which i recently used for chan, and an
> old one from 2013 iirc which you can discard

**Status**: ANSWERED. @@Architect ran `security find-identity -v -p codesigning` 2026-05-21; the recent Developer ID Application cert is `Developer ID Application: Alexandre Fiori (W73XV5CK3N)` (Team ID `W73XV5CK3N`). The Apple Development cert (second valid identity) is a different category (dev builds, not distribution). The 2013-era cert @@Alex remembered is already pruned from the keychain (not in current valid-identities listing).

Transcribed to [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) as `## 2026-05-21 — approved (transcribed by @@Architect)`. @@Systacean lands the JSON rotation commit on the next inbound poll.

---

### B.2 — ci-8: confirm secrets populated in GitHub Actions

**Source**: [`../alex/event-ci-alex.md`](../alex/event-ci-alex.md) 2026-05-20.

**Context**: @@CI's `ci-8` (DMG-on-tag dry-run with real Apple Developer ID keys) is parked pending @@Alex confirming the six signing secrets are populated in the chan repo's GitHub Actions Secrets. Per the [ci-3 brief](../../../release/macos-signing.md) "GitHub Actions Secrets shape" table:

| Secret | Holds |
|--------|-------|
| `APPLE_CERTIFICATE_BASE64` | Base64-encoded `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` export passphrase |
| `APPLE_SIGNING_IDENTITY` | Full cert name (same as B.1) |
| `APPLE_TEAM_ID` | 10-char Team ID |
| `APPLE_ID` | Apple developer account email |
| `APPLE_PASSWORD` | App-specific password from `account.apple.com` |

**Reply path**: append a one-line "checklist done, all six secrets populated" (or "still pending, no ETA") to [`../alex/event-ci-alex.md`](../alex/event-ci-alex.md). Optionally confirm the test-tag name (default `chan-v0.11.99-dryrun.1`).

**Reply from @@Alex 2026-05-21**:

> 3. do i have to do this? we have the gh binary here, you
> do it if you can

**Status**: IN PROGRESS. @@Architect attempted to populate via `gh` CLI 2026-05-21 — auto-mode classifier blocked the multi-step credential-extraction + remote-write combo. Provided @@Alex with a one-shot shell script to run locally (values pipe through stdin direct to `gh secret set`; no values appear in shell history; `security export` step pops a Keychain "Allow access" dialog).

Script includes:
* `printf 'Developer ID Application: ...' | gh secret set APPLE_SIGNING_IDENTITY` (public identifier)
* `printf 'W73XV5CK3N' | gh secret set APPLE_TEAM_ID`
* `printf 'fiorix@gmail.com' | gh secret set APPLE_ID`
* `security find-generic-password -s chan-notary -w | tr -d '\n' | gh secret set APPLE_PASSWORD`
* `openssl rand` generates a strong fresh passphrase; `security export -t identities -f pkcs12 -P "$PASSPHRASE" -o /tmp/chan-developerid.p12` exports
* `printf '%s' "$PASSPHRASE" | gh secret set APPLE_CERTIFICATE_PASSWORD`
* `base64 -i /tmp/chan-developerid.p12 | tr -d '\n' | gh secret set APPLE_CERTIFICATE_BASE64`
* Cleanup + `gh secret list` verify

Awaiting @@Alex's run + a one-line confirmation `gh secret list` shows all six names → status flips to ANSWERED + @@CI's `ci-8` unblocks for the dry-run.

**Update from @@Alex 2026-05-21**:

> 2. done! it worked (this was incredible, I ran and it
> worked on first try, perfeccc; thank you and team!)

**Status**: ANSWERED. Script ran clean on first try; all six secrets populated in GitHub Actions Secrets. `ci-8` DMG dry-run is now unblocked. Transcribed to [`../alex/event-ci-alex.md`](../alex/event-ci-alex.md) so @@CI picks up the green light on next inbound poll.

---

### B.3 — systacean-13: optional local smoke test

**Source**: [`../systacean/systacean-13.md`](../systacean/systacean-13.md) tail (commit-readiness append, 2026-05-21).

**Context**: @@Systacean landed the Keychain-driven `make app-notarized` Makefile change. Smoke test is optional but recommended: from a bare shell (no env exports), run `cd desktop && make app-notarized` and confirm the resulting `.dmg` opens cleanly on a second Mac with no Gatekeeper warning. Validates the whole local notarization path end-to-end + de-risks the eventual `ci-8` dry-run.

**Pre-req**: the `chan` notarytool keychain profile must be set up (see `desktop/CLAUDE.md` "Local notarization setup" — `xcrun notarytool store-credentials chan --apple-id ... --team-id ... --password ...`).

**Reply path**: append a result note ("smoke test green" / "failed at step X with error Y") to [`../systacean/systacean-13.md`](../systacean/systacean-13.md). Does NOT block @@Systacean's commit clearance — the commit can land before the smoke test runs.

**Status**: 

---

## How to use this file

1. @@Architect maintains this file as new open questions accumulate. New questions get appended under §A; new blocking actions under §B.
2. @@Alex drops replies in the "**Your reply:**" / "**Status:**" / "**Reply from @@Alex:**" line beneath each entry. Multi-line replies OK; bullet points OK; free-form OK.
3. When a question is fully answered, @@Architect:
   * Mirrors the answer into the canonical decision artifact
     ([`round-2-plan.md`](round-2-plan.md) for plan-level
     decisions; the relevant agent task file for task-level
     answers; [`journal.md`](journal.md) decisions log for
     the audit anchor).
   * Marks the entry "**Status: ANSWERED**" + one-line summary
     here. Keeps the entry for the audit trail; doesn't
     delete.
4. Periodically the resolved entries can move to a "RESOLVED" section at the bottom; keep the active list lean for at-a-glance scanning.

## RESOLVED archive

(empty)
