# Slice 75 oracle corrections

Date: 2026-09-10

## Approved corrections

- Reissued the governed-surface pin for the already-approved Slice 60
  `graph.expand` API. The allowlist change from the prior pin is exactly that
  one member; the count, member list, blob, SHA-256, commit, and approval
  provenance were updated together.
- Corrected the direct-Rust and post-commit WAL-attribution fixtures to expect
  zero redundant runtime-probe connections after Slice 71B connection reuse.
  The installed-Python control still expects its two deliberate probes.
- Updated the matching Windows CI diagnostic from `probes:2` to `probes:0`.

All registration and role counts, autocommit checks, idle-snapshot checks,
actual checkpoint facts, and typed erasure outcomes remain asserted.

## Focused evidence

- Exact direct-Rust actual-checkpoint test: 1 passed; observed `probes:0` and
  `complete=1`.
- Exact post-commit WAL diagnostic test: 1 passed; observed the complete
  1/8/1/2/0 connection inventory, idle collector, checkpoint attempts, and
  typed post-close outcome.
- Governed-surface direct check: passed at 67/5/5.
- Governed-surface mutation suite: passed, including add, remove, lazy-repin,
  malformed-pin, denylist, landing, and CI-wiring guards.
- `cargo fmt --check`, shell syntax, `actionlint`, and `git diff --check`:
  passed.

The corrections are commits `25f6d73a` and `b1486cf3`. An independent
read-only review passed the combined changes and confirmed that the governed
surface adds only `graph.expand`, direct-Rust/Windows expect zero probes,
installed-Python remains at two probes, and the substantive WAL assertions are
preserved. No unaffected broad verification was restarted.
