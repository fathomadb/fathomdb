# Slice 20 requirements

## Authority and allocation

Release-local refinement of R29-3P-06/D29-BLK1→O3-06 in
[completed3](../../prework/slice-3/requirements.md), A4-02 in
[completed4](../../prework/slice-4/alignment-matrix.md), and E0/O3-06 in
[completed5](../../prework/slice-5/traceability.md). Scope follows the current
[decision log](../../prework/slice-9/decision-log.md) and
[direct mandate](../../direct-execution.md). The plan's specific TDD prerequisite
is unresolved; scope is already approved. Canonical requirements and locked
acceptance remain unchanged.

| ID / need | Required outcome | Owning acceptance |
| --- | --- | --- |
| S20-R1 / N1 truthful harness evidence | Classify every historical subcase against current source; retain baseline/source/build/platform/failure provenance and exact named control. Never close an entire TC from a single repaired subcase or report a skip as execution. | S20-A1 |
| S20-R2 / N1 parallel isolation | A separate engine cannot steal/overwrite the intended evidence sidecar/resolver rendezvous. Hooks do not retain test/worker state after completion, timeout, cancellation or panic. SQLite process configuration precedes any raw initialization in affected parent processes. | S20-A2 |
| S20-R3 / N2 queue safety | A reached Ready→pre-accounting pause allows a real idle/LRU drain; resumption rejects stale phase/epoch before engine work and returns to the original open wait. Preserve absolute wait deadline, bounded queue, permit/queued/abandoned accounting, cancellation and running-work retention. | S20-A3 |
| S20-R4 / N3 diagnostic truth | Identify exact historical warning item/configuration if still present, then remove stale code or gate genuinely test-only ownership. Non-test release and seam-enabled tests remain independently exercised; no blanket allows or lint exclusion. | S20-A4 |
| S20-R5 / N4 tripwire monitoring | T2 tests read actual production RRF_K30, text3, vector1, embedder name fathomdb-bge-small-en-v1.5 and max sequence512. Failure names the re-evaluation record; changing expectation requires its recorded assessment. T1/T3 remain standing monitors, not new20 campaigns. | S20-A5 |
| S20-R6 / N1 Windows startup evidence | Map both historical failures and the hang to current accepted-probe/timeout/visibility/opener controls; preserve failed-open typed errors, profile/worker/SQLite/admission ownership and bounded termination on an actual Windows build. | S20-A6 |

[Design](design.md) names exact old/new seams and [test map](test-map.md) names
selectors, independent expected outcomes and TDD classification. No schema,
format, public product error/result field or accepted numerical target changes.
Private test-hook public functions/methods need documented input/lifetime/cleanup
contract when changed, with a test-only note in
[the Rust interface](../../../../interfaces/rust.md) in the same PR. Retain the
existing free export paths with required Engine argument; update callers
atomically without compatibility shims and retain default-feature refusal. Production interface changes discovered during diagnosis require their
own accepted ADR/interface revision before implementation.

## Completed10 input and source-entry conditions

Actual104dcbcbe92a1eca730d146b99ab17d444df6217 passed fresh independent Astra
implementation review, integrated and closed at parent d7d928f8e49d7af6a792385534f4e991c13232b3.
Current JSON records10 COMPLETE_ON_RELEASE_BRANCH,20 NOT_STARTED/next20 and
preparation acceptance PASS only. Its assembled214-registration/component
qualification does not discharge any20 criterion or claim full release readiness.
Actual final source,18-pair register, setup guards, pytest-child fix and evidence
limits have been reread. No predecessor-closure hold remains.

Before source entry parent obtains updated design review, strict closed10
state/ref preflight and creates a fresh20 checkout. Owner verifies actual base,
ownership, helper --check-ownership before mutators, own noneditable native/tools/
receipt and measured resource budget; wait for implementation GO. No borrow of10
venv/modules/native artifacts or mutable Cargo target. Run source tests only in
that owned tree. Final214 registrations are inherited; do not weaken registry/
partition evidence or call missing/zero-selector checks product passes.

Independent design review may accept genuine repair scope without waiving the
explicit owner-held race23/T2 coverage choice. A3/A5 remain pending either an
actual exception answer plus implementation/proof, or explicit defer disposition.

Source phase uses the actual design PASS/source GO and owned entry evidence in
[execution](execution.md). Parent-owned release controls remain frozen. Only
actual demonstrated repairs are implemented; original R1/R4 hypotheses that did
not reproduce are recorded rather than forced into code changes. R3/R5 new
coverage policy and R6 actual Windows evidence remain explicit holds, unaffected
by repair-subset scoped/full/native qualification.
