# Phase-16 round-1: decisions for @@Alex

@@Lead consolidated the open calls below. Edit in place: write your answer on
the `@@Alex:` line under each item (replace the blank). Recommendations are
noted so you can just confirm the default or override in a word. NONE of these
block the round - lanes are building the decided parts and holding only the
gated slices. Save when done and tell me (or I'll pick it up and dispatch).

main* mbp ~/dev/github.com/fiorix/chan $ env | grep WIN
CHAN_WINDOW_ID=workspace-1abd876af1451ce6-0

================================================================================

## 1. D1 - online-service production-setup docs depth

The tunnel/gateway messaging reframe already SHIPPED (merged 78c4586b). This
is only about how deep to document running your OWN online service.

  (a) Add a DNS-wildcard + Let's Encrypt section to gateway/README only.
  (b) New gateway/docs/production-setup.md full walkthrough (DNS + certbot +
      nginx vhosts for id+workspace + systemd + admin enrollment).
  (c) Defer the deep-dive; ship the reframe alone this round.

@@Alex (a / b / c): oh I see.. I use CF DNS + LE, recommend this setup

If (a) or (b), @@LaneE needs your infra specifics:
  - DNS provider + wildcard setup
    @@Alex: users would have to choose.. we don't have to be so specific.. just a guide, so their agents know what to do
  - cert method: certbot dns-01 wildcard vs http-01
    @@Alex:  i don't know.. we shoudln't recommend much, we should just explain that they have to choose
  - nginx vhost layout for id.chan.app + workspace.chan.app
    @@Alex: yes similar to our layout if it makes sense.. 
  - how much of your private chan-prod-setup repo to mirror (oauth config,
    user enrollment, workspace sharing)
    @@Alex: not much... we should probably do this in a way that users on a mac can setup this entire prod-like environment on a lima-vm using sdme like we do in prod; and for users on linux this would be on a local setup as well

================================================================================

## 2. P2 - onboarding card shape

After P2 opens a workspace, the SPA shows a first-load card. Should it be a
thin first-run NUDGE that points at Settings, or a full inline Semantic/
Reports TOGGLE pair?
Recommendation (@@Lead + @@LaneC): NUDGE (avoids duplicating Settings toggles).

@@Alex (nudge / toggles): NUDGE

================================================================================

## 3. F4 #3 - external-link "open" affordance

Your original ask mentioned a "small bubble with open". Two shapes:
  (A) Context-menu item only: body right-click shows "Open link" / "Copy link".
  (B) Hover bubble: a floating "Open" on hover over a link, PLUS the menu item.
@@LaneD leans A-first (B as a follow-up); but you described a bubble, so I'm
putting it to you rather than quietly downgrading.

@@Alex (A / B / A-now-B-later): A

================================================================================

## 4. F4 #4 - markdown preview scope

You asked for markdown previews (terminal = read-only). Proposed scope:
  - internal [[wiki]] / relative-md link -> a small preview of the target's
    rendered markdown. (Fetching arbitrary external URLs is out of scope.)
  - trigger: on hover, or via a menu item.
@@LaneD leans internal-link only, menu-item trigger first.

@@Alex (scope ok as above? hover or menu-item?): follow LaneD's recommendation

================================================================================

## 5. Rebuild + restart the team server?

Everything merged so far runs on code NEWER than the live team server (it
predates this round). To actually exercise C2 `cs t sc` / `cs pane` / P1 cs-
link / the F-series / TW1 - and for me to self-host on `cs t sc` - the team
server needs a rebuild + restart, which RESPAWNS the lanes. Your call on when.

@@Alex (rebuild now / keep building wave-2 / later): rebuild now and keep rebuilding on every wave if needed

  RELATED FINDING (confirmed by @@LaneA): agent terminals have NO
  CHAN_WINDOW_ID (spawn_team sets window_id:None, SPA attach never backfills),
  so `cs pane` / `cs open` / `cs survey` can't target a window from an agent
  context - which is why my survey + cs open failed. So even after a rebuild,
  @@Lead self-hosting on `cs pane` needs a fix. @@LaneA drafted: (B) add a
  `cs pane --tab-name` selector (small, unblocks scripted/agent use now); (A)
  bind agent sessions to the displaying window (complete fix, also fixes cs
  open/survey, pairs with S1). @@Lead leans B-now + A-with-S1. Want these in
  this round (alongside C3b/S1) or round-2?
  @@Alex (window-id fix: B-now / A+B / round-2): only other terminals do? becuase my regular terminal does:  CHAN_WINDOW_ID=workspace-1abd876af1451ce6-0

I'm surprised we don't already have that.. we have to, because we already have this for regular terminals

================================================================================

## 6. Date-bound CI follow-up (deadline 2026-06-16)

deploy-pages@v4 (-> v5) and apple-actions/import-codesign-certs@v3 (-> v7, the
macOS signing path) are still Node-20 and break after 06-16. B1 deliberately
left them out (riskier, signing-sensitive). Do them this round (stretch) or
round-2?
@@Lead: either is fine - date-bound but not urgent (15 days out).

@@Alex (this round / round-2): do this round 

