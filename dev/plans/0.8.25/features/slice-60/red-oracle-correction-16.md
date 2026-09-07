---
title: Slice 60 verification FIX-10 RED oracle correction 16
status: RED
---

# Slice 60 verification FIX-10 RED oracle correction 16

The terminating parallel reporter exposed a test-only Slice 55 attribution
oracle that rejected a valid dispatcher read snapshot overlapping the
deliberately paused projection-worker write transaction. Production records
every active owned role and classifies either role as
`owned_runtime_transaction`; suppressing the dispatcher would make the
diagnostic false.

The RED replaces the accidental exact-vector comparison with a named predicate
while deliberately retaining its old worker-only implementation. A new
deterministic test freezes the intended shape: exactly the selected projection
worker, with at most `ProjectionDispatcher:0`; another worker, writer, or reader
must reject. The original witness also uses the predicate for its initial and
five checkpoint snapshots, so GREEN cannot bypass the real failure path.

| Path | Pre-RED SHA-256 | RED SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/src/lib.rs` | `66c7666c94c969905696d46ad30148508daf29c03293e0c7c555fe70c8b055fb` | `944783268f7e1f3181c43ce591255ed1fe871e8de8b75bf3d5e870c4a1486e1a` |

The exact RED command was:

```text
timeout --signal=TERM --kill-after=10s 180s \
  cargo test -p fathomdb-engine --lib \
  tests::projection_worker_attribution_accepts_concurrent_dispatcher_snapshot \
  -- --exact --nocapture
```

It failed in 6.4 seconds with exit 101 on the concurrent-dispatcher assertion:

```text
assertion failed: projection_worker_attribution_matches(
    &[(WalAttributionRole::ProjectionDispatcher, 0), paused_worker],
    paused_worker,
)
test result: FAILED. 0 passed; 1 failed; 57 filtered out
```

GREEN is limited to the predicate body. The test and the original witness are
frozen through that correction.
