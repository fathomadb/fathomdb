---
title: FathomDB 0.8.26 Slice 40 — atomic derived-edge actuation
status: APPROVED_FOR_IMPLEMENTATION
---

# Slice 40 plan — atomic derived-edge actuation

## Outcome

Promote the accepted Slice 35 changed-in-place V1 actuation candidate to the
real schema-34, fresh-database-only product boundary. A fresh database can
atomically commit and replay the complete derived graph-authoring unit; every
public engine-open route refuses a non-current database before product
mutation. No historical migration or parallel API generation is introduced.

## Entry reconciliation and disposition

The draft was last changed at `7ae1905f` on 2026-09-13. The implementation
baseline is the clean `release/0.8.26` worktree at `54f1ad99`, after Slice 35
and its adversarial and verification remediations. The following changes,
assigned surfaces, and allocated items were reviewed before approval:

1. Slices 9, 10, 15, 20, 30, and 35 completed after the draft. Slice 20 added
   frozen graph evidence; Slice 30 hardened read-only operator inspection.
   Neither changes the derived-edge actuation contract. Slice 35 implemented
   the fifth V1 operation throughout the engine, bindings, conformance,
   projection, erasure, replay, and integrity paths and measured it within the
   accepted bounds.
2. Slice 35's retained candidate now satisfies the original R26-40A through
   R26-40D work: complete prospective endpoints, canonical edge reuse, one
   transaction and digest, bounded exact receipts/source references, rollback,
   restart, concurrency, and Python/TypeScript parity. Post-close commits
   `5ecb52db` and `9a81a75c` additionally bind affected revisions and reverse
   source references to the exact replayed request. Reimplementing those paths
   here would be duplicate and overbuilt.
3. Slice 35 deliberately left only a read-only classifier parameterized with
   prototype schema 34 and a test-only no-op migration. Production remains at
   `SCHEMA_VERSION = 33`; ordinary `Engine::open` still migrates older
   databases. Slice 40 therefore owns the real content-free step 34 and the
   mandatory fresh-only check on the shared public open path.
4. The assigned implementation surfaces are
   `fathomdb-schema::{SCHEMA_VERSION,MIGRATIONS}`, the shared engine open path,
   `EngineOpenError::IncompatibleSchemaVersion`, the Rust/PyO3/N-API open
   adapters, and the current wire/binding contract documents. The explicit
   test-only migration seam remains available for migration-mechanism tests;
   it is not a supported product compatibility route.
5. The Slice 3–5 allocations and D26-03 through D26-05 are fully represented:
   changed-in-place V1, complete prospective endpoints, compact receipt, and
   fresh databases only. No HITL decision remains open. Slice 45/46 retain
   architecture/design-document convergence; Slice 50 retains clean package,
   platform, and non-publishing release verification.
6. The source and interface blast radius contains stale forward-migration
   oracles and prose. Tests whose sole product assertion is automatic opening
   of a pre-34 database must be retired or moved to the explicit test-only
   migration seam. The current interface contract must change in this slice;
   broad public-document convergence remains allocated to Slices 45/46.

Disposition: **approve the narrowed plan after independent design review**. Retain the
reviewed Slice 35 actuation implementation unchanged except for defects exposed
by cutover tests. Implement schema/open activation, update the directly owned
contract, and run focused actuation/open regressions plus canonical
`agent-verify`. Do not repeat the Slice 35 performance campaign or pre-empt
Slice 45, 46, or 50.

## Need, requirements, and acceptance

Need N26-04: a caller can atomically commit a derived node, its canonical
dependency, and a provenance-bearing semantic edge. For 0.8.26, that caller
must create a fresh database rather than accidentally upgrading or interpreting
an earlier database.

### Requirements

- **R26-40A — changed-in-place grammar:** Retain the reviewed single five-
  operation `ActuationOperationV1` grammar across bindings with no parallel V2
  pair. Slice 35 satisfied this requirement; Slice 40 treats it as a regression
  obligation.
- **R26-40B — atomic graph unit:** Retain all-or-none commit of node,
  dependency, provenance-bearing edge, replay record, receipt, and projection
  work. Slice 35 satisfied this requirement; Slice 40 regresses it after the
  open cutover.
- **R26-40C — endpoint semantics:** Retain complete-prospective-state endpoint
  validation, including later same-batch endpoints and final lifecycle state.
  Slice 35 satisfied this requirement; Slice 40 regresses it.
- **R26-40D — current receipt contract:** Retain one changed-in-place V1
  receipt, digest, replay, operation-ID, and integrity contract without
  historical compatibility. Slice 35 and its post-close remediation satisfied
  this requirement; Slice 40 regresses it.
- **R26-40E — fresh 0.8.26 databases only:**
  - **E1:** Advance the product schema from 33 to 34 with one content-free
    bootstrap marker; step 34 changes no historical rows.
  - **E2:** Missing and zero-length paths bootstrap at schema 34. A non-empty
    database opens only when its effective committed `PRAGMA user_version` is
    exactly 34. All public engine-open routes share this policy and use the
    existing typed incompatible-schema error.
  - **E3:** Acquire the product lock without rewriting its metadata, then make
    the authoritative freshness decision while holding it and before
    write-mode SQLite open, connection PRAGMAs, migration, recovery,
    projection reconciliation, worker startup, or domain writes. Refusal keeps
    the database and SQLite sidecars byte-identical. Existing lock bytes are
    unchanged; when the lock is absent, the attempt may establish only the
    empty persistent lock namespace required for race-safe future opens.
  - **E4:** Retain no default or shipped product migration, translator,
    historical receipt/replay reader, version router, or version-by-version
    matrix. Move the custom-migration helper from all debug builds to the
    explicitly opt-in, non-forwarded `migration-test-hooks` engine feature.
    Its `#[doc(hidden)]` public spelling exists only so Rust integration tests
    can exercise migration mechanics; it is not re-exported or compiled by any
    facade, binding, CLI, or shipping feature set.
- **R26-40F — current restart integrity:** A schema-34 database reopens and
  replays an identical edge-bearing actuation receipt exactly; changed bytes
  conflict. The pre-open check must observe current committed WAL state rather
  than treating an uncheckpointed current database as an older one.

### Acceptance criteria

- **AC26-40A:** The one current V1 grammar executes across bindings and no V2
  parser, method, or router exists; focused Slice 35 binding checks remain
  green.
- **AC26-40B:** Transactional fault, duplicate, restart, dependency, lifecycle,
  erasure, and projection tests for the mixed graph unit remain green.
- **AC26-40C:** Missing endpoint precedence, later endpoint success, and final
  inactive endpoint refusal remain deterministic and atomic.
- **AC26-40D:** Fresh-database current-contract receipt, replay, and integrity
  tests pass and no historical compatibility path exists.
- **AC26-40E:**
  - **E1:** `SCHEMA_VERSION == 34`, `MIGRATIONS` ends contiguously at content-
    free step 34, fresh open reports `0 -> 34`, and clean reopen reports
    `34 -> 34` with no migration steps.
  - **E2:** The shared public route refuses schema 33, schema 35, and non-empty
    zero-version SQLite as `IncompatibleSchemaVersion { seen, supported: 34 }`
    without migration events.
  - **E3:** Clean, WAL/SHM-bearing, and WAL-without-SHM schema-33 database and
    SQLite sidecars are byte-identical after refusal; an attempt-created SHM is
    removed. Existing lock bytes are exact. A missing lock may become one empty
    persistent lock file and no other byte may change. A deterministic locked-
    admission test proves a competing product opener cannot turn a fresh
    candidate into a migrated schema-33 database.
  - **E4:** Rust direct and migration-event opens, PyO3, N-API, and CLI use the
    same policy/error mapping; default/facade/binding builds do not compile or
    forward the feature-gated custom-migration seam.
- **AC26-40F:** A current schema-34 database with committed state still in WAL
  is admitted and recovered; an active second open retains the existing
  `DatabaseLocked` precedence. Default engine, facade, and binding builds do
  not compile or forward the feature-gated custom-migration seam.
- **AC26-40G:** The current wire and binding interface contracts state schema
  34 and fresh-only refusal. Focused tests, `git diff --check`, contract/docs
  validators, and `./scripts/agent-verify.sh` pass; package/platform matrices
  remain Slice 50 work.

## TDD RED/GREEN delivery

1. Obtain independent read-only review of [`design.md`](design.md), resolve all
   material findings, and mark the plan/design approved before implementation.
2. **RED-1 — schema and open boundary:** add a dedicated Slice 40 integration
   suite covering AC26-40E/F, including exact no-mutation snapshots and the
   active-lock/current-WAL cases. Add an internal deterministic locked-
   admission race test. Preserve the failing test commit.
3. **GREEN-1 — cutover:** add content-free migration step 34 and route every
   public open through one pre-mutation current-schema check. Keep the
   integration-test-only `open_with_migrations_for_test` explicitly outside
   that product policy and compile it only under the non-forwarded
   `migration-test-hooks` feature.
4. **RED-2/GREEN-2 — binding and graph unit:** add focused PyO3/N-API policy
   mapping and fresh schema-34 mixed-unit/replay tests, then make only the
   smallest implementation or fixture changes required. Existing passing
   Slice 35 oracles are not rewritten.
5. Update the directly owned wire/Rust/Python/TypeScript interface contracts.
   Remove or reroute stale automatic-upgrade tests only when they assert the
   superseded public behavior; retain migration-runner tests through the
   feature-gated seam.
6. Run focused schema/open, actuation, binding, migration-policy, and contract
   checks. Obtain independent code review of the actual diff, resolve findings
   with visible RED/GREEN evidence, then use a different read-only subagent for
   final focused verification and canonical `agent-verify`.
7. Write `status.md`, advance the single-writer release state to Slice 45 with
   exact commits/evidence, regenerate its views, and preserve the release
   worktree. No temporary branch or worktree is planned.

## Stop gates

Stop on a partial graph unit, replay/digest ambiguity, a V2/router, an earlier
database mutation or upgrade, a current-WAL false refusal, weakened corruption
or lock precedence, unbounded work, migration-test access from production, or
scope that belongs to Slice 45, 46, or 50.
