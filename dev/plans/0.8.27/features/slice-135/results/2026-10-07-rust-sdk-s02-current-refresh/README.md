---
title: Slice 135 repaired-source Rust SDK S02 functional refresh
status: CANDIDATE_ONLY_FUNCTIONAL_NO_LATENCY_VERDICT
target_release: 0.8.27
---

# Repaired-source Rust SDK S02 functional refresh — 2026-10-07

The existing external Cargo [consumer source](main.rs) was rebuilt against
the Slice 135 Rust tree at product source
`3f29d649d0213e595c0dab251a449d92fd625792`, after the vector row-error
repair. Both staged and unstaged Rust-tree diffs from that commit were empty
at build time. The [external manifest](Cargo.toml.snapshot),
[lockfile](Cargo.lock), [build stderr](build.stderr),
[run metadata](run.json) and [binary hash](binary-sha256.txt) identify the
source-bound build. The build log records fresh compilations of
`fathomdb-engine`, `fathomdb-sdk` and the external consumer. This is a path
dependency, not a published Rust crate; 0.8.26 has no Rust SDK peer.

The executable ran once against a fresh [real database](consumer.sqlite).
The consumer asserts open, write, projection readiness, text/vector/hybrid
search, typed invalid-input errors, frozen evidence and graph evidence,
erasure, close and reopen; [stdout](stdout.log) records its final success
marker. The [process resource report](resource.txt)
records exit 0, 0 swaps and a 292,336 KiB peak RSS. Its 5.80 s whole-process
wall time includes setup and assertions; it is not a qualified S02 latency
sample or a paired 0.8.26 comparison.

An independent read of the retained database found 32 corpus nodes and no
graph nodes or edges after reopen; `PRAGMA integrity_check` returned `ok`.
The [independent-state record](independent-state.json) contains these values.
The [negative-control record](negative-control.json) shows that a hypothetical
retained graph edge differs from the expected final state. It is a comparison
control, not a separate fault-injection run. The [SHA-256 manifest](SHA256SUMS)
covers the raw inputs and observations; run `sha256sum -c SHA256SUMS` from
this directory to verify the retained bytes. The direct state query can be
recomputed from `consumer.sqlite` with any SQLite reader.

This refresh demonstrates functional continuity for one candidate-only Rust
SDK S02 sequence after the repair. It does not close Rust S02 timing, bounded
contention, published-artifact qualification, the operation-condition matrix,
or the full Phase 1 checkpoint.
