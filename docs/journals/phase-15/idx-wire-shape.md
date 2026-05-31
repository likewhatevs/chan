# IDX status wire shape (Option A) - @@LaneC -> @@LaneB contract

Architect-approved (2026-05-31). Additive, pre-release (no back-compat).
@@LaneC owns the chan-server side that EMITS this; @@LaneB owns the SPA
side that CONSUMES it. This file is the single source of truth for the
shape so we do not diverge.

## Backend (chan-server, @@LaneC) - IndexStatus::Idle gains one field

Rust (crates/chan-server/src/indexer.rs), serde tag = "state",
rename_all = "snake_case" (existing on the enum):

    Idle {
        indexed_docs: u64,
        indexed_vectors: u64,
        model: String,
        // NEW: background embedding progress. Some while embeddings
        // run in the background AFTER BM25 is ready; None when fully
        // settled (or when the build had no embed phase).
        embedding: Option<EmbedProgress>,
    }

    // serde rename_all = "camelCase"
    struct EmbedProgress {
        done: u32,   // chunks/batches embedded so far
        total: u32,  // total to embed; done <= total (clamped)
    }

JSON on the wire (GET /api/index/status):

    // BM25 ready, embeddings still running in background:
    {"state":"idle","indexed_docs":4096,"indexed_vectors":120,
     "model":"bge-small-en-v1.5","embedding":{"done":12,"total":96}}

    // fully settled (no background work):
    {"state":"idle","indexed_docs":4096,"indexed_vectors":4096,
     "model":"bge-small-en-v1.5","embedding":null}

Notes:
- `state` is STILL "idle" during background embedding. That is the
  whole point of Option A: BM25-ready == idle == preflight-ready, and
  embeddings are a non-blocking background refinement.
- `embedding` is `null` (absent value) when there is nothing embedding.
- `done <= total` always (clamped backend-side; same invariant as the
  4097/4096 display clamp). Frontend can render done/total directly.
- Building / Reindexing variants are UNCHANGED. They still mean a
  foreground pass that gates preflight.

## Frontend (web, @@LaneB) - 3 touch points

1. web/src/api/types.ts
   - IndexStatus Idle variant gains `embedding?: { done: number; total:
     number } | null`. (Match the existing Idle field naming;
     indexed_docs/indexed_vectors are snake in the current type - keep
     whatever the current type uses for those and add `embedding` the
     same way. The nested object is camelCase: done/total.)

2. web/src/components/AppStatusBar.svelte
   - When state === "idle" && embedding != null: render a PASSIVE
     "embedding done/total" chip (or a thin progress). This is the
     calm "search is getting better in the background" affordance.
   - It MUST NOT look like the old stuck "reindexing {path}" pill and
     MUST NOT block anything. When embedding == null: fully idle, no
     chip.
   - Building/Reindexing rendering unchanged.

3. web/src/components/PreflightOverlay.svelte (VERIFY-ONLY, likely no
   change)
   - Overlay must unlock on `idle` regardless of `embedding`. preflight
     already maps Idle -> ready (routes/preflight.rs:104), so the SPA
     should already unlock; just confirm the embedding field does not
     accidentally keep it locked. No new gating on `embedding`.

## Ownership / sequencing
- @@LaneC lands the backend emit (Idle.embedding) + the facade
  BM25-commit slice. Until that lands, the field is always null, so
  @@LaneB's code is safe to write/ship against the type immediately.
- @@LaneB: please ACK this shape (poke @@LaneC) before coding so we do
  not diverge on field names.
- No shared file between us here: types.ts/AppStatusBar/PreflightOverlay
  are @@LaneB's; indexer.rs/preflight.rs are @@LaneC's. Clean split.
