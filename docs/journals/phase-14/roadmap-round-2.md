# Phase 14 round 2

Round 1 brought the gateway into the monorepo. Round 2 is a deep code
review and cleanup of the frontend code base, now that the gateway's
identity SPA and shared web package sit alongside the existing editor
SPA. This is a quality pass, not a feature round: the surfaces work
today, so the goal is to keep the outcomes identical while making the
code simpler, less duplicated, and better documented.

Driven by `/webdev` (review + cleanup of the code) and `/architect`
(comments, documentation, and user-facing copy).

## Why now: first public release

This codebase has never been seen by anyone outside the team, and we
are about to publish it for the first time. There is therefore NOTHING
to be backwards compatible with, and no reason to carry historical
artifacts. Be ruthless: the published code must read as if it were
written today, from scratch, in its current shape. Anything that only
makes sense relative to the project's own history does not belong:

- back-compat shims, fallbacks, aliases, and deprecation paths;
- changelog-style comments ("now we also...", "used to be...",
  "renamed from...", "changed to...");
- references to phases, rounds, old names, or prior versions;
- dead transitional code kept "just in case".

Target state: pristine, fresh-like-new. No history in the source.

## Scope

The frontend trees:

- `web/` - the chan editor SPA (the largest surface; also embedded in
  the desktop app).
- `gateway/crates/identity/web/` + `gateway/web-common/` - the identity
  SPA and the shared gateway web package (Topbar, theme, api/initial
  helpers).
- `web-marketing/` - the marketing site and its release-metadata
  scripts.

## /webdev: review + cleanup

- **Correctness first.** The primary lens is correctness relative to
  today's *outcomes*: the surfaces are working. There may be hidden
  bugs, but in general things work, and the bar is that they keep
  working. Verify behavior against the live surfaces before and after
  any change; do not refactor in a way that risks an outcome. Hidden
  bugs found along the way get noted (and fixed when the fix is clearly
  correct), not papered over.
- **Remove obvious duplication.** Find and delete copy-pasted logic,
  parallel implementations of the same thing, and repeated patterns
  that can collapse into one.
- **Introduce abstractions where they help.** Look for abstractions
  that genuinely simplify readability and maintainability; prefer the
  smallest change that clarifies. No speculative or over-engineered
  layers.
- **Consistency across the frontends.** The editor SPA, the identity
  SPA, `web-common`, and the marketing site should share idioms,
  structure, naming, and component/util patterns where they overlap.
  Converge divergent approaches onto one idiomatic way; the code should
  read as idiomatic TypeScript / Svelte / Vite throughout, not as
  several different houses' styles.
- **Remove historical artifacts and back-compat.** Per "first public
  release" above: delete back-compat shims, fallbacks, aliases,
  deprecation paths, and dead transitional code. The code should read
  as if the current shape is the only shape that ever existed.
- **Strip changelog-style comments.** Most code comments were written
  as a running changelog ("now we also...", "changed to...", "used to
  be...") rather than a snapshot of the current reasoning. Remove the
  changelog narration; keep only comments that explain WHY the current
  code is the way it is (the reason, trade-off, or constraint), per the
  workspace writing principles. When the code already shows the WHAT,
  the comment goes.

## /architect: comments, docs, and user-facing copy

- Review all frontend code comments, the frontend-related documentation,
  and the user-facing communication (UI copy, banners, error/empty
  states) for clarity, factual accuracy, and reduced ambiguity.
- Comments and docs are **written for human consumption**: clear,
  readable prose for a person reading the code, not terse machine
  notes and not a log. They read as a factual snapshot of the present
  state, not a history. No marketing language, no em dashes, ASCII
  tables, claims verified against the implementation.
- Apply one consistent voice and convention across all four frontend
  trees, so a reader moving between them finds the same documentation
  style.
- User-facing copy should be unambiguous and accurate to what the
  surface actually does.

## Non-goals

- Not a feature round and not a redesign. Behavior and visible outcomes
  stay the same.
- Not a bug hunt for its own sake; correctness is framed around
  preserving the working surfaces, not chasing every theoretical edge.

## Definition of done

- Duplication removed where obvious; the remaining abstractions read
  more clearly than before, with no behavior change to the working
  surfaces (verified against the running app).
- No back-compat shims, aliases, deprecation paths, or other
  historical artifacts remain in the frontend; the source reads
  fresh-like-new.
- Comments across the frontend are snapshot-style (WHY, not a
  changelog), written for a human reader; the changelog-style narration
  is gone.
- The four frontend trees are consistent and idiomatic: shared idioms,
  naming, structure, and one documentation voice.
- Frontend docs and user-facing copy are clear, factual, and
  unambiguous.
- `npm run check` / `npm run build` / tests pass for each affected
  frontend tree; the gateway and core gates stay green.
