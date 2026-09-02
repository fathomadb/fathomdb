---
title: 0.8.25 Slice 35 — frozen reads and eligibility design
status: REVIEWED_MAX_ENVELOPE_SCOPE_NARROWED
design_version: 2
target_release: 0.8.25
depends_on: 30
readiness_blocked_on: Slice 7 architecture activation
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 35 — frozen reads and eligibility design

> **0.8.25 implementation boundary:** Eligibility-before-ranking remains
> complete; frozen reads are optional and compact. Full snapshot lease and
> retention machinery is deferred by the
> [scope adjustment](../../scope-adjustment-2026-09-02.md). Ordinary reads
> must not acquire snapshot overhead. Reconcile this maximum-envelope design
> before READY review.

## Authority and comparison

Owns R25/AC25-35, Memex needs 7/8 and the contract half of 21,
N25-02/N25-03, and A25-01/A25-05/A25-06. It extends accepted `ReadView` and
the closed filter grammar. It adds neither semantic policy nor raw SQL and does
not hold a SQLite reader transaction between calls.

| Need | Existing behavior | Decision |
| --- | --- | --- |
| Cross-operation boundary | Every call opens a snapshot; `ReadView` is policy | Persist an Engine-minted observable `FrozenSnapshot`. |
| Pre-ranking eligibility | Separate equality/range subsets | One closed indexed grammar, compiled before every candidate/frontier cap. |
| Mutation races | Successive calls can cross boundaries | Bind canonical boundary, UTC validity instant, eligibility, and projection state. |
| Evolution | Existing view/filter codecs | Versioned closed requests; Slice 15 response/wire rules. |

Accepted filter and `ReadView` designs are **reused**. The proposed 0.8.11
unification is **evidence only**; this is the 0.8.25 authority. Architecture v2
governs after Slice 7.

## Public and wire contract

```text
EligibilityEnvelopeV1 { schema_version: 1, all: [EligibilityPredicateV1] }
EligibilityPredicateV1 =
  Eq { projection, field, value }
  | Range { projection, field, lower?, upper?, lower_inclusive, upper_inclusive }
  | AnyOf { projection, field, values[1..256] }
  | Exists { projection, field, exists }
ReadContextV1 { schema_version: 1, view: ReadView,
                eligibility: EligibilityEnvelopeV1,
                snapshot: FrozenSnapshotToken? }
ProjectionBindingV1 { schema_version: 1, projection_name,
  declaration_sha256, source_boundary, projection_cursor,
  terminal_state_sha256 }
FrozenSnapshotV1 { schema_version: 1, token, valid_as_of,
  canonical_boundary, eligibility_sha256, projection_bindings[], expires_at }
FreezeReadRequestV1 { schema_version: 1, view, eligibility,
                      requested_ttl_seconds? }
```

`freeze_read` returns the frozen record. New context-bearing search/list/graph/
page/evidence methods accept `ReadContextV1`; old methods synthesize an
unfrozen context. **V1 requires exact equality with the originating canonical
view and eligibility envelope.** No subset, “narrower,” or intersection
semantics exist in 0.8.25. Canonical JSON sorts object keys and AND predicates
by `(projection, field, variant, canonical value)` after rejecting duplicates;
its SHA-256 is the equality authority.

Failures are `UnsupportedEligibility`, `InvalidEligibility`,
`SnapshotUnavailable`, `SnapshotMismatch`, `SnapshotDrifted`, and
`SnapshotExpired`. All public/persisted objects inherit Slice 15 request,
response, unknown-field/variant, u64, error, SDK, Windows, registry, and golden-
fixture rules. Opaque tokens carry an internal format version.

## Lifetime and persistence

Snapshots use persisted UTC Unix seconds. Default TTL is 900 seconds; callers
may request 60–3600 seconds. The Engine clamps nothing: values outside that
range reject `InvalidSnapshotTtl`. `expires_at = created_at + ttl`; a token is
expired when `now >= expires_at`. Tokens cannot renew or extend. Cursors,
evidence receipts, and trace leases may expire earlier but never later than the
snapshot.

Add `_fathomdb_frozen_snapshots`, keyed by random 128-bit ID, storing version,
database identity, boundary/time, canonical view JSON, eligibility digest,
`ProjectionBindingV1` JSON, created/expiry, and invalidation reason. The token
contains version/database/snapshot ID and HMAC-SHA-256. The per-database key is
generated once in `_fathomdb_open_state` and survives restart.

Expiry check precedes row lookup when authenticated token time proves expiry,
so before and after pruning it returns `SnapshotExpired`. A missing unexpired
row returns `SnapshotUnavailable`. On open and every 1,024 successful mints,
maintenance deletes at most 1,000 expired rows; failure to prune does not fail a
read. No user row is rewritten.

## Projection binding and transaction flow

Before Slice 40, `ProjectionBindingV1` uses the strongest existing durable
identity: declaration hash, enrolled source boundary, durable projection
cursor, and digest of terminal rows through that cursor. Consumption compares
all fields. Rebuild/change yields `SnapshotDrifted`; it never rebinds.

Slice 40 atomically records a one-to-one mapping from each unexpired V1 binding
to a generation only when declaration, source boundary, cursor, terminal digest,
and projection rows are equivalent. Consumption then uses that generation. No
unique equivalent maps to `ProjectionGenerationUnavailable`; mismatch maps to
`SnapshotDrifted`. Tokens and stored V1 bindings never change.

`freeze_read` resolves time once, reads boundary/bindings in one deferred
transaction, validates indexes, then commits the lease before return. Consumers
authenticate/load it, require exact context, recheck current lifecycle/erasure/
access fences, constrain to its high-water/time, and verify projection binding.
Non-reproducible state fails typed; no call silently advances.

## Eligibility invariants

- Projection/field must be declared; membership/existence require a native
  indexed projection.
- Canonical predicates precede order/limit, vector predicates are pre-KNN, FTS
  predicates constrain seeds, and graph predicates constrain seeds/frontiers.
- Predicates are parameterized implicit-AND and never SDK/post-cap emulated.
- Empty eligibility preserves SQL, ordering, hit shape, and allocation cost.
- A token identifies a boundary but never authorizes; current fences always win.

## Tests and verification

RED/GREEN fixtures cover mutation, supersession, validity, rebuild, restart,
exact-context mismatch, all expiry boundaries, pruning equivalence, tamper/
database mismatch, pre-Slice-40 drift, successful/ambiguous generation mapping,
and every predicate variant. Eligible-below-cap and query-plan tests cover
lexical/vector/graph. Golden Rust/Python/TypeScript/wire, Windows CPU/native,
compact-default, CUDA pre-KNN, and registry smokes pass.

Routes: fast, heavy, all, all-feature, Windows Rust/Python/Node, CUDA preflight/
rehearsal, and registry search. Operator/live-model are N/A. Status remains
`REVIEWED_BLOCKED_ON_SLICE_7`, blocked on Slice 7; no implementation-shaping decision remains.
