---
title: Slice 60 verification FIX-9 RED oracle correction 15
---

# Slice 60 verification FIX-9 RED oracle correction 15

Verification cycle 3 exposes a stale operational fuse in AC-059b's required
1,000-search cursor-race fixture. The complete route reached iteration 989 and
the one permitted isolated reproduction reached iteration 962 before the
fixed 30-second panic. Neither run reported a cursor invariant violation,
blocked search, hang, or deadlock. Per the same-failure retry rule, no third
unchanged run is permitted.

The pre-correction SHA-256 of
`src/rust/crates/fathomdb-engine/tests/cursor_read_after_write.rs` is
`8a51761ed95d5d6cee98ef959aacd37eca60fd27b7402aa3ad7c5e1f6632fa86`.

An independent authority audit found that AC-059b, REQ-055, and
`dev/design/engine.md` specify cursor correctness and immediate visibility,
not a duration threshold. `dev/test-plan.md` describes AC-059b only as the
approximately 1,000-iteration long-run race fixture. The introducing commit
`9b951e8f` labels 30 seconds as operational boundedness; gate-placement commit
`a0c13948` makes no policy change. Separate acceptance criteria own read
latency and projection-freshness timing.

FIX-9 therefore changes only the operational fuse and its truthful comment and
diagnostic from 30 to 60 seconds. It must preserve exactly:

- 1,000 search iterations;
- the concurrent writer and 50-microsecond throttle;
- row-count versus returned-cursor observation on every search;
- violation accumulation and the final zero-violation assertion; and
- AGENT_LONG-only gate placement.

Sixty seconds is a bounded anti-hang fuse, not a product SLA. Changing the
writer throttle would reduce concurrent-commit opportunities and weaken the
oracle; dynamic or per-iteration timeouts would make boundedness host-dependent
or allow an unbounded total run. If the unchanged workload exceeds the new
fuse, stop and investigate rather than raising it again.

GREEN requires one focused AGENT_LONG execution completing all 1,000 searches
with zero violations, followed by the complete `scripts/check.sh` route.
