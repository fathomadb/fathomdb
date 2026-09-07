---
title: 0.8.25 Slice 60 implementation review — cycle 2
status: FAIL
review_cycle: 2
candidate: 0a9c0683d12be69ac8bfc26e2cc12c6bc72ffd4e
---

# Slice 60 implementation review — cycle 2

## Verdict

**FAIL.** No P0 finding exists. Three P1 and three P2 findings remain. The
independent review was read-only and the exact candidate remained clean.

## Findings

1. **P1 — The rendezvous remains global and cancellation-unsafe.** It stores a
   process-global arbitrary closure, has no request identity, owner handle,
   infrastructure-enforced timeout, or Drop release/disarm, and any later graph
   request may consume it. FIX-2 needs an owned request-scoped bounded handle,
   idempotent Drop release/disarm, and executable cancellation and unrelated-
   request isolation tests.
2. **P1 — `both` EXPLAIN still differs from production SQL.** Production runs
   one OR statement while EXPLAIN runs separate outgoing and incoming queries.
   FIX-2 must make both paths consume the identical statement and bind
   description for every direction.
3. **P1 — High-bound and state evidence is vacuous.** Measurement returns zero,
   database seeders are empty, and the test searches source for helper names.
   Existing runtime coverage uses only 3/4 rows; 10,001 covers argument
   validation rather than database work. FIX-2 needs real SQLite fixtures for
   exactly 10,000/10,001 inspected rows, measured proportional RSS, dependency
   closure, erasure, and every required projection state.
4. **P2 — Context-union precedence still diverges.** Rust reads `context.type`
   before object closure; TypeScript reads `context.context` before validating an
   invalid discriminant. FIX-2 must close first, then validate the discriminant,
   then read its payload, with escaped-unknown plus missing/invalid parity cases.
5. **P2 — Schema-step documentation is factually wrong.** Endpoint indexes
   originated in step 12 and were recreated in step 23; step 33 added Slice 45
   page indexes and visibility state/triggers. FIX-2 must state that Slice 60
   adds no migration and reuses the existing endpoint indexes.
6. **P2 — Raw cross-binding parity is unproved and diverges for Unicode.**
   Python's default `ensure_ascii=True` emits escaped non-ASCII while Rust and
   TypeScript emit UTF-8. FIX-2 must use `ensure_ascii=False`, add a non-ASCII
   raw request compared byte-for-byte across all three bindings, and capture
   native result bytes as well.

## Required FIX-2 discipline

Commit new dedicated RED tests before corrective product changes. Prior frozen
tests and fixtures remain read-only except for separately audited mechanical
corrections. Then implement GREEN, update chronology/interfaces, and obtain
independent review cycle 3. No packaging, registry work, tags, or publication.
