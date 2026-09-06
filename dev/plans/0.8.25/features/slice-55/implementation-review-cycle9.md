---
title: 0.8.25 Slice 55 independent implementation review — cycle 9
status: FAIL
---

# Slice 55 independent implementation review — cycle 9

## Review target

- Candidate HEAD: `4187ab2366d9a6168bc0d0fe44b72b00b2ee2c14`
- Product commit: `87f6c707c761e0693c739241e1b97fd173d15df8`
- Branch: `release/0.8.25`
- Worktree: clean; 135 commits ahead of the remote branch
- `git diff --check`: PASS

The independent reviewer made no repository or Git changes.

## Verdict

**FAIL.** One P1 and one material P3 remain. The two additional owner-authorized
FIX cycles are exhausted by FIX-7 and FIX-8; no further correction may begin
without renewed owner authority.

## P1 — strict trace codecs accept an impossible generation relationship

The live trace rejects a registered dependency generation that is zero or
ahead of the captured dependency-generation singleton. The strict Rust,
Python, and TypeScript codecs do not enforce the same relationship between
each edge's `registeredGeneration` and the response's
`dependencyGenerationBoundary`:

- Rust parses edges before the boundary and does not cross-validate them in
  `src/rust/crates/fathomdb-engine/src/dependency_trace.rs`.
- Python has the same omission in `src/python/fathomdb/engine.py`.
- TypeScript has the same omission in `src/ts/src/index.ts`.

The canonical Rust wire fixture itself uses edge generation 1 with boundary
generation 0, and its property test therefore blesses an impossible response.

Required correction:

1. Correct the canonical fixture to a valid boundary in an isolated,
   design-authorized oracle commit.
2. Add genuine RED cases in Rust, Python, and TypeScript for registered
   generation zero and above-boundary.
3. Make strict encoding and decoding reject those states at the exact edge
   generation path in every language.

## Material P3 — the plan's exact TypeScript command is invalid

The plan specifies:

```text
npm run build:debug --workspace fathomdb
```

The root package declares no npm workspaces, and the chronology records the
resulting `No workspaces found` failure. The plan must use the established
`src/ts` package-directory command and align the subsequent test path.

## Resolved from cycle 8

- Projection-generation accounting charges only the singleton, present
  current-generation record, and physical members.
- Stored derived and canonical owner schema versions are dynamically guarded
  and classified correctly.
- Derived self-reference uses the required precedence,
  `dependency_derived_role_invalid`, Error severity, and minimum IDs.
- Dependency generation is restricted to SQLite's signed domain; above-range
  values map to `dependency_generation_mismatch` at Critical severity.
- Signed-rowid scans, dynamic type guards, retention and owner classification,
  aggregate bounds, ordering, privacy, AC4's ten-code matrix, the real-database
  property test, fixture consumption, and legacy trace/explanation behavior
  otherwise passed review.
- The recovery and bounded deadlock-diagnosis records are internally
  consistent; no current hung process was observed.

## Reviewer verification

- `slice55_data_plane_integrity`: 66 passed.
- `check_integrity`, `slice55_dependency_trace`, `slice55_explanation`,
  `slice55_wire`, and `trace_source_ref`: 51 passed, 1 ignored.
- Broad and independent verification remain pending.
