---
title: Slice 160 evidence composition design
status: PROPOSED
target_release: 0.8.29
---

# Slice 160 evidence composition design

[Active owner dispositions](requirements.md#owner-dispositions) select .29
identity/atomic-feedback/source-seam work, hold real resets to .30 and move new
x86 CUDA13 delivery/quantitative evidence to .31. Seq-381 additionally moves Tegra CUDA13
130A to .31;130D/E and existing CUDA12.6/platform/artifact obligations remain .29; deferral is not qualification. Owner seq-387 excludes non-Tegra ARM+GPU B/C from every .29 gate and known-working environment claim.

## Boundary

This is release qualification and bounded confirmed-defect repair. It introduces
no new product subsystem, storage format, API generation or evidence framework.
Use existing gates and consumer harnesses. The [matrix](qualification.md) is an
execution inventory, not an acceptance oracle. The parent imports reviewed own
candidate files; state/generated boards and ledgers retain their existing writer.

Sources inspected: [direct execution](../../direct-execution.md), [master](../../../plan-0.8.29.md),
[reconciliation](../../reconciliation-plan.md), [traceability](../../prework/slice-5/traceability.md),
[active architecture](../../../../design/fathomdb-data-plane-architecture-v2.md),
[release mechanics](../../../../design/release.md), [.28 qualification](../../../0.8.28/features/slice-95/qualification.md)
and its later [.28 status](../../../0.8.28/features/slice-95/status.md).
Accepted [Tier1](../../../../adr/ADR-0.6.0-tier1-ci-platforms.md),
[CUDA support](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md),
[driverless linkage](../../../../adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md),
[absolute reads](../../../../adr/ADR-0.8.25-absolute-read-performance-successor.md)
and [format37 visibility](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
constrain execution. Final accepted50/70/80/110/120 contracts must be reread when
actually integrated; their draft approvals cannot supply runtime qualification.

## Evidence applicability

For each existing observation compare only relevant executable inputs: source/
lock/fork pins, feature flags, compiler/toolchain, installed artifact bytes,
fixture/query/gold/model/tokenizer pins, configuration and performance host/device
conditions. Retain command, real exit, scope and raw log location. A Git SHA alone
neither proves equivalence nor automatically invalidates byte-identical inputs.
Use one compact comparison per affected evidence group; no new receipt validator.

| Delta | Consequence |
| --- | --- |
| Plan/status/release-state prose only | Scoped Markdown/contracts/state validation; runtime evidence remains its original observation. |
| Renderer/degradation/B2 carrier | Recheck both formats, final bytes/accounting, relevant SDK fields, paging/errors and smoke using that output; unrelated erasure controls may remain applicable. |
| Fusion/eligibility/query/queue/provider behavior | Refresh affected independent result, latency/stress/fidelity/smoke and lifecycle paths; pre-change numbers remain baseline only. |
| Compiler/features/fork/ABI/native packaging | Rebuild and qualify affected native bytes/platform/package consumers. Changed source archive cannot inherit external-consumer identity. |
| Model/query/corpus/gold change | Prior measurement is not comparable; do not substitute inputs. Resolve frozen-fixture access or return a real missing-input hold. |
| Broad runtime fix after final phase | Reassess the affected closure and broad verification need. Narrow failed-suite corrections get scoped reruns; preserve the original failed full run. |

AC072/075/076 have once-per-release measurement obligations. Reuse an applicable
actual .29 measurement from30 or the Block1 exit instead of repeating it in160;
.28 numbers alone do not fulfill that fresh .29 obligation. Final feature/runtime
campaigns likewise need an applicable actual .29 run. Existing unchanged fault
controls can discharge their mapped correctness obligations, but cannot fill
Slice100's active-checkpoint/richer-postcrash gaps by inference.

## Composed oracles

Use real SQLite databases and specified expected selected IDs, eligibility,
provenance, UTF-8 coordinates and committed/rolled-back post-state. An optimized
route and reference sharing the same predicate are not independent proof.
Same-store/view SDK parity verifies the boundary, while independently authored
expectations verify correctness. Text parse-back must check framing/quoting,
kept fields and rounded display semantics against the approved contract.

Preserve tiny-budget zero-progress/refusal cases and progress-capable continuation
controls; do not promise gap-free traversal when metadata cannot fit. Qualify
ordinary/historical flags only once the repeated-public-ref feedback rule is
actually settled. No new native ABI is selected here. Preserve stored B2 spans,
built hints and truthful unavailable/unknown coverage;90 builds only approved80.
Supplemental post-code acceptance (such as60's tiny real projection manual check)
is honestly classified as qualification, never backdated RED or new passing-on-arrival
regression coverage.

## Execution discipline and devices

Prepare own environment and final clean code tree before full verification.
Preserve actual full wrapper failures, scoped resumption and per-suite exits;
a concatenation of scopes never becomes uninterrupted all-green. Feature inventory
comes from current runner/matrix, not copied356-target or204/206 historical counts.
Strict live security needs namespaces/ptrace; AC037 downgrade or skip cannot supply
its strict0violation/0blocker/0downgrade release row. A denied executor is not a
product pass. Preserve compiler diagnostics verbatim.

Embedding/reranking campaigns use an available RTX3090 with actual engagement;
queue when busy. Only small CPU-library compatibility controls use CPU. Never
allocate K620. windchill3 driver580.178.04 stays forever; primary CUDA12.6 stays
unchanged. Orin12.6 is not reflashed.13.1+ PTX runtime behavior is not measured
on R580. Broad venue direction is already ruled; specific access/vetting/session
caps and compiler exposure remain real row prerequisites, not a new broad vote.

## Independent local execution batch

Run the existing `lifecycle_reliability` test
`ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl` with
`--features operator` and `AGENT_LONG=1`; its actual60-second window must execute
projection add/drop and rebuild work with zero `SQLITE_SCHEMA` events. Run the
existing `cursor_read_after_write::projection_cursor_bounds_observed_row_count`
with `AGENT_LONG=1`; retain its1000-search, positive overlapping-write and advancing
revision assertions. Use ordinary CPU execution, real SQLite, an owned build
directory and raw command logs. No full-gate replay or GPU campaign is needed.

Use `cargo package --offline` for `fathomdb-embedder-api`, `fathomdb-schema` and
`fathomdb-query`, retaining Cargo's archive verification. These are independent
leaf packages without unpublished sibling dependencies. This is local package
preparation, not the complete local rehearsal, installed Python/Node proof or
registry publication. Reassess the relevant source/lock/toolchain changes after90;
unchanged leaf inputs need no automatic repeated package build.

Close the missing Q4 live component with the existing
`scripts/security/check-netns-deny-egress.sh` and
`scripts/security/check-netns-deny-egress-catch.sh`, using a freshly rebuilt locked
`security_cycle` example. Both live layers must actually pass on a capable
executor, without `AC037_LIVE_OPTIONAL` or host security-setting changes. Reuse
accepted unchanged other security controls only within their actual applicability;
these scoped executions are not a new aggregate battery run.

## AC-082d installed explicit-CPU capture

Add one maintained parameterized release capture/validator for the missing
AC-082d route. It consumes the retained CUDA-compiled wheel and npm main/platform
archives, creates its own noneditable Python environment and npm consumer, and
uses the existing BGE open/embed/write/search/close lifecycle with explicit
`FATHOMDB_EMBED_DEVICE=cpu` and `FATHOMDB_RERANK_DEVICE=cpu`. The consumer reads
only its open report's compiled-capability/effective-CPU fields; it calls no
CUDA availability/device API. No source checkout is imported.

Launch each installed binding under bounded `strace -ff` file/process/ioctl
capture and `LD_DEBUG=libs,files`, covering import through process exit and all
children. Retain raw captures, real exits, artifact identities and the open
report. A handshake holds the live consumer after its forwards until the
external observer records its actual host PID, liveness and selected-UUID compute
process samples, then releases it to close. The observer samples from startup
through exit. CUDA driver/library/device access, a selected device/GPU PID, a
failed/missing trace or loader capture, incomplete lifecycle or nonzero exit
refuses CPU qualification; a negative GPU sample alone is insufficient.

For each binding, a separate fresh explicit-CUDA child uses the same capture
path and model pins. Its actual forward, selected-UUID/PID observation and
positive CUDA library/device trace establish the CUDA venue and capture power;
positive-control failure invalidates the corresponding CPU negative. Preserve
these observations separately from the existing allocation/PID-namespace oracle.
The Windchill lease is
`/mnt/data/tmp2/fathomdb-0829-continuation/slice30-gd/locks/GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b.lock`,
inode8532374, inherited on FD9. No GPU query or runtime capture runs until G is
strictly terminal and root provides fresh exclusive custody. CPU-only D work
can overlap if it neither uses the selected GPU nor mutates these fixtures.

TDD covers the new measurement behavior: observed RED for the absent route,
then GREEN for valid CPU/positive captures and refusal of actual CUDA access,
missing/failed capture, a GPU PID in a live CPU interval or a failed positive
control. Use authored small trace/observation examples and real CPU subprocess
capture controls; no database mocking, snapshot generation or unrelated tamper
suite. A discovered product CUDA initialization on explicit CPU gets its own
separate product RED/fix; the measurement must not hide it. Index the command in
the existing experiment README and perform one substantive final review after
the actual batch. This adds no product API or new release acceptance condition.

## Delivery and review

Slice10 already repaired .29 bootstrap routing. Revalidate exact remote tag,
runner/environment protections at the actual authorized release step; do not
reopen the old .28-only route premise or change permissions speculatively.
Local packaging, hosted dry-run, installed candidate consumers and published
registry consumers are four distinct observations. Skipped publisher bodies never
count as executed, and source-bound SDK OIDC proof never substitutes for packages.
Maintain AxisW .29 notes/source breaks and independent AxisE truth.

After local qualification passes, review the CI scripts and then execute CI;
the former blanket CI exclusion is superseded. Publishing tags and registry
publication remain unauthorized. Fresh Astra design review must PASS before implementation/execution entry. Any
confirmed behavioral repair gets its own meaningful staged/committed observed
RED and separate GREEN with authored oracles unchanged, then scoped verification
and fresh independent final review. Documentation/evidence closure needs no
invented failing test or TDD waiver. Publication remains separately authorized;
no release-complete claim precedes fresh registry lifecycle proof.
