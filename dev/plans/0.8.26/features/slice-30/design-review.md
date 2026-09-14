---
title: FathomDB 0.8.26 Slice 30 — independent design review
status: PASS
reviewed_on: 2026-09-14
---

# Slice 30 independent design review

The read-only design reviewer returned CHANGES REQUESTED with no P0 findings.
All P1/P2 findings were incorporated before implementation:

- normal persistent `-shm` is allowed under the accepted reader-pool locking
  ADR; non-empty WAL and rollback journals refuse without recovery;
- canonical process-lifetime SQLite runtime configuration precedes extension
  registration and every SQLite call;
- the V1 JSON guarantee applies to post-parse semantic/runtime failures, while
  Clap syntax and type errors retain exit 2 and stderr;
- unavailable path/lock I/O and runtime configuration have distinct closed
  reasons and the full error precedence is explicit;
- source-candidate install, Slice 50 final-candidate evidence, and the
  post-publication crates.io smoke are separate witnesses;
- the Rust boundary is one named operator-gated free function rather than an
  `Engine` method;
- published 0.8.25 is correctly described as schema-33-compatible but lacking
  the immutable process boundary; and
- the CLI crate README and Rust install guide were added to documentation
  scope.

The corrected design was returned to the same reviewer, which returned PASS
with no remaining P0-P2 findings before RED implementation began.

## Post-implementation clarification

The first GREEN run proved that `SQLITE_OPEN_READ_ONLY` alone can create or
alter SQLite shared-memory state. The design was narrowed to require a
percent-encoded `file:` URI with `immutable=1`, `READ_ONLY|URI`, and
`query_only`. The same reviewer requested three propagation fixes: name this
boundary in R26-30B and the GREEN step, document that raw external SQLite
writers must be stopped, and acceptance-test URI-reserved filename bytes.

Those corrections landed in the plan, public API contract, and an 11th process
test using `reserved space#%3F.sqlite`. The focused suite passed 11/11. The
reviewer then returned final PASS with no remaining P0-P2 findings.
