---
title: FathomDB 0.8.26 Slice 65 — independent design review
status: REMEDIATED_PENDING_FINAL_REREVIEW
reviewed_tip: 35328d2b
---

# Slice 65 independent design review

## Initial verdict

The independent read-only review returned **BLOCK** with two P1, one P2, and
one P3 finding. It found no overbuild in the companion catalog, bounded
eight-row error-owner correction, or decision to rerun affected platform
receipts.

1. **P1 — external authority was existence-only.** Proposed or superseded ADRs,
   draft interfaces, and role-inappropriate use of `AGENTS.md` could terminate
   the graph. The design now requires fail-closed front-matter status parsing,
   exact accepted/locked classes, role-bounded repository invariants, and RED
   fixtures for each invalid terminal class.
2. **P1 — candidate-manifest change lacked a compatible design and TDD.** The
   existing assembler is intentionally Slice-50-schema-specific. The corrected
   design leaves it and its committed manifest unchanged, adds a thin Slice 65
   wrapper that delegates base validation, and adds RED cases for old-schema
   preservation plus missing/extra/cross-schema/non-pass/candidate-drift
   rejection.
3. **P2 — verification categories were not reproducible.** The plan now pins
   lifecycle/parity/owner, recovery, manifest, release-state, Markdown,
   security, canonical-gate, package-smoke, CLI, exact-SHA CI, receipt, and
   final-manifest commands. Exact candidate coordinates are frozen in the
   completion execution record before qualification.
4. **P3 — delta count was off by one.** `65f69ca1..4077148b` contains 17
   commits; plan and inventory now say 17 while retaining the verified 53-path,
   3,629-insertion, and 861-deletion counts.

Implementation remains paused until independent rereview confirms that no
P1/P2 design finding remains.

## First rereview

The reviewer marked both P1 findings and the P3 count **resolved**, but retained
one P2 because generated-scan roots, package variables, CLI commands, CI
dispatch/downloads, and manifest coordinates were still prose or ellipses. The
plan now includes one executable candidate template with concrete variable
definitions, local build/install/smoke commands, tracked and generated Gitleaks
commands, exact-SHA branch push/workflow dispatch/watch/artifact downloads, and
full Slice 65 wrapper assembly/validation arguments. The completion record may
substitute only observed tool versions and candidate-owned paths; it cannot
change command semantics.

Implementation remains paused pending a second rereview.

## Second rereview

The reviewer confirmed the prior reproducibility P2 resolved, but found one P1:
the npm tarball recorded in the candidate manifest was not the tarball exercised
by the local native-artifact smoke helper. The candidate template now stages the
main and matched platform packages, applies the publish-time optional-dependency
injection, packs both exact artifacts into the evidence directory, installs
those exact tarballs into an isolated consumer, and opens/closes an engine from
that installation. The manifest binds the same `npm_tarball` path; the later
artifact lookup cannot replace it with the platform tarball.

Implementation remains paused pending final rereview of this binding.
