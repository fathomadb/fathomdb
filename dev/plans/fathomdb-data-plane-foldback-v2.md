---
title: FathomDB data-plane foldback plan v2
status: ACTIVE
plan_version: 2
target_release: 0.8.25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# FathomDB data-plane foldback plan v2

## Outcome

Deliver the essential performance and Memex data-plane foldback without moving
semantic policy or unproven retrieval algorithms into FathomDB. The complete
needs/design inventory remains durable; the approved
[`0.8.25 scope adjustment`](0.8.25/scope-adjustment-2026-09-02.md) defines the
narrower implementation boundary and future allocation. The active sequence is
intentionally linear. Slices 0–7 prepare the repository and decisions under
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md). Feature work
begins at Slice 10.

## Common delivery contract

Every feature slice follows the same bounded loop:

1. Write numbered requirements and slice-local falsifiable acceptance criteria.
   Map every criterion to a named test or measurement before implementation.
2. Write or reconcile a design grounded in the reviewed architecture, current
   ADRs, public interfaces, and code—not architecture prose alone.
3. Obtain an independent design review. Allow at most three documented FIX-n
   cycles; unresolved P1/P2 findings stop the slice.
4. Implement with TDD: preserve the RED test before implementation, make it
   GREEN, then refactor without changing the oracle.
5. Obtain an independent implementation review. Allow at most four documented
   FIX-n cycles, while honoring the standing same-failure retry stop.
6. Run focused real-database tests, lifecycle/erasure tests, binding parity,
   and repository verification. Store exact commands, exit codes, and commit
   identity in a concise verification record.

Do not mint global `dev/acceptance.md` IDs without separate authorization. Do
not regenerate historical benchmark results as test oracles.

## Slice groups

### Slices 0–7 — prework and approved repository preparation

Slices 0–5 inspect environment/infrastructure, dependencies, repository cruft,
product contracts/architecture, code alignment, and verification adequacy.
Except for Slice 0's isolated release setup, they write proposals and take no
implementation action. Slice 6 scores every proposal, conducts interactive HITL
decisions, and produces a subagent-reviewed Slice 7 plan with at most two FIX-n
cycles. Slice 7 implements only approved repository preparation with TDD,
independent review/verification, a durable status record, and safe worktree
cleanup. Product features discovered in prework are added directly to their
owning Slice 10+ plan and are never implemented by Slice 7.

### Slice 10 — executable measurement classification

Make experiment receipts distinguish data-plane, semantic-control-plane, and
end-to-end metrics. Record whether `Engine.search` ran, shared/differing
components, and invalid mixed-layer claims. Reclassify existing GLOBAL-01
receipts without rewriting their measured evidence.

### Slices 15–20 — identity and dependency foundation

Slice 15 adds immutable record revisions, source versions, exact locators,
hashes, and the missing Rust identity exports while preserving `IdSpace`.
Slice 20 adds one canonical-source-to-derived dependency form with bounded
bidirectional lookup and structural validation. Multi-source sets, general
derived-to-derived graphs, and configurable liveness move to 0.8.26.

Tests cover restart/reindex identity, locator/hash integrity, invalid
references, invalid dependency roles/self-reference, source removal, and
cross-SDK wire round trips.

### Slices 25–30 — actuation and lifecycle closure

Slice 25 adds one bounded atomic batch for caller-decided canonical/derived
writes, core dependencies, and lifecycle actions. Its compact receipt reports
operation identity, committed/refused outcome, affected IDs, resulting
boundary, and readiness/closure references. Operation IDs make retries
idempotent. Broader operations and complete consequence receipts move to
0.8.26.

Slice 30 propagates lifecycle and erasure through dependencies, fences
incomplete work, and proves no active/searchable orphan remains. Tests inject
mid-operation failures and verify rollback, retry, projection closure, and
core source-to-derived closure.

### Slices 35–45 — frozen reads, readiness, pagination, and current state

Slice 35 makes eligibility-before-ranking uniform and offers a compact
Engine-minted frozen read context when requested. Ordinary reads do not require
it. The existing allowlisted predicate grammar grows only where a predicate is
natively indexed. Eligibility runs before candidate truncation on lexical,
vector, and graph paths; full snapshot leases move to 0.8.27.

Slice 40 adds core durable projection-generation identity and compact
mutation/readiness correlation. Slice 45 adds stable bounded continuation for
canonical and current-state reads, including governed point/page access to
existing `operational_state`. `latest_state` remains a consumer concept.
Ranked search stays bounded top-K. Full cursor leases and generalized graph
pagination move to 0.8.27.

Tests cover optional-read-context mutation races, validity boundaries,
unsupported predicates, native query plans, request/order cursor mismatch,
duplicates, omissions, and current-state replacement.

### Slices 50–55 — evidence, explanation, and integrity

Slice 50 adds compact opt-in evidence references and resolution under the
original or equivalent eligibility envelope. Slice 55 adds bounded forward/back
provenance, compact inclusion/degradation explanation, and dependency-orphan/
projection checks. Persisted evidence/trace leases, broad exclusion tracing,
and repair orchestration are outside 0.8.25.

Tests cover current, superseded, inactive, invisible, erased, mismatched, and
unavailable evidence; stale references cannot reveal bytes. Bare search hits
retain their shape and cost.

### Slice 60 — constrained combined graph expansion

Make the existing combined-expansion path honor query/explicit seeds,
direction, edge kind, target kind, indexed eligibility, bounds, and one read
context with deterministic one-page results. Rich continuation and replayable
path evidence move to 0.8.28. Do not reintroduce the rejected GRAPH-01
exact-anchor treatment.

### Slices 65–70 — reallocated reviewed evidence

Slices 65 and 70 are not active 0.8.25 implementation slices. Their reviewed
designs remain evidence. Manual profile/temporal work is reconsidered in
0.8.28; candidate-selection experiments at the 0.8.29 planning review; and
associative/routing experiments at the 0.8.31 planning review. No default
changes.

### Slice 75 — integrated closure

Run installed cross-SDK/wire parity, including Windows CPU/native proof, plus
representative concurrency, evidence, pagination, dependency, readiness,
lifecycle, and selected performance regression checks. Add a retrieval-only
native `Engine.search` witness and keep answer-system metrics separate.
Exhaustive scale-by-feature-by-CUDA matrices are experimental, not required.

## Slice 3 draft contract allocation

These identifiers are local planning labels, not accepted global `REQ-*` or
`AC-*` entries. Each owning slice must refine and accept its contract before
implementation.

| Slice | Requirement package | Mandatory proof package |
| ---: | --- | --- |
| 10 | R25-10 executable measurement-layer classification | Receipt schema negative cases, GLOBAL-01 reclassification, native retrieval-only witness |
| 15 | R25-15 revision identity and canonical source provenance | Restart/reindex properties, locator/hash rejection, SDK/wire round trips |
| 20 | R25-20 core canonical-source-to-derived dependencies | Invalid reference/cycle rejection and bounded bidirectional lookup; broader forms move to 0.8.26 |
| 25 | R25-25 bounded caller-decided semantic batch and compact receipt | RED partial-failure injection, idempotent replay, cross-SDK receipt parity; broader forms move to 0.8.26 |
| 30 | R25-30 dependency-aware lifecycle and erasure closure | State-transition matrix, crash/restart/resume, stale-index and no-orphan proof |
| 35 | R25-35 pre-ranking eligibility and optional frozen reads | Mutation/validity races, unsupported predicates, native query-plan proof, no mandatory snapshot overhead |
| 40 | R25-40 durable projection generations/readiness correlation | Restart and wrong-generation negative tests, mutation-to-ready receipt proof |
| 45 | R25-45 minimal canonical/state pages and governed `operational_state` | Duplicate/omission race and point/page agreement; full cursor leases move to 0.8.27 |
| 50 | R25-50 compact eligibility-bound source-complete evidence | Exact-byte resolution plus invisible/erased/stale/mismatched non-disclosure |
| 55 | R25-55 basic provenance tracing, explanation, and integrity | Bounded reciprocal traces, compact degradation reasons, injected-orphan/projection faults |
| 60 | R25-60 minimal constrained combined graph parity | Constraint-before-truncation and deterministic bounded one-page evidence |
| 65 | Reallocated experimental evidence | Candidate-selection review at 0.8.29; no 0.8.25 implementation |
| 70 | Reallocated experimental evidence | Temporal reconsideration at 0.8.28; associative review at 0.8.31 |
| 75 | R25-75 trimmed parity, lifecycle, performance, and evaluation closure | Installed cross-SDK fixtures plus representative concurrency/resource receipts |

## Slice 4 architecture constraints

The architecture review adds these mandatory refinements to the owning slice
plans:

- Slice 15 defines UTF-8 byte locators, immutable revision binding, hash
  algorithm, and versioned wire evolution.
- Slices 20/30 define and close the core canonical-source-to-derived dependency
  form without assigning semantic truth to the Engine. Multi-source liveness
  moves to 0.8.26.
- Slice 35 specifies optional observable frozen-read semantics with typed
  unavailable/drift/expiry outcomes; it does not impose snapshots on ordinary
  reads or assume a permanently held SQLite reader transaction.
- Slice 45 exposes current `operational_state`; `latest_state` remains a
  consumer concept, and cursors remain distinct from ranked top-K.
- Slices 35/55/60 apply eligibility and graph constraints before truncation and
  expose compact inclusion/degradation state. Expanded deterministic
  ineligible/not-selected explanation moves to 0.8.28.
- Slice 50 preserves compact default hits and creates/resolves evidence only
  under the originating visibility envelope.
- Every public/persisted feature slice defines version/unknown-field behavior
  and ships feature-local Rust/Python/TypeScript parity before Slice 75 audits
  the combined surface.

## Slice 5 verification constraints

Every owning slice must cover positive, typed-negative, failure/rollback,
close/reopen, concurrent mutation, and lifecycle/erasure behavior proportional
to risk. Codec, dependency, cursor, projection, recovery, and cross-SDK
round-trip layers use property-based tests. The database is real, unsupported
inputs fail closed, default compact-search behavior has an explicit
non-regression test, and a skipped external/platform route is recorded as
missing evidence rather than a pass.

Feature-local parity and correctness land in active Slices 15–60. Slice 75 runs the
installed cross-SDK and integrated workload audit; it must not become a holding
area for tests omitted by their owning feature slice.

Each owning plan must select the applicable routes below and name the exact
command, workflow job, fixture, and receipt path before its design may become
READY:

- local fast: `bash scripts/agent-verify.sh --tier=fast`;
- local heavy/all: `bash scripts/agent-verify.sh --tier=heavy` and, at closure,
  `bash scripts/agent-verify.sh --tier=all`;
- Windows CPU/native: the applicable `windows-latest` Rust, Python, and Node
  jobs in `.github/workflows/ci.yml` or `.github/workflows/release.yml`;
- all-feature/operator: focused Cargo commands with the feature's actual
  feature set, plus the repository all-feature route when compatible;
- GPU/CUDA: `cuda-contract-preflight` and the applicable CUDA package-rehearsal
  route when the feature can change dense, rerank, fusion, or graph behavior;
- live-model: not selected by the retained 0.8.25 ladder; future experimental
  treatments must separately name and budget such a route;
- packaged: isolated consumers install locally built wheel, npm/native
  tarball, crate/CLI artifacts without source-tree fallback;
- registry-installed: post-publication fresh-machine smokes only, after
  separate publication authorization; they do not block slice READY.

Windows CPU/native behavior is proved feature-locally for every public or
persisted change in active Slices 15–60. Windows CUDA remains outside 0.8.25.
Slice 75 checks that the named receipts exist and agree; it does not supply
missing feature-local proof. A route that is not applicable must be marked
`N/A` with a reason, rather than omitted.

## Dependency and workload policy

The exact ladder and states live in [`plan-0.8.25.md`](plan-0.8.25.md). Prework
is sequential through Slice 7. No feature slice may start early or run in
parallel across an unmet dependency. Cross-SDK parity is part of each public
feature slice; Slice 75 audits it rather than postponing it.

After every slice plan is written, assess P0/P1/P2 value, schema/API risk,
verification cost, and critical path. The 2026-09-02 owner decision performed
that reduction; removed work remains allocated in future drafts or the
experimental review inventory rather than silently disappearing.

## Stop conditions

Stop for an unresolved P1/P2 review finding, unsafe or ambiguous atomicity,
stale/uneraseable derived state, unbounded reads, pre-ranking eligibility drift,
snapshot/cursor ambiguity, binding mismatch, a failed benchmark boundary, or a
semantic-policy requirement incorrectly assigned to FathomDB.
