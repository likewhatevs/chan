// Progress events for long-running drive operations.
//
// One umbrella shape used by `Drive::reindex_with`, the rename +
// link-rewrite path, `import_contacts_with`, `Library::reset_drive_with`,
// and the embedder model load. Consumers (chan-server's WebSocket
// fan-out, the CLI's progress bar, future native shells) build on
// `ProgressCallback` so a single sink handles every long-running op
// instead of one bespoke callback shape per surface.
//
// Design constraints:
//   * Owned String fields, no lifetimes. The public API must survive
//     the uniffi boundary later; foreign code can't hold borrowed
//     references across an FFI call.
//   * `ProgressCallback: Send + Sync`. Some ops fire from the embed
//     batch worker or from inside the graph rebuild, so the sink
//     can't assume single-threaded access. Foreign objects come in
//     as `Arc<dyn ProgressCallback>` via uniffi; deref to the &dyn
//     form the methods accept.
//   * Events are best-effort hints, not a stream contract. Dropping
//     one because the consumer is slow is fine; the on-disk state
//     is the authority. Implementations must not block (any I/O
//     work belongs on a separate worker that drains a channel).
//
// Cardinality budget per stage so consumers can sanity-check their
// progress UI without instrumenting the producer:
//
//   GraphRebuild    one per editable-text file walked
//   IndexFile       one per file before the BM25 enqueue
//   EmbedBatch      one per cross-file embedding flush
//   RenameRewrite   one per source file rewritten
//   Import          one per contact written
//   Reset           one per subsystem wiped
//   ModelLoad       one per phase boundary (resolve, download, load)
//   Heartbeat       sparse; used to keep the bar moving during
//                   silent internal phases (orphan-shard cleanup,
//                   trash sweeps)

use std::sync::Arc;

/// Which long-running operation the event belongs to. Kept narrow so
/// a UI can switch on it to pick a label / icon without parsing the
/// `label` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStage {
    /// Walking the drive and rebuilding the graph in memory before
    /// the single-tx commit. Long on large drives but rarely the
    /// bottleneck; surfaced so the UI knows the reindex has started.
    GraphRebuild,
    /// Per-file step of the search-index build: read + chunk + BM25
    /// enqueue. Dense vectors are emitted under `EmbedBatch`.
    IndexFile,
    /// Cross-file embedding flush. `current` carries the chunks in
    /// this batch; `label` is the file that pushed the buffer past
    /// the batch threshold.
    EmbedBatch,
    /// Rename + link-rewrite, one event per source file the
    /// rewriter touches.
    RenameRewrite,
    /// Contacts import, one event per contact written / overwritten
    /// / skipped.
    Import,
    /// `Library::reset_drive` wiping a subsystem (index, graph,
    /// sessions, ...). `label` carries the subsystem name.
    Reset,
    /// Embedding-model open path. Emitted at phase boundaries:
    /// resolving the cache dir, downloading from HuggingFace,
    /// loading weights into candle. `current` / `total` are bytes
    /// when known, otherwise 0.
    ModelLoad,
    /// Sparse "still here" tick for long internal phases that don't
    /// map onto the other stages (vector orphan cleanup, trash
    /// sweeps on large trees). Carries a `label` describing the
    /// phase; `current` / `total` may both be 0.
    Heartbeat,
}

/// One progress tick. Plain data so the uniffi wrapper can serialize
/// it without round-tripping through a trait. `current` / `total`
/// are domain-specific (see `ProgressStage`); the consumer treats
/// `total == 0` as "indeterminate" and renders a spinner instead of
/// a percentage.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub stage: ProgressStage,
    pub current: u64,
    pub total: u64,
    /// Human-readable label for the current item: a file path during
    /// IndexFile / RenameRewrite, a contact name during Import, a
    /// subsystem name during Reset, a free-form phase tag during
    /// Heartbeat. `None` when the producer has nothing useful to
    /// show; consumers should fall back to the stage name.
    pub label: Option<String>,
}

/// Sink for `ProgressEvent`s. `Send + Sync` so producers running on
/// worker threads (embed batch flush) can call into the same sink as
/// the main thread. Foreign-language implementations cross the FFI
/// boundary as `Arc<dyn ProgressCallback>`; in-process Rust callers
/// usually build one via `progress_fn` or use `NoProgress`.
pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, event: ProgressEvent);
}

/// Zero-cost "discard every event" sink. Used by `Drive::reindex` /
/// `Drive::rename_with_link_rewrite` / etc., which keep the no-arg
/// API alive by delegating to the `_with` overload with this sink.
pub struct NoProgress;

impl ProgressCallback for NoProgress {
    fn on_progress(&self, _event: ProgressEvent) {}
}

/// Wrap a Rust closure as a `ProgressCallback`. The closure must be
/// `Fn + Send + Sync` because progress events fire from arbitrary
/// worker threads inside the producers. Use this from the CLI and
/// chan-server entry points; foreign-language shells will pass an
/// `Arc<dyn ProgressCallback>` directly and skip this helper.
pub fn progress_fn<F>(f: F) -> Arc<dyn ProgressCallback>
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    struct FnAdapter<F>(F);
    impl<F> ProgressCallback for FnAdapter<F>
    where
        F: Fn(ProgressEvent) + Send + Sync,
    {
        fn on_progress(&self, e: ProgressEvent) {
            (self.0)(e);
        }
    }
    Arc::new(FnAdapter(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_progress_is_a_silent_sink() {
        // A consumer that wants zero progress overhead passes
        // `&NoProgress`. Calling on_progress on it must not panic
        // and must drop the event; this is the no-arg API's escape
        // hatch and it cannot regress.
        let np = NoProgress;
        np.on_progress(ProgressEvent {
            stage: ProgressStage::Heartbeat,
            current: 0,
            total: 0,
            label: None,
        });
    }

    #[test]
    fn progress_fn_dispatches_to_closure() {
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cb = {
            let count = count.clone();
            progress_fn(move |_e| {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
        };
        cb.on_progress(ProgressEvent {
            stage: ProgressStage::IndexFile,
            current: 1,
            total: 10,
            label: Some("a.md".into()),
        });
        cb.on_progress(ProgressEvent {
            stage: ProgressStage::IndexFile,
            current: 2,
            total: 10,
            label: Some("b.md".into()),
        });
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
