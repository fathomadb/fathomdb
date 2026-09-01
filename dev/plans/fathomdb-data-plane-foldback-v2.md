---
title: FathomDB data-plane foldback plan v2
status: PROPOSED
plan_version: 2
target_release: 0.8.25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# FathomDB data-plane foldback plan v2

## Outcome

Deliver the complete performance and Memex data-plane foldback without moving
semantic policy into FathomDB. The sequence is intentionally linear: each slice
creates the identity, dependency, snapshot, or evidence contract required by
the next. Slices 0–7 prepare the repository and decisions under
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md). Feature work
begins at Slice 10.

## Common delivery contract

Every feature slice follows the same bounded loop:

1. Write numbered requirements and slice-local falsifiable acceptance criteria.
   Map every criterion to a named test or measurement before implementation.
2. Write a design grounded in the reviewed architecture, current ADRs, public
   interfaces, and code—not architecture prose alone.
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
Slice 20 adds canonical-to-derived, derived-to-derived, and source-set
dependencies with caller-declared liveness rules and bidirectional lookup.

Tests cover restart/reindex identity, locator/hash integrity, invalid
references, cycles, multi-source removal, and cross-SDK wire round trips.

### Slices 25–30 — actuation and lifecycle closure

Slice 25 adds one atomic semantic batch for caller-decided canonical/derived
writes, dependencies, facts/edges, lifecycle actions, consolidation verdicts,
and metadata. The complete receipt reports both committed consequences and
whole-batch refusal reasons. Operation IDs make retries idempotent.

Slice 30 propagates lifecycle and erasure through dependencies, fences
incomplete work, and proves no active/searchable orphan remains. Tests inject
mid-operation failures and verify rollback, retry, projection closure, and
multi-source liveness rules.

### Slices 35–45 — frozen reads, readiness, pagination, and current state

Slice 35 introduces an Engine-minted frozen snapshot and extends the existing
allowlisted predicate grammar with only natively indexed membership/existence.
Eligibility runs before candidate truncation on lexical, vector, and graph
paths.

Slice 40 adds durable projection-generation identity and mutation/readiness
correlation. Slice 45 uses both contracts for opaque, stable ordered cursors on
canonical list, graph, and current-state reads, including governed point/page
access to existing `operational_state`. `latest_state` remains a consumer
concept. Ranked search stays bounded top-K.

Tests cover snapshot mutation races, validity boundaries, unsupported
predicates, native query plans, cursor mismatch/expiry/drift, duplicates,
omissions, and current-state replacement.

### Slices 50–55 — evidence, explanation, and integrity

Slice 50 adds opt-in evidence references and resolution under the originating
snapshot and eligibility envelope. Slice 55 adds forward/back provenance,
inclusion/exclusion explanation, receipt correlation, dependency-orphan checks,
and governed operator maintenance.

Tests cover current, superseded, inactive, invisible, erased, mismatched, and
unavailable evidence; stale references cannot reveal bytes. Bare search hits
retain their shape and cost.

### Slice 60 — constrained combined graph expansion

Extend the existing combined-expansion path with query/explicit seeds,
direction, edge kind, target kind, indexed predicates, frozen snapshot,
bounded deterministic continuation, and exact seed/edge/path evidence. Do not
reintroduce the rejected GRAPH-01 exact-anchor treatment.

### Slices 65–70 — benchmark-gated deterministic selection

Slice 65 evaluates entity/alias matching, duplicate suppression, diversity,
complementarity, coverage, and candidate fusion. Slice 70 evaluates temporal
retrieval and associative graph diffusion. Every treatment is named, bounded,
opt-in, preregistered, and compared with the accepted default. A treatment that
misses quality, lifecycle, or efficiency boundaries is recorded as rejected and
does not ship as a default.

### Slice 75 — integrated closure

Run installed cross-SDK and wire parity, including Windows CPU/native proof,
plus cold/steady concurrency, evidence-resolution, pagination, dependency
mutation, mutation-to-ready, erasure propagation, projection-generation,
storage/resource, and rebuild-cost measurements. Add a retrieval-only native
`Engine.search` global witness and keep answer-system metrics separate.

## Slice 3 draft contract allocation

These identifiers are local planning labels, not accepted global `REQ-*` or
`AC-*` entries. Each owning slice must refine and accept its contract before
implementation.

| Slice | Requirement package | Mandatory proof package |
| ---: | --- | --- |
| 10 | R25-10 executable measurement-layer classification | Receipt schema negative cases, GLOBAL-01 reclassification, native retrieval-only witness |
| 15 | R25-15 revision identity and canonical source provenance | Restart/reindex properties, locator/hash rejection, SDK/wire round trips |
| 20 | R25-20 explicit single/multi-source dependencies | Invalid reference/cycle rejection, bidirectional lookup, liveness-rule removal |
| 25 | R25-25 atomic caller-decided semantic batch and complete receipt | RED partial-failure injection, idempotent replay, cross-SDK receipt parity |
| 30 | R25-30 dependency-aware lifecycle and erasure closure | State-transition matrix, crash/restart/resume, stale-index and no-orphan proof |
| 35 | R25-35 frozen snapshots and pre-ranking eligibility | Mutation/validity races, unsupported predicates, native query-plan proof |
| 40 | R25-40 durable projection generations/readiness correlation | Restart and wrong-generation negative tests, mutation-to-ready receipt proof |
| 45 | R25-45 opaque ordered pages and governed `operational_state` | Duplicate/omission race, cursor mismatch/expiry/drift, point/page agreement |
| 50 | R25-50 eligibility-bound source-complete evidence | Exact-byte resolution plus invisible/erased/stale/mismatched non-disclosure |
| 55 | R25-55 provenance tracing, explanation, and integrity | Reciprocal traces, deterministic exclusions, injected-orphan/projection faults |
| 60 | R25-60 constrained combined graph expansion | Constraint-before-truncation, deterministic continuation, exact path/lifecycle evidence |
| 65 | R25-65 qualified deterministic candidate selection | Replayable quality/efficiency receipts and default-promotion guard |
| 70 | R25-70 qualified temporal and associative retrieval | Changed-fact/time-scoped/multi-hop quality with accepted-default regression guard |
| 75 | R25-75 integrated parity, concurrency, lifecycle, performance, and evaluation closure | Registry-installed cross-SDK fixtures and cold/steady distribution/resource receipts |

## Slice 4 architecture constraints

The architecture review adds these mandatory refinements to the owning slice
plans:

- Slice 15 defines UTF-8 byte locators, immutable revision binding, hash
  algorithm, and versioned wire evolution.
- Slices 20/30 define a bounded dependency-liveness grammar and test every
  source-removal consequence without assigning semantic truth to the Engine.
- Slice 35 specifies observable frozen-snapshot semantics with typed
  unavailable/drift/expiry outcomes; it does not assume a permanently-held
  SQLite reader transaction.
- Slice 45 exposes current `operational_state`; `latest_state` remains a
  consumer concept, and cursors remain distinct from ranked top-K.
- Slices 35/55/60 apply eligibility and graph constraints before truncation and
  distinguish ineligible, not-selected, unavailable, and degraded outcomes.
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

Feature-local parity and correctness land in Slices 15–70. Slice 75 runs the
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
- live-model: a named, budgeted, non-default receipt-producing route only when
  a treatment depends on a downloaded model or provider;
- registry-installed: the release workflow's fresh-machine Python, npm/native,
  and CLI smoke routes for every changed public binding or wire contract.

Windows CPU/native behavior is proved feature-locally for every public or
persisted Slice 15–70 change. Windows CUDA remains outside 0.8.25. Slice 75
checks that the named receipts exist and agree; it does not supply missing
feature-local proof. A route that is not applicable must be marked `N/A` with a
reason, rather than omitted.

## Dependency and workload policy

The exact ladder and states live in [`plan-0.8.25.md`](plan-0.8.25.md). Prework
is sequential through Slice 7. No feature slice may start early or run in
parallel across an unmet dependency. Cross-SDK parity is part of each public
feature slice; Slice 75 audits it rather than postponing it.

After every slice plan is written, assess P0/P1/P2 value, schema/API risk,
verification cost, and critical path. Keep every identified item allocated
until the HITL explicitly changes scope. Planning an overweight release is
preferable to silently dropping a consumer obligation.

## Stop conditions

Stop for an unresolved P1/P2 review finding, unsafe or ambiguous atomicity,
stale/uneraseable derived state, unbounded reads, pre-ranking eligibility drift,
snapshot/cursor ambiguity, binding mismatch, a failed benchmark boundary, or a
semantic-policy requirement incorrectly assigned to FathomDB.
