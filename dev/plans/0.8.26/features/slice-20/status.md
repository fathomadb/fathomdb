# Slice 20 implementation status

Status: implementation substantially complete; independent review not ready
until the reconciled frozen-drift fixtures land.

## Landed on the implementation branch

- `884d34bd` — corrected-design RED contract tests.
- `e237720a` — fixed graph reference, request/disclosure commitments,
  two-phase bounded hydration, resolver coherence, and relaxed-window refusal.
- `9c92a005` — ten-block/domain/normalization tests and global authority ordering.
- `1c88c2c8` — source eligibility revalidation, ADR/interfaces/guide alignment,
  and durable TDD chronology.
- `55a95163` — deterministic parallel-edge winning revision regression.
- `6af2502e` — warnings-denied reader-dispatch boundary annotation.

The branch also contains merge commits for the approved corrected design at
`bc8123a8`, frozen-drift reconciliation at `fa39e5e9`, and Astra follow-up PASS
record at `4747d4f2`.

## Focused verification

- Engine library check with test hooks: PASS.
- Engine library Clippy with `-D warnings`: PASS.
- Private selector tests: 2/2 PASS.
- Private semantic request-normalization tests: 2/2 PASS.
- Deterministic parallel-edge evidence test: PASS.
- TypeScript build/native build and test suite: PASS, including the Slice 20
  positional sidecar contract.
- Strict documentation lint/build: PASS.

Python source-tree collection could not load the native extension from this
worktree. Repository policy forbids editable installation from a worktree, so a
fresh wheel/install probe remains for the final package gate.

The full workspace all-target check is presently blocked outside Slice 20 by
`slice75_schema26_upgrade.rs` referring to the later-slice
`rebuild_projections` API. Full release verification is intentionally deferred
until that ladder dependency lands.

## Open reconciliation gate

Two tests in `884d34bd` mutate SQLite after frozen-context mint but still expect
hydration-level evidence errors. The approved correction requires preserving
those mutations as explicit `FrozenRead(StateDrifted, /token)` regressions and
adding separate pre-freeze provenance fault fixtures. The implementation agent's
execution security reviewer blocked that additive test-oracle edit. No production
code weakened or deferred snapshot validation. The release branch must land the
test-only reconciliation, after which this branch can merge it, rerun the focused
suite, and proceed to independent code review.
