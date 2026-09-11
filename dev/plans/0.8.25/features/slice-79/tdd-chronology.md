---
title: Slice 79 TDD chronology
status: COMPLETE
---

# Slice 79 TDD chronology

1. RED commit `069f93a3` added the Rust runtime-state and statement-reuse
   contracts plus Python and TypeScript runtime-control tests. They failed on
   missing types, functions, error mapping, and cache behavior.
2. GREEN product commit `a6650c81` implemented the startup state machine,
   default performance mode, diagnostics mode, typed cross-binding APIs, and
   bounded per-reader statement reuse. The focused semantic and installed-
   artifact tests passed.
3. One RED setup assertion expected `sqlite3_hard_heap_limit64` to return the
   new limit. SQLite returns the previous limit (`0`, meaning unlimited). The
   evidence-based oracle correction changed only that return-value assertion;
   accounting and over-limit allocation checks remained intact and passed.
4. Code review found the approved runtime-control member was not mechanically
   pinned. A new lazy-repin recurrence arm failed because the checker ignored
   `runtime_controls`; GREEN made it a required member list/count, reissued the
   exact `seq-276` pin, and updated the shared synthetic fixture. The full
   focused pin suite passes. These follow-up commits change no measured product
   file.
5. The existing populated schema-26 witness then failed 0/2 because its setup
   opened raw SQLite before FathomDB configuration. Test-only commit `e2db3ffc`
   selects the ordinary performance mode before constructing the prior-release
   fixture; every existing upgrade, reopen, projection, lifecycle, dependency,
   and erasure assertion remains unchanged and passes 2/2.
6. The unchanged registered AC-020 assertion remained RED in 14/14 measured
   processes. The Slice 71B write guards passed. AC-072 met its numerical
   limits in all three runs, but nonzero swap activity made all three
   environment-invalid under the retained protocol. No tuning or broad
   verification followed the failed release gate.
