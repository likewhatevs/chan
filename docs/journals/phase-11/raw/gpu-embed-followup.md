# Follow-up: GPU/Metal embedding path hangs (default flipped to CPU)

Status: FUTURE WORK. The default has been flipped to CPU as a
stopgap; the root cause in the Metal path is NOT fixed and needs a
proper future fix.

## Symptom

On at least one Apple Silicon Mac, the BGE embedding reindex hangs
indefinitely (observed > 180 s, never returns) inside the Metal
command buffer. `chan serve` wedges with the status pill stuck in
the embed phase and no error surfaces. Thread sample of the stuck
worker:

```
flush_embed_batch
  -> embed_documents_cancelable
  -> embed_with
  -> Tensor::to_vec2
  -> MetalStorage::to_cpu
  -> MetalDevice::wait_until_completed
  -> [_MTLCommandBuffer waitUntilCompleted]   <- never completes
```

The hang is in `[_MTLCommandBuffer waitUntilCompleted]`: the
command buffer we submit to read the forward-pass output tensor
back to the CPU never signals completion.

## Where

- File: `crates/chan-drive/src/index/embeddings.rs`
- Function: `select_device()` chooses the candle `Device`; the Metal
  backend is reached via candle's `Device::new_metal(0)`.
- The actual stall is in candle's Metal storage readback
  (`MetalStorage::to_cpu` -> `wait_until_completed`), triggered from
  `Embedder::embed_with` during `flush_embed_batch` in the indexer.

## Workaround (already known)

Forcing the CPU backend completes the embed phase normally (observed
~20 s on a 60-file drive, search returns hits). Historically this was
done with `CHAN_DISABLE_GPU=1`.

Note: one early repro run reported the CPU path also stalling, but a
subsequent fresh-binary run confirmed CPU completes cleanly. The
reliably-reproducing hang is the Metal command-buffer wait above; the
earlier CPU stall did not reproduce and was likely a stale-binary or
unrelated artifact.

## What "disable by default" changed (this commit)

In `select_device()`:

- CPU is now the DEFAULT. The GPU/accelerator path (Metal on macOS,
  CUDA on Linux + `cuda` feature) is OPT-IN via `CHAN_ENABLE_GPU=1`.
- `CHAN_DISABLE_GPU` is still accepted for back-compat. Since CPU is
  already the default it is now effectively a no-op, but honoring it
  means existing scripts / docs that set it keep working and it does
  not look unhandled.
- The GPU code path is intentionally NOT removed. It is fully intact
  behind the opt-in so a machine with a working Metal/CUDA stack can
  still benchmark or use it: `CHAN_ENABLE_GPU=1 chan serve <drive>`.

Net effect: a default `chan serve` can no longer hang out of the box
on the Metal command buffer, because it never selects the Metal
device unless the user opts in.

## Proper future fix (not done here)

Pick one (or both):

1. Bound the GPU forward pass with a timeout and fall back to CPU.
   Wrap the device readback / forward pass so that if the command
   buffer does not complete within a deadline, we abandon the Metal
   device, log a warning, and re-run the batch on CPU. This makes the
   GPU path safe to re-enable by default because a hang degrades to
   CPU instead of wedging the server. Requires care: candle's
   `wait_until_completed` is synchronous, so the timeout likely needs
   a watchdog (separate thread / `tokio::time::timeout` around a
   `spawn_blocking`) plus a way to drop the Metal device cleanly.

2. Fix the Metal command-buffer usage itself. Investigate whether the
   stall is in candle's `MetalStorage::to_cpu` submit/await sequence
   (e.g. an unsignaled or never-committed command buffer, a buffer
   reused across submissions, or an MTLCaptureManager / shared-event
   interaction in this sandbox). This may be an upstream candle issue;
   if so, reproduce minimally and file upstream, then bump the candle
   dependency once fixed.

Until one of these lands, GPU embedding stays opt-in. When it is
fixed, re-flip the default in `select_device()` (and consider
removing the `CHAN_ENABLE_GPU` gate or keeping it as a tunable).

## References

- Original flag (lane-a, phase-11):
  `docs/journals/phase-11/lane-a/journal.md` (search "waitUntilCompleted")
- Coordination ack:
  `docs/journals/phase-11/coordination/event-lane-a-architect.md`
- Decision (@@Alex): "disable the gpu path for now, and file a bug
  follow up for the future."
