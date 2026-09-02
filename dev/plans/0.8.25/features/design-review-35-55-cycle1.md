---
title: 0.8.25 independent design re-review — Slices 35–55 cycle 1
status: COMPLETE
review_cycle: 1
reviewed_on: 2026-09-01
verdict: CHANGES_REQUIRED
---

# Independent design re-review — Slices 35–55 cycle 1

## Verdict

**CHANGES_REQUIRED.** FIX-1 fully resolves the Slice 35, 40, 45, and original
Slice 50 findings. Slice 55 still has three P1 implementation-shaping gaps, and
one new Slice 50 P2 privacy/schema ambiguity remains. The designs correctly
remain `DRAFT_REVIEW` and Slice 7 gated.

No semantic policy moved into FathomDB. Historical designs and receipts remain
preserved. No P3 finding is recorded.

## Original-finding verification

| Finding | Cycle-1 result | Evidence |
| --- | --- | --- |
| DR35-01 exact eligibility | RESOLVED | Exact canonical originating context is required in Slices 35/50; subset semantics are explicitly absent. |
| DR35-02 snapshot lifetime | RESOLVED | UTC seconds, 900s default, 60–3600s bounds, no renewal, expiry precedence, and bounded pruning are fixed. |
| DR35-03 pre-generation binding | RESOLVED | `ProjectionBindingV1` and one-to-one proven Slice 40 mapping are defined; zero/multiple matches fail typed. |
| DR40-01 activation/readiness | RESOLVED | Serving role and readiness are separate; activation requires an exact complete ready prefix. |
| DR40-02 applicability | RESOLVED | Immutable manifest and explicit applicable/skipped work rows define `ready_through`. |
| DR40-03 all-generation erasure | RESOLVED | Serving, legacy-serving, building, retired, failed, and work rows are scrubbed. |
| DR45-01 current authority | RESOLVED | Every continuation supplies exact current context and rechecks current fences. |
| DR45-02 graph-page scope | RESOLVED | Slice 45 is limited to one-hop direct adjacency; Slice 60 owns expansion/path paging. |
| DR45-03 wire rules | RESOLVED | Public records are versioned and explicitly inherit Slice 15 wire/parity rules. |
| DR50-01 evidence carrier | RESOLVED | Separate strict equal-length/index-and-identity sidecar; whole-request failure; default hit/result unchanged. |
| DR50-02 receipt durability | RESOLVED | Atomic receipt/hit persistence, caps, restart, lease-equal expiry, and erasure cleanup are defined. |
| DR50-03 non-disclosure | RESOLVED | Pre-authorization outcomes collapse to fixed `EvidenceUnavailable`; specific integrity states follow authorization. |
| DR50-04 completeness | RESOLVED | Complete under 256/64 caps or refuse; no partial continuation. |
| DR55-01 deterministic continuation | **UNRESOLVED** | Persisted frontier exists, but response/retry transaction semantics can still omit a committed page. See C1-55-01. |
| DR55-02 bounded integrity/repair | **PARTIAL** | Types/actions are closed and bounded, but job snapshot and reverse-index cutover remain undefined. See C1-55-02/03. |

## Cycle-1 findings

### C1-55-01 — P1 — A committed trace page can be lost on retry

**Location:** `slice-55/design.md`, **Deterministic continuation**, lines 55–72.

The lease advances its frontier/visited state before the response necessarily
reaches the caller. The text says a crash after commit resumes at the stored
ordinal. If the commit succeeds but the response is lost, retrying the same
cursor advances past that page, producing an omission. Concurrent uses of the
same cursor can likewise race unless one loses with a defined replay outcome.
This does not meet deterministic/resumable trace pagination.

**Required correction:** persist an immutable page result and next cursor keyed
by `(trace_lease_id, input_ordinal)` in the same transaction that advances the
frontier. Reusing the same authenticated cursor must return that exact stored
page and next cursor, including after crash or concurrent duplicate calls.
Only the returned next cursor advances. Retain page records until the trace
lease expires and erase them with the lease. Add lost-response and concurrent
same-cursor tests.

### C1-55-02 — P1 — Multi-page integrity findings have no frozen authority boundary

**Location:** `slice-55/design.md`, **Integrity jobs and repair contract**, lines
80–111.

`IntegrityCheckRequestV1` and `IntegrityJobStatusV1` carry no snapshot,
database/write boundary, projection generation set, or strict operator view.
The prose later refers to a “job snapshot,” but never defines how it is minted
or reproduced. A multi-page check can therefore combine pre- and post-mutation
state, generate false orphan/index findings, and feed an unsafe repair plan.

**Required correction:** define an Engine-minted integrity boundary at job
creation containing database identity, canonical write boundary, fixed strict
liveness/validity instant, and applicable projection generations. Persist it
on the job/status/findings and require every page to reproduce it or end
`incomplete` with a typed drift/unavailable reason. Repair plans must name this
same boundary and revalidate current authority before acceptance.

### C1-55-03 — P1 — Large reverse-index repair exposes partial derived state

**Location:** `slice-55/design.md`, **Integrity jobs and repair contract**, lines
103–111.

For more than 10,000 reverse-index rows, regeneration becomes a bounded job,
but the design does not state whether it writes the live index incrementally,
uses a shadow generation, or fences dependent reads. Incremental live writes
can break reciprocal traces during repair; a crash can leave a permanently
partial index even though forward dependency sets remain authoritative.

**Required correction:** rebuild the reverse index into a new Engine-owned
generation/shadow table from the frozen integrity boundary, verify reciprocal
equivalence, then atomically switch the active index identity. On failure or
crash, retain the old active index or fail affected reverse reads closed; never
serve a partially rebuilt index. Define restart cleanup and receipt state for
building, verified, activated, failed, and retired generations.

### C1-50-01 — P2 — Evidence receipt identity fields contradict the privacy contract

**Location:** `slice-50/design.md`, **Evidence receipt and reference
persistence**, lines 63–71.

Hit rows store “source/dependency IDs needed for resolution” while the same
paragraph forbids natural keys. Existing `source_id` values may be application
identifiers rather than Slice 15 opaque revision IDs, so implementers must
choose whether to persist a potential low-entropy identifier. Resolution does
not require duplicating free-form source identity in the receipt because the
artifact/source revision can reach governed provenance.

**Required correction:** enumerate receipt identity fields precisely and limit
them to Slice 15 opaque non-PII artifact/source revision IDs, Slice 20 dependency
set revision IDs, and Engine-generated event/projection IDs. Do not persist
free-form `source_id`, logical/natural keys, or payload-derived IDs in evidence
receipt tables. Resolve authorized source identity from governed provenance
after current authority succeeds. Add a fixture with path-like/source-session
identifiers and prove they do not appear in receipt storage.

## Boundary and readiness checks

- Semantic-policy boundary: **PASS**.
- Historical design/receipt preservation: **PASS**.
- Slice 15 wire-rule adoption: **PASS**, subject only to C1-50-01's field
  enumeration.
- Dependency sequencing through Slice 50: **PASS**.
- Slice 55 continuation/integrity sequencing: **CHANGES_REQUIRED**.
- READY status: correctly blocked on Slice 7 and this review.
