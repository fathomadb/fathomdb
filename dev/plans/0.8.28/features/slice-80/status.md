---
title: FathomDB 0.8.28 Slice 80 — status
status: COMPLETE
target_release: 0.8.28
qualified_sha: d641f43f73d0576fc24af28c80d5be199fc5332f
---

# Slice 80 status

Complete on `dev/0.8.28` and `release/0.8.28`. Qualified candidate: `d641f43f73d0576fc24af28c80d5be199fc5332f`.
[Plan and acceptance](plan.md) and [design](design.md) reconcile the draft with
Slices 50/60/70, the retained-edge completeness obligation, and the owner's
property-corpus lifecycle/performance requirements. Astra approved the design,
production changes and final test-hook ownership correction.

Search honors independent superseded/inactive flags in Rust, Python and
TypeScript, including text, dense, property, graph, frozen, paginated and
evidence paths. Default eligibility exclusions remain. Historical vectors are
retained and metadata updated for both edge supersession keys and consolidation;
queued/in-flight work, reopen and rebuild use consistent membership. False
successful legacy skips are requeued; partial tuples are refused. Source
closures and hard erasure retain their authority. Missing dense history yields
explicit whole-arm vector fallback rather than an incomplete-history claim.
Evidence flags relax artifact lifecycle only; source authorization remains
strict and is checked at issuance and resolution.

Private in-memory FTS is used only for property searches with either existence
flag. Strict/validity-only property searches and body/dense searches do not copy
a property corpus. Each call clones the existing all-property statistical
baseline, adds authorized history, applies output eligibility before limits,
and hydrates winners from one held main snapshot. Explicit MEMORY temporary
storage prevents SQLite corpus/sorter files. Ownership is per call, with no
pool cache; success/error paths drop the corpus before response delivery.
No added rows means exact property score/order parity. Retained historical edge
FTS can change default edge BM25/graph scores or order through corpus statistics;
the algorithms and default eligibility rules are unchanged.

Concurrent writes yield coherent snapshots, not a mixture of versions.
Concurrent erasure may report `ErasureIncomplete` while a reader pins WAL;
tests verify successful retry after release and no later-query resurrection.
Erasure does not revoke already materialized results or zero process memory or
swap. Active work continues after its response receiver is dropped; close waits
for readers. There is no corpus memory/time cap. These guarantees and limitations
are explicit in the design, with engine-owned pause/fault tests for supersession,
erasure, all-eight-reader error reuse, receiver drop and close.

## Cost observations

The committed ignored `measure_historical_property_cost` test is reproducible
with `cargo test -p fathomdb-engine --features operator,test-hooks --test
slice80_history measure_historical_property_cost -- --ignored --nocapture
--test-threads=1` (one shell command). Debug build, two property declarations,
two canonical versions per logical record, three serial queries per mode,
and a synthetic common-term/tie-heavy corpus:

| Logical records | Strict mean/query | History mean/query | Four history readers, 12 queries total |
| ---: | ---: | ---: | ---: |
| 512 | 8.1 ms | 23.8 ms | 65.7 ms |
| 2,048 | 21.8 ms | 86.0 ms | 256.5 ms |

At the larger size, the concurrent phase sampled about 61 MiB total process RSS.
This includes engine state and allocator reuse; it is not per-corpus memory or
a capacity bound. Population performs corpus copies and per-declaration
canonical scans, with additional authority/filter work. Longer reader occupancy
can increase queue/close latency and erasure contention. Retained history adds
storage, embedding, scan and rebuild work. Boot lexical membership now uses one
inventory instead of repeated unindexed FTS scans; 64/256-history fixtures used
9,843/39,603 SQLite VM steps (4.023x work for 4x rows). These scoped observations
are not a production-scale latency or memory guarantee.

## Verification

The independent gate campaign is complete. Lint, typecheck and strict
unconfined security passed; security exercised netns and ptrace with zero
violations, blockers or downgrades. The initial full invocation stopped at
module ownership. Scoped fixes classified test modules, relocated the receiver
test without weakening boundaries and corrected semantic imports; the failed
lint stage and subsequent stages then ran.

The 205-suite collector ran every registered suite: 200 passed and five failed.
All five were resolved through scoped repairs and independent reruns: root
inventory, exact private-memory connection classification, six Rust targets,
and Python/native-receipt qualification. The retained property-test seed remains
tracked. The source audit accepts only the specific memory opener and rejects
file-backed, duplicate and unrelated opens. Its baseline passed 268 assertions;
root inventory passed eight. Rust reruns passed 56 focused tests plus their
child-process checks, including 17 Slice 80 history tests and six lifecycle
integration tests. The test-only frozen hook is now engine-owned; its RED test
reproduced cross-engine hook theft and its GREEN proves isolation.

The TypeScript native build and complete test suite, plus the NAPI executable
build fixture, passed in the collector. Python rebuilt the production candidate
`42c2c52a77bb5244a83f4912852478690df36f54`: 1,705 tests passed and 36 skipped;
one static checksum guard needed the approved stub-comment normalization.
The guard then passed independently at the qualified candidate above. Only
that test file changed after the native build; the original golden checksum
was preserved and the native receipt/module digest was validated. No production
or package bytes changed after the Python build. This is a completed campaign
with scoped repairs, not a claim that the initial invocation passed.

The real boot-repair defect found during qualification is fixed: lexical-only
restoration advances visibility within its transaction, invalidating prior
frozen tokens. Failure rolls back both changes; no-op reopen preserves tokens.
Independent repair, rollback and reopen tests passed. No verification failures
remain. Release platform/long-run qualification and publication remain later
release work; Slice 90 is next.

Logs: `/tmp/slice80-full-verify.log`, `/tmp/slice80-lint-resume.log`,
`/tmp/slice80-typecheck.log`, `/tmp/slice80-security.log`,
`/tmp/slice80-tests.log`, `/tmp/slice80-property-cost.log`,
`/tmp/slice80-property-lifecycle.log`, `/tmp/slice80-scaling-green.log`,
`/tmp/slice80-final-python.log`.
No new worktree or branch was created. No tag or publication is authorized.
