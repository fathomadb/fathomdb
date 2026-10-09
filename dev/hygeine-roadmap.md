---
title: FathomDB product hygiene roadmap
status: ACTIVE
scope: product-wide
updated: 2026-10-09
---

# Product hygiene roadmap

This is a product-wide register for correctness, quality, performance and
qualification risks. It is not a Slice 135 plan or a single-release checklist.
Slice 135 supplied the first evidence-backed intake; later product evidence
may change priorities or close entries. Release assignments are recorded as
current scheduling, while implementation authority remains with each release
plan and its accepted gates.

## Open risk register

Initial intake recorded 2026-10-09 from the [final Slice 135 disposition](plans/0.8.27/features/slice-135/status.md),
[Phase 1 checkpoint](plans/0.8.27/features/slice-135/phase1-checkpoint-2026-10-08.md)
and [Phase 2 report](plans/0.8.27/features/slice-135/phase2-query-correctness-report-2026-10-08.md).
Ranks are investigation priorities based on potential harm and available
evidence, not confirmed open-bug severities. The confirmed Slice 135 SQL
row-error defects were repaired; no candidate-specific deterministic wrong
result was found in the supported query cells. **Critical: none established.**

| Rank | Priority | Product risk and initial evidence | Current scheduling |
| ---: | --- | --- | --- |
| 1 | High | **Durable erasure under interruption.** An interruption inside a transaction or WAL checkpoint is untested; a violation could affect durable erasure or truthful completion. | **0.8.28 planned scope D28-09** for deeper fault qualification; a confirmed violation of an accepted 0.8.27 gate must be repaired before 0.8.27 publication. |
| 2 | High | **Multi-hop evidence reachability.** MuSiQue complete support was 36/100 at rank 20, including 1/25 four-hop cases, on both measured versions. | **0.8.28 planned scope D28-10** for qualified multi-hop improvement work. |
| 3 | High | **Cross-platform installed-artifact parity.** The 611-query CPU–CUDA match did not qualify Jetson or the final release artifact route. | **0.8.28 planned scope D28-11** for broader installed-artifact/platform parity; Slice 117 and Slice 150 retain the 0.8.27 delivery and release checks. |
| 4 | Medium | **FFI panic containment.** Representative panic tests and `catch_unwind` wrappers do not dynamically prove containment across every release-built Python and Node export. | Unscheduled; qualify the remaining exported paths before claiming full containment. |
| 5 | Medium | **Shutdown reliability and cost.** Close/cancellation under sustained contention remains unqualified; Python `close()` added about 7.8 ms while intentionally releasing model memory. | Unscheduled; preserve memory release while testing shutdown and attribution. |
| 6 | Medium | **Exploratory and multi-session retrieval.** QMSum exploratory required-document Recall@10 was 0.4249 on both versions; LOCOMO strict multi-session complete support was 215/269. | Unscheduled retrieval-quality investigation by query class. |
| 7 | Medium | **Query and open latency attribution.** Engine vector, hybrid and populated-open median-pair p50 rose about 12–16%; the engine stage cannot alone establish whole-call loss. | **0.8.28 planned scope D28-12** for matched attribution and conditional remediation. |
| 8 | Medium | **Installed TypeScript lifecycle latency.** S02 whole-sequence p50 rose 3.454%; reopened-open is a stage lead requiring controlled attribution. | Unscheduled performance investigation. |
| 9 | Medium | **Permission and sustained-load qualification.** OS permission changes, per-path CPU/queue time and sustained mixed-load throughput were not qualified. | Unscheduled fault and measurement work before those claims. |
| 10 | Low | **Vector and ranking fidelity.** Exact-f32 Recall@10 was 0.933 baseline versus 0.931 candidate with differing index states; one required document moved from rank 2 to 3 without leaving top 10. | Control index mean/layout before attributing a version regression. |
| 11 | Low | **Exceptional fallback truthfulness.** A poisoned lock can lose a precise refusal reason; legacy row-flattening and exceptional allocation/panic paths remain bounded concerns without a supported-path failure. | Re-audit if schema admission or these paths change. |
| 12 | Low | **Comparator completeness.** Matched Mem0 speed, a same-SDK Rust baseline, some provider/model cells and external generated-answer quality lack qualified runs. | Qualify when a release or product claim depends on them. |

**Evidence retention:** the three raw archives have verified copies outside the
feature worktree but only on the same host. An off-host backup or reviewed
publication route remains open before worktree removal; see the
[transfer record](plans/0.8.27/features/slice-135/raw-retention-transfer-2026-10-08.md).
The 0.8.28 placements above amend its [draft scope](plans/0.8.28-draft-scope.md)
without authorizing implementation. Other rows retain the stated priority
and remain unscheduled unless a later decision places them.
