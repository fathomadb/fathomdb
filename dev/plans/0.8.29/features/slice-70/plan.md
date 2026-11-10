---
title: Slice 70 — Truthful degradation and SDK/native contract parity
status: PROPOSED
target_release: 0.8.29
slice: 70
---

# Slice 70 — Truthful degradation and SDK/native contract parity

## Current checkpoint scope

Bounded degradation/parity correction scope approved in Slice9 Batch4. Review exact reason/error precedence; broader EP03, caller numerical injection and new thresholds are not implied.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and boundary

Give callers truthful per-call reasons for missing query embeddings, nameable
public result types and consistent snippet byte contracts across bindings.
This is a proposal under the [brief](../../planning-brief.md) and
[reconciliation](../../reconciliation-plan.md); drafting authorizes no implementation.
Track `TC-42c6708b`, `TC-c4fd774a` and `TC-36bc3ad8`; reconcile their original
claims against current source before recording closure.

- Built: `EmbedderOverloaded`, search/expand propagation and empty-list omission
  in Python/TS; Rust always carries a vector. Queue saturation and queued expiry
  share `embedder_overloaded`; do not relabel queued expiry as started timeout.
- Net-new allocated scope: absent (`NotConfigured`) and started-timeout reasons,
  facade export residuals and demonstrated documentation/native parity gaps.
- The [EP-03 draft](../../../../design/mcp-adapter/engine-proposals/EP-03-hybrid-search-degradation.md)
  is only partly implemented/approved. `EmbedderFailed`, eligibility/projection
  reasons and broader adapter changes are not authorized by this allocation.
- Preserve the [accepted open policy](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md):
  actual checker refusal returns no Engine; None admission is deliberate.
  Do not restore `dense_disabled`, caller-provider production admission, new
  checker/cache behavior, or pretend provider-free text requires embedding.

## Slice preparation and local acceptance draft

Deliver `requirements.md`, `design.md` and `test-map.md` beside the executed
slice plan. The IDs below are release-local drafts, not canonical AC additions.

| Need / requirement | Falsifiable local acceptance |
| --- | --- |
| S70-N1/R1: explain skipped vector work | S70-A1: `NotConfigured` produces only `vector/embedder_absent`; `StartedTimeout` produces only `vector/embedder_timeout`, with surviving text hits and no fabricated vector hits. |
| S70-N1/R2: distinguish overload and failure | S70-A2: saturation/queued expiry retain overload; Closing/Cancelled fail the call, CUDA typed failures remain errors and panic resumes; none becomes a success degradation. |
| S70-N2/R3: consistent requested-arm reporting | S70-A3: ordinary, frozen, evidence and expand paths preserve identical reason meaning; healthy reads have no new labels; explicit text-only reads never embed or report an unrequested vector arm. |
| S70-N3/R4: usable Rust contracts | S70-A4: external facade consumer can name the approved hit/identity/snippet types; SDK consumer still compiles independently with its existing exports. |
| S70-N4/R5: portable snippet ranges/options | S70-A5: all five option fields/defaults/ranges and half-open UTF-8 ranges agree through Rust/PyO3/Python/NAPI/TS; multibyte examples decode the intended substring. |
| S70-N2/R6: preserve result contract | S70-A6: fixed real-store cases retain hits, order, scores, soft fallback and explanations; only approved reason metadata changes; Rust empty/Python None/TS absent conventions remain. |

Read the [Rust](../../../../interfaces/rust.md), [SDK](../../../../interfaces/rust-sdk.md),
[Python](../../../../interfaces/python.md) and [TypeScript](../../../../interfaces/typescript.md)
interfaces with the [SDK ADR](../../../../adr/ADR-0.8.27-rust-sdk-parity.md).
Draft the dispatch outcome table first, then the consumer/export and field matrix;
challenge every claim against live code, including existing inline tests.

## Design challenges, research and fit revision

The inspected [embedding helper](../../../../../src/rust/crates/fathomdb-engine/src/embedding.rs)
currently swallows `Closing`/`Cancelled` via `Err(_) => None`; EP-03's assertion
that these already fail is stale. Review the exact error mapping before a bounded
fix; use the existing Closing category rather than inventing cancellation DTOs.
Audit bypass/early-return paths so reasons are neither lost nor duplicated.
Do not infer runtime provider health from open-epoch readiness metadata.

The [facade](../../../../../src/rust/crates/fathomdb/src/lib.rs) lacks exports
already present in the [SDK](../../../../../src/rust/crates/fathomdb-sdk/src/lib.rs),
including hit/identity and snippet types. Produce an exact proposed export list,
including whether the pure snippet helper belongs, with governed-surface impact.
The SDK is a separate closed operation surface; do not merge its contract with
facade wholesale engine reachability or add commands to close a type gap.

[Python types](../../../../../src/python/fathomdb/types.py) and
[TS search](../../../../../src/ts/src/search.ts) already carry five snippet fields
and UTF-8 examples. Revise historical “missing” claims to proven residuals;
make `[start, end)` explicit, preserve body-relative versus text-relative ranges,
and preserve search rejection versus pure-selector clamping.
Planned primary-source research: trace dispatch deadlines/shutdown ownership,
frozen/evidence/expand adapters, native conversion and actual exported consumers;
consult pinned PyO3/NAPI docs only if conversion semantics remain uncertain.
Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
through real-store provider faults and Unicode properties; defer broad protocol
fuzzing, allocation sweeps and CUDA campaigns. No new resource campaign is implied.

## Dependencies and newly identified prerequisites

- Hard: recorded Slice 9/10 authorization and owned tooling for execution;
  reviewed bounded reason/error mapping and governed export approval before code.
- Design may start immediately. Serialize engine landings 40, 50, 70, 100, 110;
  40/50 are shared-file/attribution ordering, not semantic prerequisites for labels.
- Coordinate Slice 60's compact/structured formatting and final integrated SDK
  parity; Slice 80 approval consumes settled degradation/field contracts, and
  Slice 90 also needs the integrated Block 1 follow-up exit after 20–70.
- Newly identified: reconcile stale Closing/Cancelled claims; specify deterministic
  started-timeout controls that finish cleanup; approve exact facade type delta.
  Existing test-only provider seams may model faults, never production injection.
- One implementation writer per isolated checkout; serialize shared dispatch,
  search/result types, conformance allowlists and canonical interface changes.

## Tasks, validation and integration

1. Complete outcome/export/option census and local requirement-to-test mapping;
   independently review design fit and authorization before implementation.
2. Add behavior-specific failing tests, commit/stage RED evidence, then implement
   absent and started-timeout labels separately; preserve overload regression.
   Use real databases and dispatcher gates with bounded joins, not sleep guesses.
3. Add runtime-close/cancel error regressions and preserve CUDA/panic controls;
   exercise ordinary, frozen, evidence and expand paths with no false fallback.
   Review existing timeout assertions intentionally: contract updates must encode
   approved behavior before the fix, never weaken tests to obtain GREEN.
4. Add external facade compile tests for approved exports; reuse actual SDK
   consumer tests and observed surface checks, not duplicate literal allowlists.
5. Extend native conversion and shared snippet-fixture coverage through installed
   checkout-owned Python/Node artifacts; wrapper mapping tests alone are insufficient.
   Properties cover combining marks, astral text, grapheme-safe bounds, defaults,
   each option field and search rejection/pure clamping; do not regenerate goldens.
6. Update actual interface docs, docstrings and approved surface records in the
   implementation PR; remove stale claims only where the approved change lands.
7. Run scoped checks while iterating, then the required full agent gate once on
   the final code phase, plus affected native/SDK parity and both MCP formats.
   Missing owned environments fail preflight; never borrow another checkout's venv.
8. Obtain independent code review, sample RED evidence and resolve findings;
   integrate on current .29 without overwriting other writers; recheck affected
   seams after rebase/landing and provide receipts for the Block 1 follow-up exit.

## Decisions and exit

Owner/reviewer must settle the exact export list and bounded Closing/Cancelled
correction; broader EP-03 reasons or adapter redesign need separate authorization.
Exit requires A1–A6 evidence, approved contract changes, native parity and review;
unavailable venues or residual findings remain explicit open items. No review,
HITL ruling, test pass, ledger closure or publication is claimed by this draft.
