---
title: FathomDB 0.8.27 Slice 85 - independent design review
status: PASS
reviewed_candidate: d1918892e598ecc60e50fe22712d1ae0e71f62ee
target_release: 0.8.27
---

# Slice 85 independent design review

An independent, read-only GPT-6 Astra medium reviewer examined the exact
committed Slice 85 design at `d1918892`. The verdict is **PASS for prospective
design commissionability**, subject to the already specified AC27-85F entry
receipts and an explicit repository-owner execution ruling. No P0-P3 design
findings remain.

The checkout contained concurrent uncommitted Slice 90 documentation edits, so
the reviewer bound this verdict to the immutable committed candidate rather
than claiming a clean shared worktree. The complete Slice 85 master-plan
section in the checkout was byte-identical to `d1918892`, with SHA-256
`3c00c0c6fe569679148bd523f268378985ca9c04742c483d97b2e44861d75673`.
No production-source change was present and the reviewer made no edits.

## Findings and corrected-design verification

No new findings. The reviewer independently confirmed that the corrected
design:

- extracts whole-crate edges before scoped enforcement, follows executable
  root and outside-module paths to a fixed point, and fails conservatively on
  unresolved relevant forms;
- limits the `errors -> graph_expand::types` admission to the named payload
  relationship while retaining every executable dependency;
- gives governed wildcard-import removal explicit per-module batches;
- specifies the standalone `dev/tools/module-boundary-gate` crate, parser
  features, lockfile, dependency/license coverage, lint hook, cache invalidation
  and stale-policy behavior;
- detects local free-function shadowing and covers it with fixtures and
  production mutants;
- makes the frozen item-level policy file authoritative while keeping prose
  examples illustrative;
- fixes projected-text result ownership, the AC-037 decision identifier and the
  Slice 80 handoff supersession;
- cites the findings-resolution record from Slice 85's release-state entry; and
- requires `AGENTS.md` section 3 to name the boundary gate whenever it joins
  normal lint, independently of whether the tool becomes a workspace member.

## Acceptance assessment

| Acceptance | Independent assessment |
| --- | --- |
| AC27-85A | **PASS.** Named homes cover root carriers, attribution and transaction helpers; fields remain private and construction paths are explicit. |
| AC27-85B | **PASS.** All four inherited cycles and indirect recreations are prohibited; handler/pool and graph/read/search error direction is coherent. |
| AC27-85C | **PASS.** Complete discovery, item-level frozen policy, edge kinds and shrink-only admissions make the bounded scope falsifiable without whole-crate normalization. |
| AC27-85D | **PASS.** The syntax-bounded AST design is implementable, source-derived and conservative across the required configurations. |
| AC27-85E | **PASS.** Sole-reverse-edge fixtures, transitive/admitted/glob cases, shadowing checks and independent production mutants prevent vacuous success. |
| AC27-85F | **PASS as an entry/exit contract; operational evidence still required.** Exact-baseline canonical, native, public and hidden receipts must precede code movement. |
| AC27-85G | **PASS.** Historical security runs do not qualify HEAD; Slice 150 retains exact-final-candidate AC-037. |

AC27-85F's missing receipts are an environment and evidence prerequisite, not
a design defect. The detailed master-plan section, curated design references
and generated commission manifest are a sufficient implementation package; a
duplicate Slice 85 plan or design file would add another drift-prone authority.

## Checks and limitations

The reviewer passed the plan-status, design-status and plan-anchor checks;
release-state view verification (12 generated blocks across seven state
files); the Slice 85 commission manifest (17 paths, 20 anchors, zero dead);
`git diff --check`; and exact committed/worktree comparison of the Slice 85
section.

No runtime tests, native receipt, official surface capture, full verification,
gate implementation, or destructive mutant was run by the design reviewer.
This PASS authorizes no implementation by itself. Slice 85 becomes
commissionable only after AC27-85F evidence and the owner execution ruling are
durably recorded.

## Execution-plan review — FAIL at `ca93721c`

The required GPT-6 Astra medium execution-plan review confirmed that engine
source and assigned carriers had not drifted since `d1918892`, the accepted
D27 runtime successor remained Slice 90 work, and the reviewed requirements,
acceptance criteria, ownership design, boundary gate, TDD stages, and review
plan remained feasible and appropriately scoped.

It found one P3 reconciliation defect: the manual Slice 85 board row still
said planned/uncommissioned and instructed the already-completed capable-host
entry qualification. The release-state JSON and execution plan correctly said
`IN_PROGRESS`, recorded the commission, and bound the passing entry receipts.
The board row is corrected in the same fix. No design or production change is
required. A subsequent GPT-5.6 Sol high review must verify this correction
before implementation begins.
