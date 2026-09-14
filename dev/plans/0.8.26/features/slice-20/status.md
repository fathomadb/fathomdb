# Slice 20 implementation status

Status: COMPLETE ON `release/0.8.26` at reviewed implementation tip
`c0a567d5867c89b4f26caaaaa188373222d6c2f6`.

## Landed on the implementation branch

- `884d34bd` — corrected-design RED contract tests.
- `e237720a` — fixed graph reference, request/disclosure commitments,
  two-phase bounded hydration, resolver coherence, and relaxed-window refusal.
- `9c92a005` — ten-block/domain/normalization tests and global authority ordering.
- `1c88c2c8` — source eligibility revalidation, ADR/interfaces/guide alignment,
  and durable TDD chronology.
- `55a95163` — deterministic parallel-edge winning revision regression.
- `6af2502e` — warnings-denied reader-dispatch boundary annotation.
- `588a1b67` / `f7f7853a` — temporal-boundary RED/GREEN.
- `d8ae9a50` — independently reviewed additive frozen-drift oracle correction.
- `b5b47d33` / `d14fff43` — no-SQL nonce RED/GREEN using OS CSPRNG bytes.
- `5242b0ab` / `fbce32ae` — complete deterministic phase-two fault
  classification and exact paths.
- `7be5aea4` / `bf4f3525` — real reader-connection statement-count RED/GREEN.
- `ca66f23a` / `6986d44f` — recursively closed Rust evidence sidecar/entry
  decoder RED/GREEN.
- `dfaf33a0` / `48b9a5c3` — recursively closed and coherent Python/TypeScript
  resolved-response RED/GREEN.
- `0ad5df62`, `cf5c9895`, `36e80852` — missing precedence, metadata,
  multi-source, erase/excise linearization, and real-NAPI oracles.
- `ecf60335` — fresh installed-wheel graph-evidence profile.
- `6666b906` / `6f7b946d` — canonical-source authority-before-detail and
  globally ordered exact missing-link paths RED/GREEN.
- `a5389d3c` / `e47bdce0` — engine-scoped resolver rendezvous and both
  resolver/erase/excise transaction-order outcomes RED/GREEN.
- `3fc9ed6e` / `6f3662b0` — engine-scoped erasure rendezvous RED/GREEN and
  cross-engine non-consumption proof.
- `5e84f837` — real release injection/pack/offline-install N-API graph-evidence
  proof, including declarations, exports, loader, target, and terminal edge.
- `28fb8ef0` / `abc77847` — canonical non-`test-hooks` Rust lint RED/GREEN;
  trace-only nonce instrumentation is correctly feature-scoped.
- `919180d5` / `6ec740e4` — Python artifact-revision grammar and cross-binding
  resolved-dependency identity-coherence RED/GREEN.
- `2b7ee5d3` — registered-dependency, restart, nondisclosure, lifecycle/storage,
  token-integrity, and ranked/graph domain-separation acceptance coverage.

The branch also contains merge commits for the approved corrected design at
`bc8123a8`, frozen-drift reconciliation at `fa39e5e9`, and Astra follow-up PASS
record at `4747d4f2`.

## Focused verification

- Engine library check with test hooks: PASS.
- Engine library Clippy with `-D warnings`: PASS.
- Private selector tests: 2/2 PASS.
- Private semantic request-normalization tests: 2/2 PASS.
- Deterministic parallel-edge evidence test: PASS.
- Full focused graph-evidence integration suite with `test-hooks,operator`:
  25/25 PASS.
- Runtime SQL differential: empty 0, one target 2, multiple targets and distinct
  sources 2 PASS.
- TypeScript typecheck and focused suite: 3/3 PASS, including real-engine target
  and terminal-edge resolution plus malformed-response refusal.
- Python Ruff and Pyright: PASS. The focused Python suite ran against the fresh
  installed wheel with repository source injection disabled: 9/9 PASS.
- Fresh wheel build/install/profile: PASS; wheel SHA-256
  `df36b7886157527ecd30d3cf6d49270dcb90431cc9ea2a9a34302874c59991c5`.
- Engine library check with `test-hooks,operator`: PASS.
- Engine library Clippy with `test-hooks,operator` and `-D warnings`: PASS.
- Selector property/framing/domain suite: 3/3 PASS; request normalization: 2/2
  PASS; no-SQL nonce: 1/1 PASS.
- Repository release-path npm proof: PASS. The thin main and matched
  `fathomdb-linux-x64-gnu` platform package were injected, packed, installed
  offline in a clean consumer, loaded without source fallback, and exercised
  exact target plus terminal-edge resolution.
- Strict documentation lint/build: PASS.
- Corrected-tip graph evidence: 28/28 PASS; installed-wheel Python: 13/13 PASS.
- Corrected-tip wheel SHA-256:
  `2ae529add9a59a54456256ab4ef044d41ebf75dcc32cded172a810f7559b3865`.
- Canonical `agent-verify`: lint, typecheck, security, Rust workspace, and 109 of
  110 registered test suites passed. Python collection alone failed because
  the linked worktree `.venv` points at the primary checkout and has no native
  module for this checkout. The environment was not rebound through the
  forbidden editable/maturin worktree path; fresh installed-wheel evidence
  covers the changed Python surface.

The source-tree Python collection still intentionally cannot load a native
extension from this worktree; no editable install or `maturin develop` was used.
The fresh wheel and import-isolated pytest route replaces that invalid evidence.
The prior fresh-local-npm deferral is closed by FIX-2 using the repository's
actual platform-package injection and pack route. Hosted registry and
cross-platform publication remain release-ladder concerns, not Slice 20 gates.

The full workspace all-target check is presently blocked outside Slice 20 by
`slice75_schema26_upgrade.rs` referring to the later-slice
`rebuild_projections` API. Full release verification is intentionally deferred
until that ladder dependency lands.

## Resolved reconciliation gate

Post-mint mutations remain explicit `FrozenRead(StateDrifted, /token)` security
regressions. Separate pre-freeze provenance fixtures prove the global
authorization-before-detail behavior. No production code weakened or deferred
snapshot validation.

## Final verdict

Independent code review and independent verification both returned `PASS` at
the exact clean implementation tip with no remaining P0, P1, or P2 finding.
The durable verdict and the bounded canonical-gate environment exception are in
`review-verification.md`. Slice 20 is complete; Slice 30 is next.
