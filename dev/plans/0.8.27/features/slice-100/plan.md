---
title: FathomDB 0.8.27 Slice 100 - PyO3 execution plan
status: IN_PROGRESS
target_release: 0.8.27
planning_baseline: c149584bcc259594856433a0963ea126c633a634
---

# Slice 100 execution plan

## Entry decision and changes since the draft

Slice 90 is closed on `release/0.8.27` at `c149584bc`. The required preflight
`./scripts/preflight.sh --expect-closed 90 --plan dev/plans/plan-0.8.27.md`
passes. This plan supersedes the pre-entry assessment at `5f9433c98` and
approves Slice 100 for execution, subject to the design review below.

The prospective design was drafted against `63091b624` before Slice 90. The
entry comparison (`git diff 63091b624..c149584bc`) and assigned work yield:

1. Slice 90 completed the five effective configuration knobs, runtime
   qualification, the installed Python/Node handoff, and engine runtime
   closure. Its final status and release-state receipt are now inputs, not
   work for Slice 100. The Python native source remains a single `lib.rs`
   with the same one `_fathomdb` initializer and one `PyEngine` methods block.
2. The native root now has Slice 90 configuration forwarding and associated
   tests. Preserve its exact keyword/default/validation order and effective
   behavior; the Python SDK configuration changes stay closed with 90.
3. The Slice 110 binding-closure review still assigns PyO3 inventory,
   installed-artifact joins, subscriber disposition, FFI/GIL/lifetime proof,
   and scanner retargets here. NAPI subscriber and executor work stays in 110.
4. The prework test approach requires real installed native artifacts,
   registration/signature/error/ABI comparison and deterministic thread
   witnesses. Slice 30's baseline remains immutable. The Slice 90 approved
   deltas are carried separately into the entry comparison.
5. The accepted Python interface and binding design promise LogRecord delivery
   from `attach_logging_subscriber`, but the native method still discards both
   arguments. The engine has `subscribe` and synchronous callbacks, but no
   heartbeat emission or operation ID. A separate bounded-delivery successor
   is proposed before moving code, after the callback architecture review.
6. Source readers include `dev/tools/surface_comparator.py`, the Slice 50
   hook inventory, Windows WAL guard, Slice 70 embedding-doc gate, stub and
   Python source citations. Re-enumerate readers before each move, then prove
   scanner retargets with a failing mutation before trusting a green result.
7. Existing `src/python/tests`, in-crate `#[cfg(test)]`, release CI feature
   selectors, and installed-wheel verification already cover much of the
   acceptance set. Add focused tests only for missing behavior and proof;
   preserve existing tests and their oracles during extraction.

**Evaluation:** keep R27-100A–E and the draft's semantic owner map. No new
public verb, async facade, schema, dependency upgrade, free-threaded mode,
Python SDK rewrite or NAPI work is needed. Amend R27-100C to require bounded, fault-contained subscriber delivery
under the proposed accepted successor, with no synthetic heartbeat;
amend R27-100D to bind only affected feature/platform routes to this candidate.
The work remains one slice with small, sequential batches. An unavailable
required route blocks completion rather than being recorded as a pass.

## Requirements and acceptance

| ID | Need and requirement | Acceptance criterion |
| --- | --- | --- |
| R27-100A | Native ownership is explicit and complete. | AC27-100A: source-derived entry/final ledgers name every native item, cfg, registration and Python-visible identity, with one owner or reviewed root exception; one initializer and one Engine methods block remain. |
| R27-100B | Python contracts survive extraction. | AC27-100B: installed runtime, package exports, stub declarations, signatures, class modules, exception identity, cfg and ABI match entry plus only reviewed contract deltas; the Slice 30 baseline is unchanged. |
| R27-100C | FFI behavior and lifetime are correct. | AC27-100C: real-native tests prove blocking GIL progress, ownership through concurrent close, panic/error/hostile-input precedence and unchanged DB on invalid write; bounded subscriber delivery, overload, replacement, callback failure/reentrancy and close behavior meet the reviewed successor design. |
| R27-100D | Artifact proof uses the shipped path. | AC27-100D: fresh isolated wheels at the candidate exercise default and affected feature routes, with imports inside the venv, abi3 floor, test hook presence/absence and configuration efficacy. Required platform-only compilation has candidate-bound receipts. |
| R27-100E | All Slice 100 work closes before 110. | AC27-100E: scanner guards, repository verification, independent code review and independent read-only verification pass on the final candidate; state/status record zero remaining Slice 100 obligations and advance the ladder. |

## Design and change control

The [Slice 100 design](design.md) owns semantic module boundaries and exact
behavior constraints. The entry subscriber addendum there and the proposed subscriber ADR own
the bounded adapter and delivery policy. The public Python reference and
EARP knob catalog are updated with the changed signature and live behavior. An accepted successor is required
before closeout because the old locked design promises Python heartbeats. Review this code-grounded
design with `gpt-6.1-sol` high **first**; any follow-up design review uses
`gpt-6-sol` high. Resolve findings in the design before RED/GREEN. The Python
interface and binding design receive the exact interval/error-policy amendment
in the same change. No baseline is regenerated to excuse a difference.

## RED/GREEN implementation order

1. Freeze entry SHA, native item/registration and package/stub ledgers,
   installed runtime identity and feature matrix. Enumerate all source readers
   and full existing tests before claiming gaps. Record owner for every item.
2. Add fixed subscriber tests and demonstrate RED against the no-op. Review and
accept the bounded-delivery successor before relying on GREEN. Add only
   missing FFI/GIL/lifetime/precedence witnesses. Implement bounded subscriber delivery
   to GREEN as a separate, reviewable behavior commit. Capture
   the corrected surface comparison point.
3. Extract errors/FFI, carriers, Engine facade, write, read/search,
   graph/evidence, projection, embedding, admin and test support in cohesive
   batches. Keep one Engine methods block; move the block whole. Keep root
   initializer registrations visible to the existing comparator. For each
   batch: format, compile affected feature routes, run focused tests and
   source scanners, and fix code without weakening tests. Retarget scanners
   with a RED mutant before accepting their GREEN.
4. Reconcile final ledgers against entry plus the subscriber delta. Build
   wheels through `scripts/verify-release-python-wheel.sh` with new owned
   wheel/venv paths; run installed consumer import/open/write/read/close,
   configuration, exception and native-hook checks outside the checkout with
   `PYTHONPATH` unset. Run the affected feature/platform matrix and
   `./scripts/agent-verify.sh`. Broader final release qualification remains
   Slice 150 unless this slice changes its route.
5. Bind independent `gpt-6-sol` high code review and a separate read-only
   tester/verifier to the final candidate. Resolve concrete findings with
   tests fixed, rebind if production changes, then write the Slice status and
   release-state record. Merge the branch into `release/0.8.27`, verify Git
   state, and remove the temporary worktree and branch.

Keep batches reviewable and tests fixed. Neither line-count symmetry nor a new
Python helper is an acceptance criterion. Do not use editable installs from a
worktree; the shared `.venv` belongs to the canonical checkout.
