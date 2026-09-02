---
title: 0.8.25 Slice 10–30 independent design re-review — cycle 1
status: COMPLETE
review_cycle: 1
reviewed_on: 2026-09-01
verdict: CHANGES_REQUIRED
source_review: dev/plans/0.8.25/features/design-review-10-30.md
source_fix: dev/plans/0.8.25/features/design-review-10-30-fix1.md
---

# Slice 10–30 independent design re-review — cycle 1

## Verdict

**CHANGES_REQUIRED.** FIX-1 materially resolves the cycle-0 design gaps, but
two new/unresolved P1 and two P2 implementation-shaping findings remain. The
five designs must remain `DRAFT_REVIEW`; no reviewed design was edited.

## Cycle-0 finding verification

| Cycle-0 finding | Cycle-1 evidence | Status |
| --- | --- | --- |
| Slice 10 exact GLOBAL-01 paths | Per-arm execution witnesses, coverage-backed negative state, exact source hashes, and all four path classifications are defined. | RESOLVED |
| Slice 15 artifact/source revision conflation | `ArtifactRevisionId`, `SourceRevisionId`, and revision-bound source links are distinct. | RESOLVED |
| Slice 15 total legacy migration | Table-class matrix distinguishes known source, exact derived, incomplete derived/legacy, and Engine projections without fabricated completeness. | RESOLVED |
| Slice 15 identity validation/scope | Database-wide uniqueness, length, caller/Engine namespaces, and conflict behavior are intended, but their grammar is contradictory; see C1-15-01. | PARTIAL |
| Slice 20 caller-visible liveness | Engine strict-current liveness is independent of caller access/eligibility/relaxations. | RESOLVED |
| Slice 20 reference/set persistence | Logical-current versus pinned reference, immutable set revisions, current pointer, retire/delete, and active-cycle semantics are explicit. | RESOLVED |
| Slice 25 future Slice 35/40 types | Baseline write boundaries and projection work intents are Slice-25-owned; later additions are explicitly successors. | RESOLVED |
| Slice 25 refusal crash stability | Admission journal captures request/boundary, terminal refusal commits with validation, and open recovers admitted plans before later writes. | RESOLVED |
| Slice 25 undefined large closure | Every lifecycle batch creates barrier/closure intent; no size-dependent branch remains. | RESOLVED |
| Slice 30 erased bytes under `any_surviving` | Physical erasure now fences/removes inseparable caller artifacts and requires an externally submitted clean revision. | RESOLVED |
| Slice 30 total-impact cap | Total impact is unlimited; bounded work pages and resumable incomplete state replace permanent refusal. | RESOLVED |
| Slice 30 post-enumeration dependency race | All mutation/dependency/projection paths reject new derivation from barriered roots and proof checks post-admission dependencies. | RESOLVED for writes; read visibility has a separate new blocker C1-30-01. |

## Cycle-1 findings

### C1-15-01 — P2 — Revision-ID grammar rejects the Engine's own namespace

**File/section:** `slice-15/design.md`, **Identity and provenance contract** and
**Total legacy migration**.

The common syntax requires IDs to match
`[A-Za-z0-9][A-Za-z0-9._:-]*`, while Engine-minted and migrated IDs are
`_fdb:r:*` and `_fdb:m:*`. Both start with `_` and are invalid under the stated
grammar. Implementations and SDK validators could either reject migrated rows
or silently diverge.

**Correction:** define separate closed validators: caller IDs use the stated
leading-alphanumeric grammar and reject reserved prefixes; Engine IDs use
`_fdb:r:<ULID>` or `_fdb:m:<64-lower-hex>`. The shared stored column accepts
the union, still capped at 128 bytes. Pin positive/negative Rust/Python/
TypeScript/wire fixtures for all three forms.

### C1-20-01 — P2 — Validity scheduling conflates activation with loss of liveness

**File/section:** `slice-20/design.md`, **Structural liveness and validity
boundaries**.

The design queues the “earliest future validity boundary” and asks Slice 30 to
admit closure when it becomes due. A future `valid_from` can make a member live;
a `valid_until` can make it non-live. Treating both as closure either permits
automatic reactivation (which belongs to caller policy) or creates a permanent
`dependency_closure_due` refusal for a boundary that requires no closure.

**Correction:** queue only boundaries that can change an active dependency from
live to non-live, principally `valid_until`; writes/supersession handle immediate
losses. Registration of an active dependent whose set is not live at the fixed
instant rejects or requires the caller to create it inactive/pending. A future
`valid_from` never auto-reactivates; the caller may later request the existing
transition to active, which re-evaluates strict liveness then. Pin both boundary
directions and the absence of automatic reactivation.

### C1-30-01 — P1 — Paged closure does not fence undiscovered dependents on reads

**File/section:** `slice-30/design.md`, **Barrier, paged propagation, and race
closure** and **Invariants**.

The root/source barrier blocks new writes and propagation fences artifacts as
each 1,000-row page discovers them. A pre-existing transitive dependent not yet
visited is therefore neither directly fenced nor rejected by the documented
read path, despite the invariant that no impacted artifact is visible after
barrier admission. Large closures can leak derived records between admission
and their work page.

**Correction:** define one enforceable conservative read mechanism. The
recommended shape is a pre-ranking `closure_visibility_guard` on every governed
canonical/search/graph/projection read: an artifact is ineligible if its active
dependency ancestry reaches a barriered root/source, even before a work row is
materialized. The guard uses indexed Slice 20 dependencies, is checked before
candidate/frontier truncation, and fails closed on limit/storage error. Keep it
until proof retires the barrier. Alternatively, atomically enumerate and fence
the complete transitive closure at admission; do not combine paged discovery
with the current visibility claim without such a guard. Add a >1-page fixture
that queries the last undiscovered descendant immediately after admission.

### C1-30-02 — P1 — Erasure omits the new semantic-operation journal

**File/section:** `slice-25/design.md`, **Crash-stable idempotency state
machine**; `slice-30/design.md`, **Barrier, paged propagation, and race closure**
and **Tests**.

Slice 25 adds a durable journal whose nonterminal rows contain the complete
prepared request, potentially including canonical/derived source bytes. Slice
30's erasure proof enumerates canonical, dependency, FTS/vector/graph,
telemetry, and WAL stores but not this journal. An admitted or recovery-required
operation can therefore retain erased bytes after a successful erasure claim.

**Correction:** add source/revision reference indexes to the operation journal,
strip prepared request bytes when an operation becomes terminal, and include
nonterminal journal payloads in barrier conflict and erasure handling. Source
erasure must either deterministically finish/rollback a referencing admitted
operation before propagation or fail closed while its barrier remains; it may
not delete state needed for crash recovery. Completion proof and raw-table
canaries must cover journal request payloads and terminal receipts, which retain
only the non-content idempotency/audit minimum.

## P3 observations

None. The remaining issues affect executable contracts rather than editorial
quality.

## Cycle-2 acceptance

Cycle 2 may PASS only when the four findings above are corrected and tests are
mapped for:

1. caller/Engine/migrated revision-ID validation parity;
2. `valid_from` versus `valid_until` liveness behavior;
3. immediate invisibility of a not-yet-processed transitive descendant; and
4. source-byte erasure from admitted and terminal semantic-operation journals.
