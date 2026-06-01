# bootstrap.md - chan branding rollout

You are one of several agents applying the chan branding/positioning
rollout in parallel. Follow these steps in order. Stay inside your lane.


## 1. Find out who you are

Run:

```
echo "$CHAN_TAB_NAME"
```

That value is your lane name (for example "LaneA" or "@@LaneA"; ignore
any leading @@). Map it:

```
LaneA -> positioning text: README.md, design.md, docs/manual/index.md,
         CLAUDE.md, AGENTS.md, + sync branding-story.md   [Wave 1, now]
LaneB -> in-app Dashboard About slide (web/src)           [Wave 1, now]
LaneC -> marketing site + founder /story page             [Wave 2, GATED]
```

If CHAN_TAB_NAME is empty or is not one of LaneA / LaneB / LaneC, STOP
and ask @@Alex which lane you are. Do not guess.

LaneC only: this wave is GATED on screenshots. Before doing anything,
confirm the new Team Work + refreshed screenshots are already in
web-marketing/assets/. If they are not there yet, STOP and tell @@Alex
you are waiting on them.


## 2. Read the plan

In this directory (docs/journals/pub-site-release/), read:

```
execution-plan.md   the "Shared rules" section, then YOUR lane's
                    section in full (exact files + verbatim new copy)
branding-story.md   the locked brand decisions (context)
founder-note.md     the /story page draft  (LaneC only)
```


## 3. Do your lane

Apply exactly what your lane's section specifies, and verify the way that
section says to. Reminders that bite if ignored:

- No em dashes. README/design/manual/CLAUDE/AGENTS stay factual; brand
  voice is for the site + /story page only.
- Keep "sigma" / "100x" / "first IDE" out of all new copy. Do NOT edit
  brainstorm.md.
- Do NOT kill the running chan.app. In-app smoke uses a renamed binary on
  a throwaway workspace + separate port.
- For web/src changes: `npm run build` in web/ FIRST, then
  `cargo build -p chan`, then a hard browser reload.
- Do NOT commit or push unless @@Alex tells you to.


## 4. Report and stop

Summarize what you changed (files touched + one line each) and stop for
@@Alex's review. Do not start another lane's work.
