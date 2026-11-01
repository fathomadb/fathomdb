---
title: FathomDB 0.8.28 Slice 60 — requirements and acceptance
status: draft; awaiting Astra design review
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 60 requirements and acceptance

Identifiers below are release-local; `dev/acceptance.md` stays locked.

| Requirement | Acceptance |
| --- | --- |
| REQ60-01: fresh stores use format 35; prior stores have no automatic upgrade | AC60-01: version-34 main-only and WAL-effective stores return `IncompatibleSchemaVersion { seen: 34, supported: 35 }`; canonical data and sidecars remain unchanged; fresh create and current reopen succeed |
| REQ60-02: every vec0 creation path uses the qualified chunk layout | AC60-02: fresh, rebuilt, attribute-reshaped and reopened partitions have `chunk_size=64`; malformed/current old-chunk shape is rejected, never silently upgraded; dimension and attribute contracts remain intact |
| REQ60-03: node body FTS identity is its write cursor | AC60-03: ordinary writes, batched writes, update/reprojection and rebuild produce `search_index.rowid = write_cursor`, including cursor gaps and mixed nodes/edges; text results and ranking stay unchanged |
| REQ60-04: per-vector coverage has a stable record/window identity | AC60-04: span table admits multiple vector IDs per record with distinct windows, rejects duplicate record/window and invalid ranges, indexes record identity; only window zero is emitted today |
| REQ60-05: coverage is truthful UTF-8 byte coverage | AC60-05: unknown is SQL NULL, full coverage is body byte length, truncated coverage is an exclusive UTF-8 boundary from the embedder; short, empty, long and Unicode inputs are covered; invalid provider offsets are rejected |
| REQ60-06: projection coverage and vectors commit atomically | AC60-06: scalar and batched node/edge projection create exactly one span per vector; failure, cancellation and stale generation create none; delete/excise/rebuild remove stale spans; recenter and attribute reshape preserve spans and f32 vectors |
| REQ60-09: span completeness is part of the physical projection contract | AC60-09: scalar/bulk/empty checks, restore, reopen, readiness and integrity reject missing, mismatched or orphan spans; custom durable/TEMP span triggers remain active and canonical mutations invalidate frozen reads |
| REQ60-10: the public trait follows independent Axis E versioning | AC60-10: embedder-api advances 0.7.0 to 0.8.0 with dependency/lockfile consistency; default unknown and wrapper forwarding compile and test |
| REQ60-07: public results and recall remain sound | AC60-07: clean-store result comparison preserves f32 scores and results absent cutoff ties; tie differences are classified at the Hamming cutoff; real BGE eu7 passes its existing recall criterion, ac_013b is report-only |
| REQ60-08: qualify the layout's latency and scale costs | AC60-08: existing AC-013 passes 80 ms p50 / 300 ms p99 at the gate's prescribed size; vector-only 64/128/256 cells reach 2M rows and report 5.1x record-count equivalents under the plan's fixed decision rule |

No overflow vectors, native eligibility metadata, historical dense semantics,
search view flags, snippet reporting, alternative rerank storage, cache-size
increase or new public configuration is included. Those remain in their
assigned slices. Coverage is metadata, never a second embedding identity.
