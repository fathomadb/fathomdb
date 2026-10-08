---
title: Slice 135 installed Python S01 paired comparison at source 224e44c59
status: AUDITED_DIAGNOSTIC_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Installed Python S01 paired refresh at source 224e44c59

The [frozen protocol](../../s01-python-224e-comparison-protocol.json), SHA-256
`019c953091eb6a0582e64ed4e420a13e5d6d71ab7ce4b4d8f3053ef69289173e`,
preceded candidate timing. It compares the exact 0.8.26 wheel/source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` with a rebuilt wheel from
candidate `224e44c593c13d86ece648adabe445723db04070`. The candidate wheel
SHA-256 is `ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`.
The unchanged corpus, model, query shapes, timing boundary and five-pair
alternation match the earlier [current-wheel diagnostic](../2026-10-08-python-s01-current-paired/README.md).

All 20 blocks passed the runner and [independent raw/order/resource
audit](independent-audit.json): 60,120 checked materialized calls, 1,000 warm
samples per query cell and block, and no semantic failures. Four blocks have
host-only swap-counter warnings; no measured child had a swap event or major
fault. The [negative controls](negative-controls.json) rejected a changed
text result, a reordered block and a measured-child swap event.

| Corpus | Median five-pair p50 change: text | Vector | Hybrid |
| --- | ---: | ---: | ---: |
| 32 rows | -2.974% | -3.425% | -2.023% |
| 256 rows | +1.863% | -0.045% | -0.456% |

The independent audit retains every p50/p95/p99 pair delta and full observed
range. There are four warning-free pairs at 32 rows and three at 256 rows.
Median child peak RSS was 451,920/364,720 KiB for baseline/candidate at 32
rows and 456,832/371,200 KiB at 256 rows; these are whole-worker resources,
not per-query allocations. The results are workload-specific diagnostics,
not a statistical equivalence or release verdict.

The copied [raw archive](raw-archive/) currently remains untracked pending
end-of-phase retention. Its 248-file `SHA256SUMS` manifest has SHA-256
`3c5d7658544fab0d05956f851215498c7d9732a8259aaa1d57bc70e45c8cd49a`;
all copied bytes passed `sha256sum -c`. The tracked audit SHA-256 is
`fcee812c132333c8d8a6d235912d1aacdf9c9cbade910fc7f694fe92a55caad2`.
The final Phase 1 checkpoint still needs the broader operation mix, other
installed SDK boundaries, robustness and coverage matrices, and a final
exact-candidate disposition of any later source changes.
