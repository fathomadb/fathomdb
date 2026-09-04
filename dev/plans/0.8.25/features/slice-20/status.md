---
title: 0.8.25 Slice 20 status
status: COMPLETE_ON_RELEASE_BRANCH
completed: 2026-09-04
implementation_review: PASS_CYCLE_3
verification: PASS
memory_checkpoint: COMPLETE
---

# Slice 20 status

## Outcome

Slice 20 is complete on `release/0.8.25`. Schema step 28 adds immutable,
caller-named canonical-source-to-derived dependency registration over Slice
15's authoritative source links. Reciprocal lookup is deterministic and
bounded, and dependency membership uses a separate monotonic generation that
does not consume or wedge canonical write or projection cursors.

The additive Rust, Python, and TypeScript contracts validate the complete
relevant provenance chain, fail closed on malformed persisted identities, and
preserve dependency identity across restart and rebuild. Purge and source
erasure remove affected registrations atomically and advance dependency
generation only when membership changes.

## TDD and review

- Initial RED `6ed8aa9a`; GREEN `a2abcda3`.
- FIX-1 RED `6ff62489`; GREEN `a37ba549`.
- FIX-2 RED `ed54fff9`; GREEN `7766d4c4`.
- Independent design review passed at cycle 5.
- Independent implementation review passed at cycle 3 with no unresolved P1,
  P2, or P3 finding.

The initial GREEN added some Python, TypeScript, and lifecycle tests that were
not present in the initial RED. FIX-1 then used one grammar-valid source-ID
oracle and edited it during GREEN. Published history is preserved: FIX-2
records both process defects explicitly, labels the corrected source-ID test
post-hoc, and supplies genuine unchanged RED-to-GREEN Rust, Python, and
TypeScript evidence for the remaining source-revision defect. The reviewer
found the disclosed process exception bounded and required no owner waiver.

## Verification

- Fast aggregate: 103/103 suites passed with zero skipped or excluded.
- Focused schema, dependency, lifecycle, integrity, property, prospective
  transaction, Slice 15, and erasure tests: 55/55 passed.
- Exact freshly built Python wheel: 7/7 passed.
- Exact N-API build and all three TypeScript Slice 20 fixture files passed.
- Locally packaged wheel and matched N-API smoke passed.
- Commission manifest: 18 paths and 20 anchors, zero dead.
- Security/ptrace: zero violations and zero blockers.
- Release-state views, diff checks, and checkout cleanliness passed.

Windows runtime was unavailable on the Linux verifier. Existing Windows build
matrices and PowerShell packaged-smoke wiring include the new surfaces and were
verified structurally. CUDA, live-model, and registry-publication routes are
N/A for this slice. A sandbox-denied N-API shell and an initially incorrect
test-tree TypeScript package shape were verifier setup issues; the unchanged
unconfined native build and correct release-tree package both passed.

## Between-slice memory checkpoint

Reusable lessons were compacted into
`0.8.25-slice20-dependency-registration-lessons.md` in the external FathomDB
memory store and added to its `MEMORY.md` index before Slice 25 was
commissioned.
