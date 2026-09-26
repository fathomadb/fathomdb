---
title: FathomDB 0.8.27 Slice 70 - design review
status: PASS
target_release: 0.8.27
---

# Slice 70 design review

The reviewer is an independent, read-only, adversarial subagent (Opus 5.5,
high effort).

## Cycle 1 — FAIL at `a95b5b0f` (text-only findings)

The architecture stood. Each finding was verified against the code and closed
in the design and plan revision:

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P1 | `check-c1-conformance.sh` probes `lib.rs` for moved owners, and its self-test edits `lib.rs` text. | Slice 40-style RED/GREEN retarget, with per-module path constants and fixture paths. The pin JSON is unchanged. |
| 2 | P2 | The seq 257 audit fix also needs the stale `delete_vector_partition_row` helper-caller entry removed. | Removed in the same commit. |
| 3 | P2 | Three batches exceeded 1,200 lines. | Ten batches of about 500-955 lines. |
| 4 | P2 | The classifier oracle was ambiguous for a failed, unenrolled edge. | The specific failed row wins, with its reasoning recorded. Edge and node kinds are named, and the corruption reason is asserted. |
| 5 | P2 | The zero-residue audit clause was vacuous with a working embedder. | A failed-outcome arm with an always-failing embedder. A concrete mutant. The file is gated `debug_assertions` only. |
| 6 | P2 | Missing `cfg` routes (`ce_rerank` gates on `tc5-benchmark` / `slice72-test-hooks`); focused-owner routes were unnamed. | Combined routes were added. Each owner's route is named, and a zero-test route counts as a failure. |
| 7 | P3 | `default_embedder_identity`, the edge-vector prune, and `projection_status` were misassigned; `branch_str` and `append_jsonl` were unlisted. | Kept at root and listed. `drain_embedder_events` stays at root. |
| 8 | P3 | `mean_centering_internals_for_test` is a test seam, and the removal gate was named wrongly. | It stays at root with `MEAN_VEC_PIN_THRESHOLD`. AC-050c runs against the pre-move base. |
| 9 | P3 | The slice35 manifest `function_body` can go vacuously green after a visibility change. | Per-batch byte-identity check of the extracted bodies. |
| 10 | P3 | Stale plan text and baselines; the `plan-0.8.20.md` anchors were not named. | Corrected and named. |

## Cycle 2 — PASS-WITH-FIXES at `b33e8091`

Every cycle-1 resolution was verified against the code. The reviewer
confirmed:

- **C1 retarget:** owner-to-batch mapping and `--list-sources` fixture
  derivation.
- **Batches:** all ten are within 300-1,200 lines and compile-feasible.
- **Classifier oracle:** it equals `classify_completion`, and both mutants
  are killed.
- **Failed-outcome arm:** the audit row and the `failed` terminal are written
  on the transaction that the forced failure drops.

Four P3 text findings were closed without a further review cycle:

- the node-scoped precedence quote was removed;
- the `mean.rs` dependency direction and the method visibility were stated;
- the failed-outcome arm now checks recovery; and
- the pre-move receipt and baseline wording were aligned.

Implementation may start.
