---
title: FathomDB 0.8.26 Slice 50 — verification architecture
status: DRAFT
---

# Slice 50 design — verification architecture

## Evidence topology

One exact candidate commit produces all artifacts. Each witness runs from a
clean installation and records artifact hash, platform, runtime/toolchain,
database creation version, command/test identity, bounded output, and result.
Cross-SDK tests exchange only public persisted state and public models.

The integrated Memex-shaped flow remains FathomDB-owned contract evidence:

1. atomically write the supported canonical/derived/dependency/edge unit;
2. restart and prove deterministic replay;
3. freeze, search with explanation, and resolve exact ranked evidence;
4. traverse and resolve target/terminal-edge evidence by revision;
5. page and inspect dependency/projection state; and
6. run the version-matched external integrity check under its quiescence rule.

It does not import Memex, choose ontology or contradiction policy, or assert
answer/admission semantics.

## Manifest and decision boundary

The manifest is generated evidence, not a manually asserted green status. It
separates locally executed targets, externally executed targets, skips with
owner/reason, and failures. Slice 50 can establish a release candidate; only a
separate explicit HITL action may tag or publish it.

## Windows inventory contract

One machine-readable inventory owns the Windows structural and installed-wheel
module/symbol expectations. Source-tree checks, built-wheel inspection, and
clean-environment import probes consume that same inventory and report the
candidate commit and wheel hash. No test keeps a second handwritten list.

## Registry visibility seam

The non-publishing release-smoke harness tests a bounded classifier and polling
policy with local fixtures. Retry applies only when the exact requested version
is not yet visible; authentication, metadata mismatch, hash mismatch, install,
import, and general network failures fail immediately. Bounds, interval, final
diagnostic, and registry identity are recorded. Slice 50 does not perform a
real publish or treat fixture success as registry evidence.

## Gitleaks evidence admission

Run the exact candidate evidence generator and scan its actual output. If—and
only if—a stable benign generated value is rejected, propose or register that
exact digest through the existing versioned release input, with a mutation test
showing nearby values still fail. Do not add a path, regex, blanket allowlist,
or full-history triage. If no candidate evidence requires registration, make no
Gitleaks configuration change.
