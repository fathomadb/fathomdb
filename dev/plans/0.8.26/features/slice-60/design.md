---
title: FathomDB 0.8.26 Slice 60 — current design-owner model
status: APPROVED
target_release: 0.8.26
---

# Slice 60 design — current profiles over historical baselines

## Authority and evidence order

Accepted ADRs decide policy. Requirements state need. `dev/interfaces/` owns
public contracts. Active architecture v2.2 owns system shape. Schema and product
code witness implementation; tests witness enforced edge cases. Maintained
topic designs explain how those authorities compose and do not override them.

Every normative section in the rewritten owners maps to a bounded cluster in
`inventory.md` with both code and test evidence. Exact fields/errors stay with
interfaces and `errors.md`; limits stay with `retrieval-result-limits.md`;
projection-generation repair stays jointly owned with `recovery-0.8.25.md`.

## Shared seam model

- retrieval owns candidate generation, ranking/fusion, view eligibility,
  frozen authority, evidence, graph expansion, and explanation;
- recovery owns operator diagnosis, maintenance, export, and loss-authorized
  recovery boundaries plus the exact CLI action inventory; and
- engine owns admission/runtime topology, canonical identity, transactions,
  cursors, projection publication topology, and shared storage seams.

Cross-links point to narrower owners rather than copying their detailed rules.
The final engine pass triggers a mandatory retrieval/recovery cross-read.

## Retrieval profile

The retrieval owner describes one typed current pipeline:

1. validate query, bound, filter, view/frozen context, explanation, and evidence
   requests before candidate work;
2. execute safe compiled lexical node/edge/projected-text and eligible vector
   branches, applying roles, validity, lifecycle, and frozen authority before
   truncation;
3. use shared `vector_default` binary shortlist plus exact-f32 rerank, fuse
   contributing arms by RRF, and retain typed fallback/degradation state;
4. apply only shipped optional recency, importance/confidence, graph-arm, and
   cross-encoder mechanisms; embedding and reranker device policies remain
   independent;
5. finalize opt-in explanation and compact evidence from the same snapshot; and
6. keep bounded graph expansion as its own governed operation with current or
   frozen context, eligibility-before-expansion, deterministic bounds, and
   frozen-only exact artifact evidence.

Standalone `rerank` is live in Python and TypeScript. It reuses cross-encoder
machinery but does not make reranking unconditional in search. Deferred tuning,
general graph continuation, and full-path evidence remain non-current.

## Recovery profile

The old “doctor is bit-preserving/read-only; recover mutates” shorthand is too
broad. Current actions are classified by effect:

- database-free diagnostics: `gpu`, `platform`, `reranker-gpu`;
- engine-backed diagnosis/readout: `check-integrity`, `verify-embedder`,
  `trace`, `dump-schema`, `dump-row-counts`, `dump-profile`, `dump-mutations`,
  `orphan-provenance`;
- immutable quiescent inspection: `data-plane-integrity`;
- artifact/cache work: `safe-export`, `warm-cache`;
- explicit non-lossy derived-vector maintenance: `recompute-mean`; and
- loss-authorized recovery: `truncate-wal`, `rebuild-vec0`,
  `rebuild-projections`, `excise-source`, and paired
  `excise-collection`/`excise-record-key`.

The operator roots stay absent from SDKs, but governed SDK `purge` and
`erase_source` coexist with reserved-namespace/op-store excision behind
recovery. Only `data-plane-integrity` is described as immutable/quiescent. Each
live recovery action emits one JSON object and success uses the accepted-loss
exit class.

The classification is not inferred around the accepted 0.6.0 ADR. A narrow
0.8.26 successor supersedes only its doctor-wide bit-preserving/read-only
clause. It grandfathers the already-shipped `recompute-mean` command as the one
atomic, non-lossy, derived-vector maintenance exception. Its command-owned
transaction derives the mean from retained uncentered
`vector_default.embedding` values, updates the stored mean, and
recreates/requantizes vector rows with governed metadata preserved. Failure
rolls that transaction back. Shared admission and dependency-closure pre-writer
maintenance remain governed by their existing owners and are not new doctor
authority. This does not authorize another mutating doctor command.
Data-loss-authorized CLI operator recovery/rebuild actions remain under
`recover --accept-data-loss`; governed SDK erasure remains separate.

## Engine profile

Schema 34 and the fresh-only public rule govern admission. A missing or
zero-length database is bootstrapped; a nonempty noncurrent database is refused
under the persistent lock before product mutation. Internal migration machinery
is not a public upgrade promise.

Canonical, dependency, lifecycle, operational, receipt, FTS, and vector state
share one SQLite database. Active canonical identity is `logical_id` alone,
not `(logical_id, kind)`; kind changes supersede rather than fork. Callers do
not supply a separate current `row_id`: committed per-row `write_cursor` values
remain positional identity, while logical/content/passage identity is distinct.

Caller mutations use the mutex-serialized primary writer connection and commit
a validated batch atomically. Async vector projection workers use separate
connections and totally order their write transactions through `commit_gate`.
Pooled readers own current/frozen read transactions and do not move onto a
write lane. Current receipts expose batch high-water cursor, per-row cursors,
and dangling endpoint count.

Write cursors order writes; projection/read boundaries state derived visibility;
neither is an immutable record revision. Shared `vector_default` plus authority
sidecars replaces the historical per-kind vector-table description. Exact
generation, readiness, and repair details stay with their narrower owners.

## Authorized operator recovery entry paths

The implementation review exposed a topology bug, not an accepted-policy
change. Public `Engine::open` must continue to reject corruption before
returning a handle, but the operator tool needs a separate path to act on the
WAL that open refused. One operator-feature-only free function in the engine
crate provides that path:

```rust
recover_truncate_wal(path: impl Into<PathBuf>)
    -> Result<TruncateWalReport, EngineOpenError>
```

The facade re-exports it only behind its existing `operator` feature for the
CLI; it is absent from the default Rust/Python/TypeScript SDK surfaces. The
function and additive report field are public operator-feature Rust contracts
owned by `interfaces/rust.md`; JSON and exit behavior are owned by
`interfaces/cli.md` and `design/recovery.md`.

### Corrupt-WAL truncation

The path-scoped WAL recovery function:

1. resolves the canonical database namespace, preflights an existing nonempty
   regular database, and does not bootstrap a missing path;
2. acquires and initializes the same persistent sidecar lock used by normal
   admission, then rechecks the existing/nonempty/regular-file facts while
   holding it;
3. refuses a nonempty rollback journal because WAL recovery does not authorize
   rollback-journal recovery;
4. validates the main file independently through an immutable, read-only,
   query-only connection: header probe, full schema traversal, schema-cookie
   observation, legacy-shape refusal, and—when the standalone cookie is
   current—the current Fathom schema invariants used by normal admission, so a
   counterfeit database stamped with `user_version == 34` is not accepted;
5. re-reads and records the WAL-header classification while holding the lock.
   A sidecar shorter than 32 bytes carries no committed frames; at least 32
   bytes with invalid masked magic or a non-power-of-two/out-of-range page size
   is the exact malformed state shared with `probe_wal_sidecar`; sidecar I/O
   errors fail rather than becoming "absent";
6. requires `user_version == 34` plus those invariants from the standalone main
   file before discarding a malformed WAL; for a healthy WAL, validates the
   same version and invariants through SQLite's effective main-plus-WAL view
   because current schema state may itself be checkpoint-pending, snapshotting
   and restoring transient SHM bytes if that preflight refuses recovery;
7. opens one recovery-only `mode=rw` SQLite connection without create
   permission, deliberately omitting only the public open path's WAL pre-probe,
   and runs
   `PRAGMA wal_checkpoint(TRUNCATE)`; and
8. releases every connection and the sidecar lock before returning.

The malformed branch is destructive and is reachable only after the CLI has
validated `--accept-data-loss`. SQLite, not raw filesystem code, owns WAL
locking and destructive checkpoint/discard. The only direct sidecar operation
is restoring the pre-probe SHM snapshot when a healthy-WAL validation refuses,
matching normal admission's byte-preserving refusal boundary. The report
retains SQLite's counters/status and adds
`discarded_corrupt_wal`. It is true only when the locked pre-probe classified a
malformed WAL and SQLite returned `Done`; it is false for healthy/absent WAL and
for `Busy`. Thus an empty healthy WAL cannot be confused with deliberate
discard, while an incomplete attempt cannot claim recovery. A busy checkpoint
maps to retryable exit `71`, not accepted-loss `64`.
SQLite checkpoint errors map through the existing WAL-replay open-error class;
no error or Busy result claims discard completion.
The function does not bootstrap, migrate, reconcile, load embedders, start
workers, accept a different corruption kind, or weaken public open. The final
connection cannot recreate a database that disappears between validation and
recovery. A subsequent normal open proves that the base database is otherwise
admissible.

### Standalone safe export

`doctor safe-export` remains the logical `VACUUM INTO` plus SHA-256 manifest
workflow on an admitted database. A malformed SQLite header cannot be parsed
into that self-contained logical artifact, so it remains a fail-closed
`E_CORRUPT_HEADER` result before export. The operator guidance preserves the
original and directs external forensic/SQLite recovery tooling; it no longer
claims `safe-export` can bypass the header failure. A raw byte copy would be a
different forensic artifact and is explicitly out of scope. Clean current
databases retain the existing artifact and manifest shape.

### Isolation and errors

The WAL function serializes against a live Engine through the established
canonical lock. Lock contention and busy checkpoint retain exit class `71`;
completed WAL recovery retains accepted-loss success `64`. A nonempty rollback
journal and missing, zero-length, effectively noncurrent, counterfeit-current,
or main-corrupt database use the existing unrecoverable class `70`; a malformed
WAL additionally requires its standalone main file to be current and to satisfy
current Fathom schema invariants. A healthy WAL may carry the current schema
cookie over an older standalone main, but its effective view must satisfy the
same invariants. Refused healthy-WAL validation restores transient SHM state so
the database, WAL, and SHM bytes remain unchanged. Malformed-header
`safe-export` also remains open corruption at `70`, not artifact-failure `66`.
Other recovery and doctor actions still require a successfully admitted Engine
because they depend on current canonical/schema invariants. No generic "open
corrupted database" handle is introduced.

## RED/GREEN proof shape

The RED is semantic and source-grounded: required current facts are absent and
specific disproven claims are present. GREEN requires the same probes to find
current facts and reject old claims. Lifecycle, Markdown, links, source diff,
and full repository verification remain separate oracles. No generated product
oracle or generalized Slice 65 checker is added.

The recovery addendum uses executable product RED tests in
`fathomdb-engine/tests/truncate_wal.rs` and
`fathomdb-cli/tests/recovery_cli.rs`. They bind public-open refusal before
recovery, acknowledged malformed-WAL recovery and reopen, missing/zero/
noncurrent/counterfeit-current/main-corrupt/rollback-journal/live-lock refusal
without database/WAL/SHM byte mutation, valid/absent WAL with a false discard
disposition, Busy exit `71`,
default-SDK nonpresence, and malformed-header safe-export failure with no
artifact or manifest. The pre-existing CLI test that allowed Busy with exit
`64` is changed first so a non-completed checkpoint cannot remain accepted as
success. A default-build compile-fail doctest proves that
`fathomdb::recover_truncate_wal` is absent without the `operator` feature.

## Compatibility and change class

The original owner reconciliation is explanatory/contract documentation. The
authorized addendum changes only operator-feature Rust engine/CLI dispatch and
focused tests. It adds no SDK operation, schema or migration, binding surface,
package dependency, or publication action. Historical rationale remains in Git
or behind explicit historical links rather than masquerading as current
behavior.
