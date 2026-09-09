---
title: Slice 71B — General write regression
status: COMPLETE
date: 2026-09-08
parent_slice: 71
---

# Slice 71B — General write regression

## Authority, scope, and precedence

The owner approved this sub-plan integration on 2026-09-08 and directed that
the write regression be completed now within Slice 71. This resumes the write
work independently of the unresolved AC-013/AC-072 read-latency disposition.
71B is an internal work-package label, not a new release slice or version.
The release ladder remains 71 → 72 → 73 → 75.

This document owns the amended write execution protocol, completion criteria,
and verification scope. It supersedes conflicting write sequencing, fixed
event counts, experiment arms, and blanket schema-change stops in the parent
[plan](plan.md), [design](design.md), and
[remaining-work outline](remaining-work-outline.md). Historical measurements,
receipts, schemas, and review verdicts remain immutable evidence of their
original candidates; they do not certify this amendment or a future fix.

Ordinary diagnostics, scoped implementation, and focused verification are
authorized without another permission handshake. Before collecting timing
evidence, commit the concrete versioned protocol, runner, receipt contract,
and validator, and obtain independent review. This integration does not itself
claim that a campaign has run or that an implementation design is reviewed.
AC-072 thresholds and its retained `environment_invalid` outcome are unchanged.
Do not rerun the read-latency campaign as a prerequisite for 71B.

## Evidence and hypotheses

Six retained Slice 35 campaigns show 10k acknowledgement increasing from
2.09–2.13 s to 3.82–4.28 s (81–104%). The narrow `b2bfb1f3..c3de8c59`
window leaves `write_inner` and `commit_batch` unchanged while introducing
schema-31 visibility triggers. This strongly implicates their interaction
with the existing writer and projector; it does not quantify each mechanism.

| Candidate cause | Introducing feature and value | Evidence boundary |
| --- | --- | --- |
| Per-row singleton generation updates | Slice 35 frozen reads: persistent, cross-process, raw-SQL-visible invalidation | Three updates per simple foreground node; drain activity must be counted separately. |
| Writer/projector contention | Slice 35 triggers interacting with the existing asynchronous projector | Hypothesis; measure lock waiting and transaction duration, not only seed/drain redistribution. |
| Repeated SQL/trigger compilation | Existing uncached `execute()` calls amplified by Slice 35 | Source-confirmed redundant preparation; production timing contribution unisolated. |
| Per-fire random nonce generation/encoding | Slice 45 branch-sensitive frozen authority | Later additive cost; cannot explain the initial Slice 35 regression. |
| Index/projection-generation maintenance | Slice 45 indexed pagination and Slice 40 serving identity/readiness | Secondary, fixture-dependent costs; count mutations before expanding investigation. |

The second investigator reported a scratch SQLite microbenchmark and a
since-reverted AC-013 diagnostic: 2,130 ms writes + 2,893 ms drain, with ending
generation 50,005 after 10k nodes. Reported synthetic costs were about 0.8 µs
per trigger execution, 5 µs trigger preparation, and 0.4–1.5 µs additional nonce
cost. These are preliminary reviewer-supplied observations until scripts,
exact source/runtime identities, starting generation, and raw logs are recovered
and checked. Missing artifacts do not block new controlled evidence; mark the
claims unverified and do not use them as acceptance evidence or timing bounds.

Three foreground fires and roughly five including drain describe different
boundaries. Use generation deltas, subtract setup activity, and attribute
background work; an ending counter alone is not an exact per-node count.
Artifact insertion is present in the ordinary node path. A five-fire aggregate
does not disprove its index maintenance. The partial canonical pagination
index receives no entries for `logical_id: None`.

## Phase 1 — Seal the execution protocol

Retain the Scale-02 10k fixture, no embedder, batch size 256, and pre-Slice-35
baseline `b2bfb1f318f58041144acb2356a6a4c9624068b9`. Include a separate
projection-active AC-013 seeder fixture with batches of 1,024 and baseline
`4fc1b890a11ebfaa8f11b15823656e856002807a`; bind its exact vector/runtime
configuration from the retained runner. This is a write/drain measurement,
not a new AC-072 search-latency campaign. Never pool these fixtures' timings.

Pin the unchanged current candidate and each treatment by Git identity; bind
toolchain, release build mode, SQLite/native runtime, fixture/configuration
digests, host controls, instrumentation, timeouts, and raw-log hashes. Use
fresh databases and isolated processes. Baseline checkouts/targets must be
isolated under the normal preflight and one-writer rules; never install an
editable package from a worktree into the shared environment.

Version the new manifest/receipt contract instead of modifying sealed old
receipts to accept new metrics. Reuse existing validators where compatible;
write deterministic rejection tests first for new fields/classification.
Pre-visibility generation/nonce observations remain null with the existing
`schema_predates_visibility_state` explanation, not fabricated zero deltas.

Start with three repetitions per arm. Two-arm comparisons retain order
`B,C,C,B,B,C`; preregister balanced orders for three-arm comparisons before
measurement. Use the existing host exclusions (load above half online CPUs,
available memory below 25%, swap activity, thermal throttling, competing
build/test/benchmark processes). For each primary timing metric, invalidate an
arm when `(max / min - 1) * 100 > 25`; reject nonpositive timings. Stop that
campaign on invalidity, retain all cells, and diagnose the environment. A
revised protocol must be recorded before any replacement campaign; no silent
retries, discarded outliers, or threshold changes after seeing results.

For Phase 5 recovery, the 2026-09-09 owner direction prospectively narrows
"primary timing metric" to match the actual completion criteria below. The
1–1,000-row cells use their registered median and conjunctive relative/absolute
limit; sub-millisecond scheduling range does not invalidate them. At 10k,
Scale-02 acknowledgement and total, and AC-013 total, retain the 25% spread
gate. AC-013 acknowledgement remains mandatory and reported but is diagnostic:
asynchronous projection can move the same work across the acknowledgement and
drain boundary. No performance limit or 25% threshold changes. Earlier invalid
observations remain retained and cannot be reused in the prospective run.

## Phase 2 — Attribute the regression

Measure acknowledgement, subsequent drain, and total ingest-to-drained wall
time with explicit endpoints. Record generation before/after setup, writes,
and drain; actual trigger executions where instrumentation permits; writer
and projector transaction counts; preparation/cache-miss/reprepare counts;
lock acquisition wait and transaction hold duration; CPU, peak RSS, database
and WAL bytes, checkpoints, errors, and host pressure. Overlapping background
work requires connection/event attribution, not subtraction presented as
exclusive foreground activity. Document instrumentation cost and use identical
instrumentation within pairs; confirm final timings without intrusive tracing.

The already-repeated >20% historical regression justifies attribution without
reopening the broad historical bisect. Begin with production, generation-only
(checked exhaustion retained, nonce removed), and no-op-body triggers on the
current candidate. Use three repetitions per arm and disposable diagnostic
databases. Keep production trigger inventory/open validation intact; any
post-open trigger replacement must be verified and restricted to diagnostics.
Qualify this seam for all projector connections before using it with the
projection-active fixture. If existing connections cannot safely execute the
counterfactual, review an isolated diagnostic-build seam instead. Altered
schemas/builds never constitute shipping candidates or acceptance evidence.

If preparation versus execution attribution remains unresolved, cross these
three arms with preparation reuse on/off, up to six configurations. Seal this
conditional extension before its measurements. Use narrow historical schema
boundaries only where they resolve remaining attribution; do not automatically
run every historical commit at every size.

Exit with controlled evidence naming the supported causes and residual cost.
A large no-op recovery does not prove the remainder is contention: that claim
requires reduced lock waiting/hold time. If hypotheses fail, retain the result
and review a bounded revised investigation; do not ship a speculative fix.

## Phase 3 — Select and independently review the correction

Choose the smallest correction supported by Phase 2, without preselecting a
headline fix:

- Prepared-statement reuse: target preparations proportional to distinct SQL,
  not rows. The three identified foreground inserts should require roughly
  three initial preparations for a homogeneous 10k batch instead of 30,000,
  absent schema invalidation or eviction. Measure the actual cache working set
  and projector paths. The reported 0.1–0.3 s saving is a hypothesis.
- Visibility coalescing, only if justified: target one generation/nonce advance
  per relevant engine transaction while preserving external-SQL trigger
  coverage. The target is measured row fires → measured transaction count T,
  not an assumed 20–30. A marker/guard design must explain connection scope,
  persistent-trigger compatibility with unmodified external SQLite clients,
  savepoints, errors, rollback, cleanup, all writers, and in-transaction frozen
  observations. Do not assume WAL isolation alone proves semantic equivalence.
- Nonce amortization belongs to a proved coalescing design. Preserve random
  branch sensitivity; never derive it solely from the generation counter.
- Index/metadata changes require measured benefit and retained indexed read
  plans/readiness semantics. Do not remove useful indexes by default.

An additive migration/manifest change preserving accepted guarantees is within
this work's authority after concrete design review and upgrade coverage.
Update schema, exact open-time trigger validation, and relevant design/ADR
records together. A change to an accepted guarantee requires an explicit owner
decision on the concrete proposal. No blanket permission request is needed
merely because the implementation needs an additive schema step.

## Phase 4 — Implement with deterministic RED/GREEN evidence

Record failing tests before product edits, stage or commit RED visibly, and
preserve their oracles during GREEN. Required affected invariants include:
committed relevant mutations invalidate prior contexts; rollback preserves
prior state; external SQLite and cross-process mutations cause drift;
generation exhaustion fails atomically; equal-count divergent copies have
different bindings; restart and savepoints are correct; every authoritative
table and virtual-table owner coupling remains covered. For coalescing,
explicitly test no unguarded mutation and no persisted suppression after error.
Reuse unaffected tests; add property coverage for changed round-trip layers.

## Phase 5 — Demonstrate recovery

Compare contemporaneously rerun historical baseline, unchanged current code,
and corrected candidate. Report each repetition, per-arm medians, absolute
milliseconds, relative changes, and fraction of regression recovered. Historical
2.5 s/2.4 s estimates are explanatory only, not fixed host-independent limits.

Completion requires the corrected Scale-02 10k median acknowledgement to be
within 20% of its matched historical baseline. Also require median total
ingest-to-drained time within 20% of the corresponding historical baseline
for each of the two fixtures. Retain acknowledgement separately for the
projection-active fixture; fast acknowledgement cannot hide slow completion.

Measure 1, 10, 100, 1,000, and 10,000 records. For 1–1,000 records, use one
write transaction per workload to expose interactive batch latency; the 10k
fixtures retain their original 256/1,024 batching. Record actual transaction
counts and final partial batches. Before measurement, seal matched smaller
fixture construction and any projector configuration required at these sizes.
For each small cell and both acknowledgement and total median latency, an
increase over the matched historical baseline is a regression when it exceeds
both 10% and 0.25 ms. Report unchanged-current comparisons as well; do not let
an improvement over an already-regressed current build substitute for recovery.

An unstable environment, unresolved correctness defect, or missed recovery
criterion leaves 71B incomplete. Retain the precise limitation and revise only
through a recorded prospective plan; never declare completion by documentation
or automatically defer the correction to Slice 75.

## Phase 6 — Review, verification, and closeout

Obtain independent code review on the exact candidate and a separate read-only
audit of evidence and focused verification. Serialize shared-state edits; use
isolated worktrees for concurrent implementers. Read-only reviewers may share
the checkout. Resolve blocking findings before closing 71B.

Documentation-only integration requires Markdown, local-link, JSON, and
generated-view checks. Harness changes require only affected contract and
syntax/type tests. Product changes require new RED/GREEN tests, affected
visibility/rollback/frozen/cross-process/projection suites, and focused check
and clippy for affected crates. No `agent-verify`, `scripts/check.sh`, long
stress, CUDA, Windows, packaged cross-SDK, or hosted CI in this work package;
Slice 75 owns the full release matrix.

### Verification budget — owner clarification

71B can complete with focused correctness tests and its bounded performance
evidence. Do not repeat broad suites after each edit, review fix, or document
change. Reuse prior passing results when their code/dependency paths are
unchanged; each new diff gets an explicit affected-test selection. Code review
and the separate evidence audit inspect retained results rather than each
launching another verification campaign.

The default for Slice 71 remains zero broad/full rounds. If the owner later
authorizes an exception, run one consolidated round on the final reviewed
candidate. At most one second round is permitted, only for a failure or a
subsequent change that invalidates the first round's coverage. The ceiling is
two rounds across all of Slice 71, not two per agent, fix, or work package;
count existing rounds before scheduling any new one. This ceiling does not
authorize otherwise deferred Windows/CUDA/package/hosted-CI work. A third
round requires a new owner decision with the remaining gap explained.

Record every verification invocation with command, source identity, selected
scope, reason, result, duration, and whether it consumes the broad-round
budget. A broad round is one predeclared consolidated suite campaign; splitting
the same broad coverage among agents does not reclassify it as focused. Each
sub-plan performance campaign has its own preregistered repetitions and is
not a license to rerun broad regression suites. On timeout or failure, diagnose
the affected check before any retry; never restart the entire matrix reflexively.

Retain protocol, attribution, selected design, TDD chronology, raw receipts,
review verdicts, and final disposition with exact source identities. Update
parent status and the release-state single writer, then regenerate its views.
71B completion alone neither passes AC-072 nor closes Slice 71. Slice 72 stays
dependency-blocked until all Slice 71 obligations have a disposition.

## Incorrect paths to avoid

- Treating synthetic microbenchmarks as production timing bounds or seed/drain
  redistribution alone as proof of lock contention.
- Equating trigger fires, preparations, transactions, and WAL frames. Coalescing
  does not inherently remove the singleton page's frame in each transaction.
- Assuming larger batches remove repeated preparation or treating all writes
  as having the same trigger count.
- Moving invalidation solely into `Engine.write`, weakening nonce randomness,
  or shipping diagnostic no-op triggers.
- Broad bisects/campaigns before the narrow hypotheses are exhausted, or
  declaring success from one 10k result without small-write measurements.

## Immediate next action

71B is complete at product candidate `eda95b07`; its retained performance
evidence and focused verification are recorded in
[71b-performance-recovery.md](71b-performance-recovery.md). The parent Slice
71 remains open for the separate AC-072 disposition, which 71B neither reran
nor waived.
