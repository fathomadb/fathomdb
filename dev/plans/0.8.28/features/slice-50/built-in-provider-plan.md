---
title: Built-in provider contract — design work plan
status: proposed; design work authorized; production implementation not authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider design work plan

Owner scope: steward `seq-355`. User-specified embedders are a long-term goal,
unsupported before 1.x. The owner commissioned Astra-medium to develop the
replacement contract, enumerate existing extension paths, update needs and
requirements/acceptance criteria, and bring the result back before implementation.
This work does not place a new implementation unit in the release ladder.

## Design sequence

1. Audit actual production entry points, declarations, loader/construction paths,
   alternative backends, standalone helpers, test hooks and accepted ADRs.
   Give every discovered extension path a close/eliminate or contained/deferred
   disposition, with a source witness and a future verification requirement.
2. Specify the built-in runtime contract: actual loaded assets and fixed vector
   function, execution stability, verification cadence, coherent store-state
   identity, first enrollment, live mean changes and failed-open recovery.
3. Specify positive-result cache ownership, exact bounds, eviction, invalidation,
   concurrency and publication ordering independently of database/model lifetime.
4. Update user needs and traceable feature requirements/acceptance criteria;
   reconcile Slice 50's old Unit D documents and the proposed successor ADR.
5. Have Astra check the integrated contract for contradictions and missing
   closure paths. Record scoped document/state/ledger validation and push the
   concrete proposal for owner review.
6. Stop before production source, dependency, schema or behavior changes.

## Implementation sequence proposed for later approval

After contract and release placement are approved, delegate implementation in
isolated worktrees, one writer per checkout. Implement behavior-specific RED
then GREEN pairs in dependency order: surface closure; stable construction and
runtime; store verification-state transitions; bounded reuse; typed failed-open
and cleanup; recovery/diagnostics/binding parity. Do not carry D50-1/D50-2 forward
as an alternative implementation without a fresh owner ruling. Unit S's scan
optimization retains its separate requirements and existing review gate.

Tests must encode the approved contract; intentional supersession of old
behavior tests must be explicit, never an edit to make a fix pass. Preserve
committed characterization and timing data as historical baseline evidence.
Run scoped checks during work; run the full strict agent gate once on the final
code tree. The current slice checkout has no checkout-owned `.venv`; its
previous full-gate attempt stopped at that preflight, so setup is a prerequisite
to a future full verification claim. Do not borrow another checkout's venv.
