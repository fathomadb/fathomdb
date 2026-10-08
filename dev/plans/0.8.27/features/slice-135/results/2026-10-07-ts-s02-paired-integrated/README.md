# Installed TypeScript S02 paired diagnostic — 2026-10-07

**Status:** independently audited, exact-source diagnostic. The
[frozen subset](../../s02-ts-comparison-protocol.json) SHA-256 is
`63e060aca9d9236665875b9c6deb6a9d82fd7ecac6b7818c9d6a6db27d8bf6cc`;
it was written after the [baseline-only pilot](../2026-10-07-ts-s02-baseline-controlled/README.md)
and before paired candidate timing. The baseline source is
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the post-Slice-132
candidate source is `cdf253cd223a82e954591db532397a3d78a2027a`, which
includes both recorded off-ladder fixes. Retained main/platform npm archives
and installed package/native bytes were bound by SHA-256 for each role. The
package metadata still says 0.8.26, so that label alone is not the candidate
identity.

The campaign receipt (`run-manifest.json`) and execution order (`run-order.jsonl`)
contain five alternating pairs and ten valid blocks. Each block has one
fresh-process warm-up and 20 measured sequences. All **100 measured sequences
per version** passed materialized text/vector/hybrid, graph/evidence,
erasure and reopened-state checks, with direct SQLite canonical-row validation
after the product timer. Every raw sequence, command, child resource report,
host snapshot and output stream is retained under `pair-01-*` through
`pair-05-*`. No valid slow sample was removed.

The independent audit (`independent-audit.json`) rehashed every raw sample,
reran a separate state oracle, checked the archives and command order, and
recomputed the statistics:

| Whole installed-SDK sequence | 0.8.26 baseline | 0.8.27 candidate | Candidate delta |
| --- | ---: | ---: | ---: |
| Pooled nearest-rank p50 | 4,943.808 ms | 5,085.706 ms | +2.870% |
| Pooled nearest-rank p95 | 5,182.480 ms | 5,169.239 ms | −0.256% |

The five preassigned pair p50 deltas were +2.968%, +3.339%, +2.601%, +2.685%
and +3.667%; their median was +2.968% and range +2.601% to +3.667%. Every
pair had host-only swap-counter movement, while all measured child processes
reported zero swap events. There are **zero warning-free pairs**, so a
warning-free sensitivity estimate is unavailable. P99 is unsupported with
100 samples per version. Child peak RSS reached 479,964 KiB on baseline and
345,904 KiB on candidate; this whole-process maximum is not a post-close
memory-release measurement.

The negative controls (`negative-controls.json`) show the auditor rejecting a
surviving canonical edge, a changed native artifact identity and a swapped
execution-order entry. This is a TypeScript whole-sequence latency lead, not
an equivalence, significance or release verdict. Python S02 and engine
E01–E12 use different call boundaries and workload shapes. Rust S02 timing,
bounded contention, S03, C01 and the other Phase 1 matrices remain open.
`SHA256SUMS` binds the local archive; raw retention remains local pending the
Slice 135 final evidence step.
