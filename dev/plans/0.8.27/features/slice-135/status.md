---
title: FathomDB 0.8.27 Slice 135 — final system qualification disposition
status: COMPLETE_WITH_EXPLICIT_LIMITATIONS
target_release: 0.8.27
product_source: 224e44c593c13d86ece648adabe445723db04070
baseline_source: f99e002f0d2e4002f3694c9f8d4986b56089edaa
---

# Slice 135 final disposition — 2026-10-08

Slice 135's five-area diagnostic inquiry is complete for the recorded source,
artifacts, host and workloads. The [Phase 1 checkpoint](phase1-checkpoint-2026-10-08.md)
covers Pareto paths, system latency, robustness, and logic/exception handling.
The [Phase 2 report](phase2-query-correctness-report-2026-10-08.md) covers all
six correct-results dimensions with independently audited scores or an
evidence-backed omission. The subsequent [CPU–CUDA sample](phase2-gpu-sample-2026-10-08.md)
checks 611 frozen candidate queries. This is a completed measurement and
disposition slice, not a finding that all paths improved or that the final
0.8.27 release is qualified. Slice 150 owns integrated final-candidate
qualification and the accepted release gates remain in force.

The 0.8.26 baseline source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`.
The measured post-Slice-132 candidate product source is
`224e44c593c13d86ece648adabe445723db04070`, including both off-ladder
landings. Current product sources under `src/rust/crates/`,
`src/python/fathomdb/` and `src/ts/src/` have no diff from that source.
The linked reports pin installed artifact hashes, model revision, frozen
protocols, corpora and environment. Later changes add measurement machinery,
tests, audits and documents; they do not change the measured product.

## Verdict by area

| Area | Finding | Limit or disposition |
| --- | --- | --- |
| Pareto path | Four of ten equal-frequency Python query paths accounted for more than 80% of measured elapsed cost at both corpus sizes; selected tests reached every workload-hit branch ID in the E01–E12 overlay. | This is a proxy, not a production-traffic ranking or per-path CPU/queue attribution. |
| System latency | Engine vector, hybrid and populated-open median-pair p50 increased 15.67%, 14.54% and 12.14%; installed TypeScript S02 pooled p50 increased 3.454%. Python S02 whole-sequence p50 increased 0.287%, while close p50 increased 143.722% alongside intentional model-memory release. | These are workload-specific descriptive leads, not an equivalence finding or new release threshold. Preserve the close memory-release contract and investigate the leads under a reviewed follow-up. |
| Robustness and logic | Selected real-database concurrency, recovery, provider, projection, erasure and binding cases passed. Current-schema swallowed SQL row errors were repaired test-first and passed exact-source replay. | OS permissions, in-commit erasure/WAL, all exported FFI panic paths, sustained mixed load and some platform/provider variants remain unsupported. |
| Query correctness | Deterministic contract cases passed both versions. Judged IR scores were equal by class; MuSiQue and LOCOMO support-set hit lists matched across versions. Vector exact-f32 Recall@10 was 0.933 baseline / 0.931 candidate, with differing index mean/layout state and no established product-code cause. | No confirmed candidate-specific deterministic wrong result. Absolute evidence gaps and one required-document rank 2-to-3 change remain diagnosed. Generated-answer quality is unsupported because no qualified answerer and spending setup was available. |
| Candidate CPU–CUDA sample | All 611 frozen queries had the same ordered hit lists and gold scores on CPU and RTX 3090 CUDA. | Qualifies only this installed Python x86_64 candidate route and sample; no baseline CUDA, Jetson, release artifact, other model or answer claim. |

The measured slowdowns and absolute retrieval gaps warrant follow-up but do
not cross a pre-existing numerical release gate. The Phase 2 vector and rank
differences are recorded as unresolved diagnostics, not silently called
equality or repaired without a demonstrated defect. If a future product
change addresses any lead, repeat its affected paired cells and shared-system
checks at the new source identity. Slice 140 may use this disposition for
documentation convergence; Slice 150 must independently qualify its final
integrated candidate.

## Follow-up placement

1. Profile installed TypeScript S02 reopened-open and engine
   vector/hybrid/populated-open with matched timer, stage and resource
   boundaries. Investigate Python close cost while retaining model-memory
   release. Add per-path CPU/queue and sustained-load measurements only under
   a reviewed instrumentation protocol.
2. Qualify the missing OS-permission, in-commit erasure/WAL, close/cancellation,
   release-binding panic, Mem0, Rust baseline, provider/model and platform
   cells before making claims about them. The sampled GPU query result narrows
   only the named candidate retrieval cell.
3. For any version-specific vector-fidelity claim, equalize or control index
   mean/layout state and repeat the paired study. Investigate the shared
   QMSum, MuSiQue four-hop and LOCOMO strict multi-session evidence gaps as
   retrieval-quality work, not as Slice 135 candidate regressions.
4. Choose and qualify an external answerer/judge protocol and its spending
   controls before scoring generated answers. Retrieval and evidence
   availability are not answer accuracy or citation faithfulness.

## Verification and raw retention

The Phase 1 and Phase 2 reports link the failing-first tests, independent
audits and exact-source full workspace gates. After the executable Phase 2
changes, `./scripts/agent-verify.sh` passed 186/186 suites, with no skips or
exclusions and zero reported security violations. The GPU sample has its
own frozen manifest, device/installed-wheel checks, independent score
recomputation and a retained invalid first vector probe. Final documentation
changes receive the scoped Markdown and release-state validators.
The [independent closeout review](closeout-review-2026-10-08.md) approved
the diagnostic disposition with the named limits and scoped review coverage.

The [retention transfer](raw-retention-transfer-2026-10-08.md) records
worktree-external local copies of the Phase 1, Phase 2 and GPU raw archives,
their SHA-256 values and successful decompression tests. The Phase 2 source
payload store remains separate and pinned by its protocols. These licensed
or derived raw materials remain outside Git. Local preservation does not
constitute public distribution or off-host backup; keep the feature worktree
until any desired additional storage route is settled.
