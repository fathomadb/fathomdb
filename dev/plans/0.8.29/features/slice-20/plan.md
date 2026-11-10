---
title: "Slice 20 — dependable queue and parallel-test evidence"
status: PROPOSED
target_release: 0.8.29
slice: 20
---

# Slice 20 — dependable queue and parallel-test evidence

## Current checkpoint scope

Scope approved in Slice9 Batch3: hygiene and queue proof. Reproduce actual residuals; preserve existing targets/queue semantics. Final execution acceptance remains pending.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and boundary

Make queue/engine evidence dependable before Slice 30. This proposal follows
the [drafting brief](../../planning-brief.md) and
[reconciliation](../../reconciliation-plan.md), not execution authorization.
The format-37 baseline includes published .28 and integrated Block 1; retain
the existing admission, run-to-completion, failed-open and evidence contracts.
No queue-policy redesign, benchmark campaign, chunking implementation, blanket
serialization, warning suppression or changes to accepted result oracles.

| Carryover | Current evidence and proposed disposition |
| --- | --- |
| TC-cbff7211 | Split by behavior: .28 Slice 42 repaired the Slice 135 owned-file FD assertion. TC-57's `wal_pair` already configures runtime before raw SQLite opens; reproduce its remaining parallel failure before assigning a fix. |
| TC-7a7bf5b0 | Evidence sidecar/resolver hooks remain process-global one-shot statics; historical parallel failures exist. Confirm current interference, then isolate hook ownership. |
| TC-404cd302 | `before_enqueue` is absent. Existing arrival hook follows queue insertion; registry already rechecks phase/epoch after permit acquisition and can return to open wait. Add a deterministic observation seam and race-23 proof. |
| TC-0f65b549 | Historical MCP release-only dead-code warning is unconfirmed on current source; identify the exact item/build before changing it. |
| TC-fea3c1c4 | Original Windows startup tests changed for failed-open semantics; opener test now configures runtime before its barrier. Map all three failures to current controls individually. |
| TC-2a9f89fa | T2 constants guard is planned; production values already match its accepted baseline. Chunking remains parked; T1/T3 measurement/decisions belong elsewhere. |

## Inspected inputs and architecture preparation

Read the [release allocation](../../../plan-0.8.29.md),
[queue design, including test 23](../../../../design/agent-access/queue-admission.md),
[runtime ADR](../../../../adr/ADR-0.8.25-sqlite-runtime-configuration.md),
[evidence ADR](../../../../adr/ADR-0.8.25-compact-source-evidence.md),
[Slice 42 design](../../../0.8.28/features/slice-42/design.md), and
[quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
Bounded source inspection covered MCP `registry.rs`, `hooks.rs`, `seams.rs`
and manifest; engine `test_hooks.rs`, TC-57 helpers, evidence race bodies,
the complete `slice40_fresh_database_cutover.rs`, runtime/provider admission
tests, current startup-fault modules and opener rendezvous in `lib.rs`.
Also inspected engine `fusion.rs`, embedder `candle_bge.rs`, and the
[chunking re-evaluation record](../../../../design/overflow-chunking-reevaluation.md).
Historical TC details were retrieved read-only through `ledgerwatch`.

Prepare local `needs.md`, `requirements.md`, `acceptance.md`, `design.md` and `test-map.md`.
Draft architecture: startup-only SQLite selection precedes raw connections;
test hooks belong to their engine/request; queue observation must precede
the Ready-to-admission gap without holding registry locks across a wait.
Challenge timeout/unwind cleanup, hook theft by unrelated engines, epoch
changes, process-global SQLite state, and dev-feature unification hiding warnings.
Primary-source research is conditional: inspect the pinned Tokio semaphore/
watch documentation and SQLite configuration/locking documentation when the
reproducer depends on their guarantees; retain exact versions and source links.
Fit review applies bounded real-database anomaly/resource checks and targeted
interleavings; defer broad fuzzing, soak, exhaustive mutations and new harnesses.
Revise the draft after reproduction and independent requirements/design review;
any public test-hook API change needs its contract documented in the same PR.

## Draft local needs, requirements and acceptance

| Need / requirement | Falsifiable local acceptance |
| --- | --- |
| S20-N1 / S20-R1: parallel evidence must retain its meaning | S20-A1: every carried failure has exact test/build/source provenance and current reproduced, already-fixed or unresolved disposition; no whole-TC closure from one repaired subcase. All IDs here are release-local. |
| S20-N1 / S20-R2: unrelated tests cannot steal hooks or poison startup | S20-A2: deterministic overlap on separate real databases preserves each intended rendezvous/outcome; parallel targets and isolated controls pass with original semantic assertions. |
| S20-N2 / S20-R3: admission must survive a pre-enqueue drain | S20-A3: hold a call after observing Ready but before admission accounting; start idle/LRU drain, resume, observe stale phase/epoch rejection and open-wait/reopen outcome within the original wait budget; no old-engine call or leaked permit/queue count. |
| S20-N3 / S20-R4: release diagnostics remain truthful | S20-A4: reproduce any warning in the exact affected non-test release configuration; fix feature ownership/removal and show the diagnostic absent with test seams still functional. No blanket allow or reduced lint scope. |
| S20-N4 / S20-R5: model/fusion drift leads to re-evaluation | S20-A5: guard actual RRF_K=30, text=3, vector=1, DEFAULT_EMBEDDER_NAME="fathomdb-bge-small-en-v1.5", MAX_SEQUENCE_TOKENS=512; failure names the re-evaluation document and requires a recorded review. |
| S20-N1 / S20-R6: Windows evidence reflects failed-open policy | S20-A6: each historical startup error/hang maps to a current named test; accepted-probe error, verification-timeout precedence, owner cleanup and opener-lock controls execute on an available Windows venue with bounded termination. |

## Dependencies, resources and conflicts

Hard inputs: recorded Slice 9 decisions; needed Slice 10 owned environment,
current .29 state/preflight and approved preparation; reviewed local design.
Slice 30 depends on reliable affected harnesses, not on an invented new soak.
Conditional: Windows execution requires its approved runner/VM and exact source
build; lack of access leaves S20-A6 open, never a Linux-as-Windows claim.
Conditional: adding a guard or coverage that already passes needs an explicit
TDD disposition before landing; do not fabricate RED by changing constants or
breaking correct admission behavior. A missing-seam compile failure alone does
not establish a production queue defect. Carry this prerequisite to Slice 9/10.
No GPU is inherently needed; default-embedder builds use approved existing
resources only if the reproduced warning/configuration requires them.
One writer per checkout; serialize engine test-hook/lib, MCP registry/seams and
manifest edits against other slices. Land before measurements for attribution;
independent requirements work for 60/70 may continue in isolated worktrees.

## Work breakdown and deliverables

1. Fold TC evidence by behavior; preserve .28 Slice 135 repair and historical
   failed receipts. Read complete affected test files before prescribing edits.
   Record current ordinary-parallel and isolated-control commands/results;
   include Slice 40 cutover raw-SQL/process-start paths in runtime-race triage.
2. For each confirmed residual defect, first stage/commit a minimal meaningful
   failing regression at that baseline, record its assertion, then fix only
   the owned cause. Prefer engine-scoped hooks and configure-before-open;
   justify narrowly scoped serialization only for unavoidable process state.
3. Design `before_enqueue` with explicit reached/release acknowledgement,
   bounded waits and cleanup on panic/cancel. Drive the actual race-23 window;
   preserve shutdown test 24 and idle/LRU busy-store restrictions.
4. Reproduce the release warning under non-test release features, then remove
   stale code or gate test-only ownership. If absent, record that finding and
   avoid speculative edits or a test that merely mirrors a cfg expression.
5. Add the approved constants guard at module-local locations for private
   `MAX_SEQUENCE_TOKENS`; avoid exporting a production API just for inspection.
   Bind assertions to live constants, not copied source-text regex snapshots.
6. Map old Windows tests to the accepted-probe startup-error control,
   timeout-before-visibility control, accepted-probe visibility-error control
   and `current_opener_holds_lock_before_admission_classification`; retain exact
   cleanup/error assertions and report each remaining defect separately.
7. Deliver reviewed local documents, per-behavior RED/GREEN receipts where a
   defect exists, current parallel/control evidence, diagnostic transcript,
   Windows disposition and trusted-harness handoff to Slice 30.

## Verification, review and exit

Use small targeted reproductions first; preserve real databases and existing
snapshot/refusal, provenance, lock, worker and cleanup oracles. No sleep-only
race proof, relaxed assertion, global single-thread gate or retry-until-green.
Each fix needs its own failing RED SHA and separate GREEN; verify RED from Git.
Stop after two same-issue failures to reread test, ADR and memory;
unresolved product semantics return for a decision.
Run appropriate lint, typecheck and targeted tests, then mandatory full
`scripts/agent-verify.sh` once on final source/test/script changes using owned
tools. Keep release non-test warning checks distinct from test-feature clippy.
Independent review checks design fit, seam placement, unchanged oracles,
parallel isolation, TDD disposition and exact platform claims before landing.
Exit with S20-A1–A6 satisfied or explicit owner disposition; unresolved harness
defects block affected Slice 30 measurements. Integrate
reviewed current-.29 work without rewriting historical evidence or ancestry.
