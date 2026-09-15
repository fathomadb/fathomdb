# Slice 30 implementation status

Status: COMPLETE ON `release/0.8.26` at reviewed implementation tip
`067d74b4`.

## Completed scope

- Reconciled the draft against Slice 20, shipped 0.8.25 DPI behavior, assigned
  requirements/acceptance, interface docs, and D26-02. The approved plan keeps
  the existing four checks and V1 report and rejects new packaging, bindings,
  checks, migrations, repair authority, or report versions.
- Added operator-gated
  `inspect_data_plane_integrity(path, DataPlaneIntegrityRequestV1)`, a free
  function that validates before filesystem access, acquires the existing
  product lock without rewriting it, refuses non-empty WAL/rollback journals,
  configures SQLite in canonical order, and opens an exact-schema immutable
  read-only/query-only connection.
- Routed only `doctor data-plane-integrity` through the static boundary and
  unified all post-parse failures under the privacy-safe DPI V1 envelope.
- Added the exact-version, identity, quiescence, invocation, interpretation,
  and evidence-scope procedure across the CLI/Rust interfaces, install/reference
  docs, CLI crate README, and the new operations guide.
- Strengthened the future crates.io smoke with exact binary identity and a
  complete product-file no-mutation witness.

## TDD and review

- `3834f22a` — initial immutable-boundary RED.
- `0fa51f05` — GREEN operator inspection boundary; plain read-only sidecar
  failures drove the `immutable=1` correction.
- `4cbb8218` — reviewed design propagation, reserved-path oracle, operator
  procedure, and post-publication smoke.
- `0794bcf0` — operator-off warnings-denied feature-scoping correction.
- `e267f652` / `067d74b4` — code-review signed-schema RED/GREEN.

Independent design review, code review, and verification all returned PASS
with no remaining P0-P2 findings. Detailed chronology and evidence are in
`tdd-chronology.md`, `design-review.md`, and `review-verification.md`.

## Verification

- Slice 30 CLI process suite: 11/11 PASS.
- Legacy Slice 55 CLI suite: 3/3 PASS.
- Existing Slice 55 engine integrity suite: 66/66 PASS.
- Operator-off and operator-on governed-surface checks and doctests: PASS.
- CLI all-target Clippy with warnings denied: PASS.
- Markdown/public-doc lint, shell syntax, and shellcheck: PASS.
- Clean-root source install/discovery on Linux x86_64: PASS; binary identity is
  the expected pre-integration `fathomdb 0.8.25`.
- Canonical `agent-verify` reached only the known durable-worktree environment
  limitation: Python typecheck cannot resolve dependencies because this
  worktree has no local `.venv`. No prohibited editable installation was used;
  Slice 30 changes no binding source. Slice 50 retains the supported-environment
  integrated gate.

## Final verdict

AC26-30A-E are satisfied at the source-candidate boundary. The command is
bounded, exact-schema, private, out-of-process, and product-file immutable on
the qualified host. Final version integration, wider target evidence, and the
post-publication registry witness remain correctly assigned to Slice 50 and the
release smoke. Slice 35 is next.
