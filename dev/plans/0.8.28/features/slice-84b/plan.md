---
title: Slice 84b plan — exact certification work and transactional resource totals
status: IN_PROGRESS
target_release: 0.8.28
---

# Slice 84b plan

## Reconcile and select

Freeze the actual retained Slice 84 code after its final verification. Reconcile
any intervening changes to admission, commit, projection, dependency closure and
worker paths before code. Slice 83's safety suites and Slice 84's negative results
remain constraints. The rejected priority candidate is historical evidence only.
Review all callers of `guard_plain_write`, `certify_plain_write`, `validate_zero`,
`envelope`, artifact registration, provenance retention and cleanup metadata
mutations. Inventory every counted table and every path that can affect it.

First attribute SQL VM steps/full-scan steps by query on the pinned SQLite build.
Inspect actual query plans; do not assert their unstable display text in tests.
Prioritize eligibility and registered-copy scans, not the already-small MAX query.
Complete the effect table in the design against actual code before enabling any
fast path. Astra reviews concrete changes and any contract/schema impact before
implementation, then reviews code. A separate subagent verifies independently.

## Requirements and acceptance

These are local slice labels, not new canonical product AC identifiers.

| ID | Requirement and decisive acceptance |
| --- | --- |
| B1 exact admission | Same admit/refuse, error precedence and rollback as the reference on real SQLite stores. Preserve same-source, old/new identities/endpoints, all retained revisions, protected provenance, same-batch effects and persistent/TEMP trigger refusal. No extra supported-write refusals hidden as fallback. |
| B2 less repeated proof work | Deterministic RED measures actual VM/full-scan work or validator invocations; GREEN reduces repeated eligibility/proof work without deleting an obligation. Transaction-local decoded facts cannot replace queries whose answers earlier batch members change. Optimized and full-reference outcomes agree after every generated transition. |
| B3 exact transactional envelope | Actual canonical row/body/maximum, vector-mapping and all metadata-column totals equal the original SQL reference after inserts, supersession, retained history, retention sweeps, updates and deletes. Test each threshold below/at/above, Unicode, embedded NUL, NULL, numeric metadata and maximum deletion; cold/hot initialization must preserve a retention transaction that reduces over-cap metadata below the cap. Evaluate file pages independently. No submitted-size estimate or subtraction from saturated values. |
| B4 authority and recovery | Certificate is bound to one connection and pending root job. Unknown mutations, failed/deferred commit, rollback, generation/scope changes, corruption and reopen cannot preserve unproved authority. Tests force each invalidation, same-connection bypass, external commit (including the COMMIT/publication race) and lost certificate; exact reconstruction or original refusal follows. No local state authorizes durable completion. |
| B5 retained physical/service safety | Full exact admission/reopen/finalization proofs, independent FTS/main/WAL/sink residue and survivor witnesses, mapped non-head vectors, crash/reopen, redacted fence, idle worker, priority/wake, pinned readers and close/join tests pass. Same singleton/envelope/write class and actual TRUNCATE requirement. |
| B6 integrated benefit | Against the frozen retained baseline, each family has three valid pairs: at least 25% lower median public erase duration and strictly more median completed writes; every pair improves duration without fewer writes or reads, extra Engine refusals/errors or incomplete operations. All matched controls serve all offers; every erase completes actual physical proof. |
| B7 truthful outcome | Preserve every attempt and budget/resource outcome. Report work counts separately from service, idle/control cost and CPU availability. A failed control or performance gate is unqualified, with no automatic replacement or threshold relaxation. |

## Sequential implementation with TDD

1. **Attribution and effect inventory.** Reuse Slice 84 observers/driver. Add only
   counters needed to distinguish redundant queries from required scans; observe
   them on baseline and candidate identically. Prove the counter with a failing
   test. Select exact query improvements only where the actual plan supports them.
2. **Eligibility/proof work.** RED each selected behavior separately, commit or
   stage the failing oracle, then GREEN. First use existing indexes/query shapes
   and transaction-local decoded-root reuse. Where full proof repetition remains,
   implement only the design's effect-preserving certificate transition, with
   exact fallback. Do not substitute FTS rowids, MATCH or live-head-only queries.
3. **Transactional envelope totals.** Implement the same connection-bound owner
   with real changed-row SQL deltas and commit publication. RED missing effects,
   rollback/reopen/corruption and threshold behavior individually. This target is
   required even if step 2 yields a useful improvement. Tests compare totals with
   the full reference after every generated transition, not with hand-built totals.
4. **Review and independent verification.** Astra reviews code and the finalized
   effect inventory. Independently rerun decisive RED/GREEN and retained safety
   suites, including real commit failure and process exit. Scoped checks during
   repairs; no full regression rerun for a narrow finding.
5. **Measure only after cheap gates.** Work reduction and differential/physical
   correctness must pass before timed screening. Freeze source, binaries, inputs,
   commands, observers and budget. Stop on failed screening; do not automatically
   start a replacement candidate or a larger campaign.
6. **Finalize.** One final workspace gate for the final code phase, with scoped
   repairs/resumption as appropriate. Write a concise status that distinguishes
   both implementation targets, safety, work reduction and service qualification.
   Update release views only once execution/allocation is actually established.

## Proposed measurement protocol — not yet executed

Reuse original 10k-owner/five-revision/10%-target masters, release builds,
performance runtime and fixed provider. Eight readers at 100 Hz, one writer at
100 Hz for node/edge or 20 Hz for property; 15-second offer windows and 90-second
whole-cell cap. Both arms retain the same observers and exact physical witness.

Propose one 30-minute active measurement allowance, 8-GiB sampled RSS and 12-GiB
owned scratch ceilings. Include fixture/setup, profiling, validation and shutdown;
pause only for implementation/review. No automatic clock resets or retries.
At most three baseline diagnostic cells, only if deterministic query attribution
cannot answer selection questions. Then run one paired erase/control screen per
family: baseline erase/control plus candidate erase/control, twelve cells total.
These are predeclared repetition zero, not disposable warmups. A valid first pair
must improve duration with no fewer reads/writes and no additional refusals,
errors or incomplete operations. Failed control/accounting/physical screening
stops qualification; retain the negative outcome.

If every family passes screening, run two more paired repetitions with matched
controls and alternating arm order: 24 additional cells, plus one baseline/candidate
idle-erasure pair per family (six cells). Maximum 45 cells including diagnostics,
rather than automatically repeating Slice 84's 54-cell campaign. The idle pairs
retain physical proof and run without foreground offers. Deterministic cold/reopen
work-count tests are separate from timed service claims. B6 governs final medians and all pairs. No new Phase 82 comparison is
required; its historical results remain context, not new-candidate measurements.
Report every scheduled/issued/completed/refused/missed/unfinished/unknown offer;
300 property writes cannot qualify p99. No pooling across repetitions/protocols.
This budget/protocol must be approved with execution scope before any new timing.

## Quality fit

The quality roadmap favors real-database differential/state-machine tests,
threshold boundaries, selected commit/crash faults and resource cleanup here.
Use independent physical markers in addition to the shared validator. Defer
exhaustive VFS/OOM/MC/DC, new frameworks and broad tuning grids. New evidence is
Slice 84's measured certification cost and failed scheduler-only qualification;
it motivates less repeated exact work, not relaxed safety or extra machinery.

## Execution reconciliation — 2026-11-03

Owner corrected the target to 84b and authorized implementation of both targets
and the proposed bounded screening protocol. Baseline `77e2de796` retains 84's
observers/tools and original Slice 83 production policy; it also fixes a stale
WAL test selector. No performance suggestion has qualified yet. Astra completed
source-level effect review; concrete inventory is in the design. Work proceeds
with one writer in the existing development checkout and read-only reviewers.
No new branch/worktree or public contract change is needed.
