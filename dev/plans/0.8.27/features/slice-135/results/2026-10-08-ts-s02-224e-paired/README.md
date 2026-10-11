---
title: Slice 135 exact-candidate installed TypeScript S02 paired comparison
status: AUDITED_SUBSET_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Installed TypeScript S02 paired comparison — 2026-10-08

The [frozen protocol](../../s02-ts-224e-comparison-protocol.json), SHA-256
`5d52c0a6be26936afd24bfe5917b4ba05535c1f3539abfeb7dd465b91690a75a`,
preceded candidate timing. It binds the exact baseline source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and candidate source
`224e44c593c13d86ece648adabe445723db04070` with installed npm archive
and native-addon bytes. Each fresh-process sequence opens the real database,
writes and projects 32 rows, retrieves text/vector/hybrid and graph/evidence
results, erases, closes, reopens and checks persisted state. The product timer
includes SDK state reads and materialization; direct SQLite validation is
outside it. One warm-up and 20 measured sequences per block, five alternating
pairs and 20-second minimum idle gaps were frozen after the baseline pilot.

All **10/10 blocks** and **200/200 measured sequences** passed the
[independent raw, resource and reopened-state audit](independent-audit.json).
The [order audit](order-audit.json) checked every block and all nine idle
gaps; the minimum was 20.000786 seconds. The
[negative controls](negative-controls.json) rejected an erased source
surviving reopen, SQLite validation entering the timed path, changed block
order and a shortened idle gap. No valid slow sample was discarded.

| Whole installed TypeScript sequence | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Pooled nearest-rank p50, 100 samples per version | 5,018.054 ms | 5,191.387 ms | +3.454% |
| Pooled nearest-rank p95, 100 samples per version | 5,165.872 ms | 5,296.755 ms | +2.534% |

The five preassigned pair p50 changes were +3.283%, +3.173%, +3.675%,
+3.618% and +2.772%; median +3.283%, range +2.772% to +3.675%.
Three pairs had host-only paging or child-major-fault warnings, leaving two
warning-free pairs with +3.283% and +3.173% p50 changes. No measured child
swapped. The two clean pairs are too few for a separate inference. Candidate
peak child RSS reached 343,808 KiB versus 477,360 KiB baseline; these are
whole-process peaks, not post-close retention. **p99 is unsupported** with
100 samples per version. Five pairs support a descriptive lead, not a
significance, equivalence or release verdict.

The [stage diagnostic](stage-diagnostic.json) reports separately calculated
medians over the 100 sequences per version. Reopened open increased by
158.891 ms, `close()` by 8.029 ms and reopened `close()` by 5.160 ms; vector
and hybrid retrieval rose by 0.588 and 0.562 ms. Marginal stage medians do
not add to a whole-sequence percentile or prove causality. This renewed
TypeScript S02 lead contrasts with the [exact Python S02](../2026-10-08-python-s02-224e-paired/README.md)
pooled p50 change of +0.287% at its separately measured installed boundary.

Audit, order, negative-control, stage and run-order SHA-256 values are
`e73d4d1c9e66df8805ca96fd847f98ab9c067ad52ccc6950322fa7f8d4fc0a3e`,
`905389577aa36b74282f7bbf5209149336845abf3a1a9bc4965e37ed7c09b255`,
`e62b9d51b94a933b3bf7a15ce6800c7b1183d10123e99feb3c32ea911f4a59ed`,
`0a0c9471fa193e45933a5e084db26a4ddae2f9ad7475d71457d2b4515acd9d8e`
and `27e9c24dea14c26fd5dc43cbaec498ac03147a9e879ff37e9592a0879440397a`.
The copied raw archive (`raw-archive/`) retains sequences, environment
records, SQLite databases, commands and resource reports. Its 1,122-file
`SHA256SUMS` manifest has SHA-256
`bef3056ceda8c4a84a995392b80a949da0c01c2221611bfd43196577a951822e`;
all copied bytes were verified. The raw archive remains untracked pending
end-of-phase retention; the linked audits are tracked. This S02 subset does not close the broader Pareto,
robustness or logic matrices.
