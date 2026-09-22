---
title: FathomDB 0.8.27 Slice 30 - surface comparator plan
status: COMPLETE
target_release: 0.8.27
baseline_entry_sha: a7e1f1bb5cd5c8c6c17435e122cd611440b9982d
---

# Slice 30 - current inventory and comparison guardrails

## Reconciliation and disposition

The release plan was authored at `a3e6cff6`. Since then:

- Slice 10 landed at `3097d191` and closed at `ee1b0fbe`. It supplied the
  exact comparator prerequisites: `cargo-public-api 0.52.0`,
  `nightly-2026-04-24`, checkout-owned `.cache/0.8.27-slice30`, owned scratch
  at `/tmp/fathomdb-0.8.27-slice30`, and a 100 GB heavy-route disk floor.
- Slice 20 landed at `3943cb64`, closed at `5fab7da5`, and bound verification
  at `a7e1f1bb`. Its product change was confined to corrected erasure ordering
  in the engine plus tests and contract clarification. It changed no public
  shape or schema.
- The five planned refactor targets now contain 34,189 Rust engine, 5,943
  PyO3, 5,406 NAPI, 4,439 TypeScript, and 2,846 Python lines. The five-line
  increase since prework is advisory and does not change scope.
- Existing governed-surface and SDK-parity checks protect 44 canonical
  operations and selected release surfaces. They do not capture the complete
  Rust signature/re-export, Python native registration/stub, generated NAPI
  declaration, TypeScript declaration, or package-export surfaces.
- Prework P27-08 and P27-17, R27-04/AC27-07, and the Slice 10 prerequisite
  allocation remain applicable. Reserved Slice 8 allocated nothing.

The draft is **approved with narrowing**. Slice 30 owns a reusable comparator,
an immutable reviewed baseline, and a human navigation inventory. It does not
move product code, change an ADR/interface/schema, qualify release packages,
redesign CI, merge experiment branches, or turn file size into a gate.

## Need, requirements, and acceptance

This slice specializes existing N27-02 and R27-04 without changing the locked
`dev/acceptance.md` register.

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| R27-30A | Capture is deterministic and derived from real compiler, registration, declaration, and package metadata, bound to the source SHA, exact tools, and exact feature rows. | AC27-30A: two clean captures from one source normalize byte-identically; capture rejects a dirty tree or a source SHA different from `HEAD`; comparison validates both provenance SHAs separately while permitting a later clean candidate with equal tools, row identities, and surfaces. Wrong tool or feature-row identity is rejected. |
| R27-30B | The baseline covers Rust facade and engine default/operator/test-hook rows and root re-exports; Python package exports, resolved public wrapper signatures, cfg-aware native registrations, and stubs; generated NAPI declarations (build identity checked against `build:native`); re-export-resolved TypeScript compiler declarations without comments; and supported package roots/subpaths. | AC27-30B: the release candidate compares equal to the reviewed baseline across every required row without copied expected-symbol lists. |
| R27-30C | The comparator is non-vacuous. | AC27-30C: independent fixture mutations reject added and removed Rust symbols, a changed Rust re-export, a missing Python class/function/alias registration, changed Python stub membership or class inheritance, a changed NAPI declaration, a changed TypeScript export/declaration including one reached through a re-export, a changed Python wrapper name or signature, a changed `#[cfg]` gate on a registration (statement, block, or enclosing item), a drifted `build:native` script, and a changed package export/path. Comment-only edits, body-only Python edits, and moves behind re-exports compare equal. |
| R27-30D | Maintainers have a source-bound navigation record for later refactors. | AC27-30D: the reviewed inventory records paths/line counts, top-level symbols, direct dependencies and co-change neighbors, focused owning suites, boundary maps, and exceptions; file-size findings are explicitly advisory. |

## Implementation plan

1. **RED:** add focused tests for the absent capture/compare entry point and
   every AC27-30C mutation arm. Keep registration and stub mutations distinct.
   Commit the failing tests before implementation.
2. **GREEN:** implement one deterministic normalized-JSON capture/comparison
   tool. Add adapters for `cargo public-api`, Python source/native registration
   and stub inspection, generated NAPI declarations, TypeScript compiler
   declarations, and package entrypoints. Capture and review the real baseline
   using the Slice 10 versions and owned roots.
3. **REFACTOR:** deduplicate only normalization, command execution, and diff
   reporting while all mutation tests remain green.
4. Run the focused comparator suite, existing governed/parity/re-export and
   release-surface suites, applicable Rust default/operator/test-hook checks,
   Python and TypeScript checks, then `scripts/agent-verify.sh` because the new
   repository tool crosses language boundaries.
5. Obtain independent code review and independent verification. Fix findings,
   record RED/GREEN chronology and exact receipts, update release state through
   its JSON authority, generate its views, and close Slice 30 with Slice 40 next.

Capture uses the clean pre-baseline implementation commit: tests and the tool
exist there, but the tracked baseline does not. Capture must verify clean-tree
status and `HEAD`, and the manifest records that inspected source SHA. The
later commit that first tracks the reviewed baseline is recorded separately in
the status record. The entry SHA in frontmatter is historical reconciliation,
not the baseline source. Generated caches and scratch output are removed at
close; the durable release worktree remains.
