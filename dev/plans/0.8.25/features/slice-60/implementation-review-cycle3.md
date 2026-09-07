---
title: 0.8.25 Slice 60 implementation review — cycle 3
status: FAIL
review_cycle: 3
candidate: d731e03c1ab3b872f572cabee98915bde988e9e5
---

# Slice 60 implementation review — cycle 3

## Verdict

**FAIL.** No P0 or P3 finding exists. One P1 and two P2 findings remain. Owned
request-scoped rendezvous, exact production/EXPLAIN `both` SQL, context-union
precedence, Unicode wire bytes, and schema history are closed. Review was
read-only and the exact candidate remained clean.

## Findings

1. **P1 — State and RSS evidence remains hollow.** RSS subtracts a baseline
   initialized to zero and never assigned, so it reports whole-process historic
   peak rather than expansion delta. Dependency, erasure, and projection seeders
   remain empty; source-name assertions and pure degradation-helper calls do not
   exercise real graph expansion. FIX-3 needs real SQLite executions for
   registered/unregistered/closure-fenced dependencies, erase/excise
   disappearance, and every required projection origin/readiness/degradation
   combination. RSS must measure an immediate baseline or isolated process
   delta/control and prove a ceiling against work/database size.
2. **P2 — Rust RFC 6901 escaping fails tilde-only names.** It escapes `~` only
   when the field also contains `/`, returning `/context/a~b` rather than
   `/context/a~0b`. FIX-3 must always replace `~` before `/` and add top-level
   and nested tilde-only cross-language wire cases.
3. **P2 — Python invalid context carriers leak `AttributeError`.** Recursive
   validation dereferences `request.context.context` before checking its carrier
   type. FIX-3 must validate the carrier first and prove a binding-local
   `TypeError` with no native invocation.

## Required FIX-3 discipline

Commit additive executable RED tests before product fixes. Existing frozen
oracles remain read-only. Then implement GREEN and obtain independent review
cycle 4. Packaging, registry work, tags, and publication remain prohibited.
