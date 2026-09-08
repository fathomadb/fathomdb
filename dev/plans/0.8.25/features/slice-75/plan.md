---
title: 0.8.25 Slice 75 — integrated release closure
status: DRAFT
depends_on: 73
design: design.md
design_status: SPLIT_RECONCILIATION_REQUIRED
---

# Slice 75 draft plan

## Outcome and inputs

Produce the non-publishing release-ready decision for 0.8.25. Consume, rather
than repeat, the exact-commit receipts from Slices 71–73 for AC-013, bulk
ingest, installed CE CPU/CUDA, generic preflight, and focused Windows Node/N-
API coverage. Audit all retained Slice 10–73 contracts and run the integrated
matrix on one unchanged candidate.

## Retained integrated work

- Seal strict `IntegratedClosureManifestV1` and
  `IntegratedClosureReceiptV1` schemas that bind commit/version, design and
  prior-slice receipts, packages, fixtures, commands, platforms, workload
  cells, thresholds, N/A routes, raw outputs, and result state.
- Build registry-equivalent Rust crate/CLI, Python wheel, npm/native, and
  applicable CUDA artifacts once per selected target. Test isolated consumers
  with no editable/source-tree fallback and require SDK/wire equality.
- Run representative 10k and 50k concurrency/consistency workloads covering
  bounded reads, mutation, projection readiness, evidence, dependency,
  lifecycle, erasure/recreate, integrity, and constrained graph expansion.
- Run focused lifecycle/overhead comparisons and the locally packaged native
  GLOBAL-01 `Engine.search` witness. Keep retrieval metrics explicitly data-
  plane-only and make no answer-quality claim.
- Execute the complete local CI-equivalent matrix required for release
  closure, including fast/heavy/all, applicable feature/operator cells,
  packaged consumers, strict ptrace stress, and the already-designed platform
  routes. Do not silently waive a route.
- Open the final unmerged release PR at the exact locally verified SHA and
  require all hosted checks green at that unchanged head. Earlier Slice 73
  Windows evidence is necessary coverage but cannot substitute for this final
  exact-head gate.
- Reconcile every receipt and write the final release-ready status. Actual
  registry publication, tags, release creation, and post-publish smoke require
  separate explicit authorization.

## Deduplication boundary

Slice 75 must not rerun the Slice 71 baseline investigations, redesign the
Slice 72 preflight or CE profile, or reimplement Slice 73 Windows coverage.
It validates their receipt identity, reruns only the integrated candidate-side
cells naturally included in the final matrix, and stops on missing,
contradictory, stale, or non-passing evidence.

## Draft-to-ready sequence

Reconcile this draft after Slices 71–73 close; seal manifest, workloads,
thresholds, platform/N/A table, and receipt agreement rules; obtain independent
design review; implement harness checks RED/GREEN; obtain code review; execute
the full local matrix; obtain exact-head hosted CI; and record release-ready or
blocked without publishing.
