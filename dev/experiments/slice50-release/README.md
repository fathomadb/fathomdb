# Explicit module model release controls

These real installed SDK controls complement the
[Rust SDK fixture](../../../src/rust/crates/fathomdb-sdk/tests/slice50_module_models.rs).
They require an already authorized guarded venue, forced CUDA policies, frozen
model and installed artifact identities, and fresh task-owned databases. They
never authorize a venue, reset, publication or host change.

- `healthy.py` and `healthy.mjs` exercise repeated empty release, CLS and CE
  population, engine ownership, exact healthy reload and normal close/exit.
  On Orin, passive observations assert3→1→3→0 owners and final USED/RESERVED0
  within5s. On x86 the actual observer status is `not_built`, with unavailable
  private counters.
- `busy.py` and `busy.mjs` positively observe typed overload during actual CLS
  inference, preserve its exact output and require subsequent release/reload.
- `orin-loss.py` and `orin-loss.mjs` use the existing completed-construction process-local reset
  actor, immediately calls the new API, requires persistent typed loss and
  performs fresh-process recovery against the committed database. This case
  requires standing Orin release-verification authority and its resource guard.

- `proc_exit_transition.c` reproduces Linux main-thread exit/procfs disappearance
  before the child becomes waitable. Its bounded CPU controls run through
  `python3 scripts/tests/test_slice130_soak.py`; they perform no GPU work.

Python runs with its owned installed environment's `bin/python -I`; retain that
path without resolving its virtual-environment symlink. Node reads
`FATHOMDB_PACKAGE_DIR` and `FATHOMDB_NATIVE_PACKAGE`. Both healthy controls use
`FATHOMDB_DB`. The loss control uses `FATHOMDB_CAPTURE_DIR`,
`FATHOMDB_COORDINATED_RESET=reset-refuse` or `recover`, the exact selected UUID,
and the checked existing reset actor path/hash. No forced GC or heap flags are
used.

Actual source/native identities, guarded commands, original failed harness
attempts and scoped recoveries are indexed in
[Slice50 status](../../plans/0.8.30/features/slice-50/status.md). The adapted guard
and raw results remain in the task-owned evidence named there; the underlying
D45 guard is maintained in the owning Candle fork at the recorded commit.
