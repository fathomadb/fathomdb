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
   design compiles it only under the engine's existing non-forwarded
   `test-hooks` feature and requires default/facade/binding surface guards.

A traceability rereview also caught recycled requirement identifiers. The
final plan preserves N26-04 and R26-40A through R26-40E with their Slice 3
meanings, records A–D as already-satisfied regression obligations, and refines
R26-40E with schema/open subcriteria. The last rereview confirmed the explicit
schema-33 WAL-without-SHM restoration oracle and returned PASS with no
remaining blocker.
