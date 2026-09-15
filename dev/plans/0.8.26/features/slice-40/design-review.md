---
title: FathomDB 0.8.26 Slice 40 — independent design review
status: PASS
reviewed_on: 2026-09-15
---

# Slice 40 independent design review

The read-only reviewer first returned FAIL because the original draft repeated
the Slice 35 actuation implementation and did not define the production fresh-
database cutover. The draft was replaced with a code-grounded plan that treats
the accepted Slice 35 result as a regression obligation and limits new product
work to schema 34 and public-open admission.

The reviewer then identified three material design defects:

1. A version check before lock acquisition allowed a cooperating schema-33
   opener to win between check and writable open. The final design acquires the
   product lock without rewriting metadata and makes one authoritative version
   decision while holding it.
2. The Slice 35 `immutable=1` prototype ignored committed WAL state, while a
   simple read-only replacement could create SHM. The final design uses WAL-
   aware read-only SQLite under the product lock, restores attempt-created SHM
   on refusal, preserves exact durable bytes, and tests current and old WAL
   states both with and without pre-existing SHM.
3. `open_with_migrations_for_test` was public in every debug build. The final
   design compiles it only under the dedicated non-forwarded
   `migration-test-hooks` feature and requires default/facade/binding surface
   guards.

A traceability rereview also caught recycled requirement identifiers. The
final plan preserves N26-04 and R26-40A through R26-40E with their Slice 3
meanings, records A–D as already-satisfied regression obligations, and refines
R26-40E with schema/open subcriteria. The last rereview confirmed the explicit
schema-33 WAL-without-SHM restoration oracle and returned PASS with no
remaining blocker.

After code review exposed the split-inode race caused by unlinking an advisory
lock path, the design was adjusted to make the lock namespace persistent. A
final design rereview confirmed that classification remains entirely under the
held lock; refusal preserves every database/SQLite-sidecar and existing-lock
byte; and an absent lock may leave only one empty lock file. The reviewer found
the contract internally consistent, race-safe, covered with lock/SHM/WAL
present-and-absent tests, and not overbuilt, and returned **PASS**.

## GPT-6 Astra medium post-implementation review

A fresh GPT-6 Astra medium design review found two gaps in the approved design
and its evidence:

1. **P1:** `admit_current_database` could open SQLite before process-global
   runtime configuration. In a fresh process this made a clean schema-34 or
   current-WAL reopen fail `RuntimeConfiguration(TooLate)`; a schema-33 refusal
   could likewise poison a later fresh creation. The required implementation
   change was to configure SQLite under the already-held product lock and
   before classification, without weakening `DatabaseLocked` precedence. The
   required tests were child-process first-operation clean reopen, WAL reopen
   with and without SHM, and schema-33 refusal followed by fresh creation in
   the same child.
2. **P2:** the existing older-wins case did not prove the claimed current-wins
   race ordering. The required implementation change was a minimal test-only
   synchronization point after lock acquisition and before classification or
   writable open. The required test pauses there, has a competing older opener
   attempt the same product lock and conditionally install schema 33, proves it
   cannot, and verifies the current opener's schema/migration result.

The design and plan now state the corrected ordering and exact evidence. The
implementation and focused tests contain both remediations; final Astra
rereview is recorded below after verification.

The same GPT-6 Astra medium reviewer rereviewed RED `7eadeb94` and remediated
HEAD `68152886` and returned **PASS** with no remaining P1/P2 finding. It
confirmed `DatabaseLocked` precedence, runtime-before-admission ordering, the
three fresh-process cases, both race directions, untouched pending-lock
metadata, and exact `0 -> 34` winner evidence. The reviewer also confirmed the
path-scoped `cfg(test)` hook adds no shipped API or production behavior.
