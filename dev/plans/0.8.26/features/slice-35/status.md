# Slice 35 implementation status

Status: COMPLETE ON `release/0.8.26` at reviewed and independently verified
implementation tip `88e04f26`.

## Completed scope

- Reconciled the draft against completed Slices 10–30, the assigned engine and
  binding surfaces, the allocated requirements/acceptance items, and ruled
  decisions D26-03 through D26-05. The approved plan explicitly leaves the
  production schema-34 fresh-only cutover, release integration, and smoke to
  Slices 40 and 50.
- Added `PutDerivedEdge` to the single changed-in-place V1 actuation grammar,
  digest, rollback-only simulation, committed application, counters, receipt,
  replay/integrity, erasure, and source-reference paths by reusing canonical
  edge machinery.
- Enforced both endpoints against the complete final active batch state, with
  deterministic `from`-before-`to` refusal and atomic rollback. Edge identity,
  supersession, traversal, projection, and lifecycle semantics remain
  canonical.
- Preserved the 256 affected-revision bound despite unbounded G11 fan-in.
  Capacity is detected during semantic savepoint simulation so it cannot mask
  earlier provenance or collision failures. Receipt loading rejects unknown
  affected revisions and uses the exact receipt-dependent source-reference
  ceiling.
- Added the fifth variant to Python, TypeScript, PyO3, and N-API closed shapes;
  expanded full precedence matrices; updated interface and public reference
  docs; and proved one Unicode/embedded-NUL fixture across Rust, Python, and
  TypeScript.
- Preserved the sealed 0.8.25 Slice 25 fixture byte-for-byte and placed the new
  contract in a Slice 35 fixture and separate TypeScript test, leaving the
  retained installed Windows smoke inventory valid.
- Added a read-only prototype classifier and test-only schema-34 bootstrap that
  prove fresh admission and exact no-mutation refusal of schema 33 without
  activating the production open cutover.

## TDD and review

- `c29c8db6` — reconciled and independently reviewed plan/design approval.
- `7f3e4a29` — committed RED contract for derived-edge actuation.
- `261782c2` — GREEN engine, bindings, docs, conformance, and performance
  implementation after resolving all initial design and code findings.
- `13b704ef` — verification-driven Python edge-time fixture translation fix.
- `88e04f26` — verification-driven sealed-fixture split and final reviewed tip.

The design reviewer returned final PASS after reviewing the capacity/source-ref
correction, benchmark protocol, and historical/current fixture boundary. The
code reviewer returned final PASS after all seven initial findings and both
verification findings were resolved. Detailed evidence is in `design-review.md`,
`code-review.md`, and `performance.md`.

## Verification

- Engine actuation suites: 32/32 PASS (Slice 25 contract 4, verification 15,
  Slice 35 spike 13).
- N-API and PyO3 edge parser suites: 2/2 PASS each.
- Fresh non-editable Python wheel: shared Slice 35 fixture 1/1 and Python
  actuation suite 7/7 PASS; no editable worktree install was used.
- TypeScript native/runtime suite after pinned `npm ci`: 463/463 PASS,
  including the separate Slice 35 exact-digest fixture.
- Deterministic ignored performance harness: 1/1 PASS; all sequential and
  concurrent workloads completed with zero observed writer-lock failures.
- Canonical `agent-verify`: exit 0, 109/110 registered suites PASS, zero
  exclusions, and zero security violations/blockers/downgrades. Its only
  declared environment skip was TypeScript before `node_modules` was installed;
  the subsequent full 463/463 TypeScript run closes that evidence gap.
- Historical closure integrity: sealed Slice 25 digest restored exactly;
  Slice 75 manifest verification and all 12 mutation tests PASS; retained Slice
  73 Windows N-API structural tests 5/5 PASS.

## Performance and prototype disposition

The canonical three-operation candidate remained millisecond-scale (p50 5.203
ms, p95 6.665 ms) and exact concurrent replay remained cheap (p50 1.557 ms).
The maximum 128-operation batch completed in 200.385 ms. Full response, cursor,
completion-spread, slow-event, and seeded storage-growth accounting is retained
in `performance.md`; no architectural stop condition was crossed.

Slice 40 may retain the fifth operation, canonical edge reuse, exact endpoint
and capacity semantics, receipt/integrity work, binding grammar, and regressions
as its implementation base. The pre-edge control is characterization only. The
test-only no-op schema-34 bootstrap and parameterized classifier are not a
public compatibility path: Slice 40 owns the real migration step and mandatory
fresh-only public-open enforcement. No V2 surface or historical replay path was
created.

## Final verdict

AC26-35A through AC26-35H are satisfied at the source-candidate boundary.
Slice 35 is complete; Slice 40 is next.
