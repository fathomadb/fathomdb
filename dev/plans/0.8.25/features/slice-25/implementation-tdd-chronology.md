---
title: 0.8.25 Slice 25 FIX-6 TDD chronology correction
status: COMPLETE
updated: 2026-09-04
red_commit: 8db99b12
green_commit: f9a65511
---

# Slice 25 FIX-6 TDD chronology correction

The TypeScript `__proto__` and Python/TypeScript parser-precedence tests in
`8db99b12` were genuine failing tests against the preceding implementation and
were preserved unchanged through GREEN.

The rollback test in the same RED commit was not a valid product RED witness.
It added the required before-to-success delta assertions, but its new
`s25src`/`s25drv` kinds were outside the engine's vector-committable vocabulary.
Consequently, its projection/vector assertion failed because the fixture could
not exercise the claimed path. During `f9a65511`, the fixture was corrected to
use the governed `doc` kind and a deterministic accepted unit vector. The
corrected test then proved that the already-implemented atomic rollback path
changes all claimed surfaces on success and restores exact pre-state at each
fault index.

This was a test-adequacy correction, not a production rollback correction. It
did edit a test during GREEN and therefore did not follow the repository's
test-read-only fix-to-spec discipline for that one witness. The deviation is
recorded rather than represented as preserved RED/GREEN evidence. Future
rollback work must validate that the control fixture reaches every intended
projection arm before committing the RED test.
