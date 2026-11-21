---
title: Inspector 0.2.0 Slice 7 — delivery evidence design
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Minimal evidence review of build-to-installed-product delivery failures
---

# Slice 7 delivery evidence design

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Boundary and existing architecture

At inspected clean `609dd85c41fb7980e05faa1132a248d376d8ec89`, Inspector is
an unpublished0.1.0 Rust workspace member with embedded browser assets, a
loopback supervisor and cleared-environment worker consuming unchanged format37
Engine. The [HLD](../../../design/high-level-design.md),
[scope](../scope.md) and [plan](plan.md) govern this review; Engine APIs,
ranker, schema and public pins are out of scope. Proposed remedies remain drafts
until9. No runtime architecture or delivery mechanism is implemented by7.

The existing delivery paths differ:

- Source: locked/offline Inspector build and paired examples; fixture-control is
  an explicit test-only feature. Browser tests and native/scale qualifications
  require separate prerequisites and do not follow from compilation.
- Repo verifier: checkout-owned Python/pip/tool preflight, lint, typecheck,
  security and tiered tests. Missing `.venv` stopped the port's full gate;
  scoped port success cannot prove full workspace or installed success.
- CI: non-Markdown Inspector changes run fast verification, but the inspected
  rust filter omits `src/inspector/**`; heavy job selection depends on language
  or harness classification. This is a source-derived gap candidate, not an
  observed hosted failure. Inspector browser/package coverage needs separate tracing.
- Archive: `src/inspector/tools/package.sh` stages a default Linux x86_64 binary,
  embedded assets, build description, license, README and checksums. The documented
  GLIBC2.39 development binary does not satisfy the22.04-compatible target.
- Engine release: multi-registry publication and exact-version waits already
  exist; Inspector has `publish=false`. Their auth/propagation failures are
  external ownership unless an explicit Inspector delivery dependency is proved.

## Method and genuine challenges

Use one compact findings table in the plan. Each row captures a concrete symptom,
source/run commit, first actionable cause, observed delay or unknown, current
versus historical state, existing remedy, remaining minimum proposal, owner and
validation approach. Never copy secret values or unsafe full scanner output.
Prefer existing logs and committed tools; no new scanner, telemetry service,
evidence register, registry probe or experimental gate is needed for this review.

A correct preflight can report an environment blocker rather than a product bug.
The generic worktree preflight selects Engine release-state; Inspector ancestry
must retain the final planning HEAD and its separate ladder. Resolve this seam
with Slice0, without rewriting Engine state or borrowing another environment.

Routing must scale with changed paths while preserving required positive release
qualification. A later approved fix should use existing CI fixture tests for
Inspector-only source/assets/package, Markdown-only, Engine-only and manual
candidate cases; do not add required-status aggregators, merge queues or soak periods.
A green fast job cannot silently become proof of heavy/native/installed coverage.

The package target triple does not establish the glibc floor. A proposed remedy
must identify a compatible build host/container and extracted-artifact check,
then preserve exact build pairing and default feature controls. Slice70 owns
actual installed launch/views/offline/close/resource evidence; feature20–60 owns
its positive behavior. Generic Engine Tier1 platform ADR obligations remain with
Engine/bindings and do not expand Inspector's one-pair qualification target.

Current-tree and historical Gitleaks findings have different meaning; both are
already report-only/advisory in inspected workflows. Historical100/121 mismatch
is documented evidence, not today's scan count. Propose precise security-owner
classification, never blanket allowlisting, secret exposure or a claim that this
advisory job is on the publisher dependency chain. Likewise distinguish registry
auth/first-publish from bounded visibility wait and failed installed smoke;
reuse existing wait/retry-safe helpers before suggesting new machinery.

## Feasibility, research questions and review handoff

Public callable Inspector production routes stay as implemented in worker code;
private fixture IPC/test hooks and package/browser harnesses are not supported
product operations. No new Engine callable route is needed for evidence review.
A missing product API belongs to feature feasibility reviews, not CI remediation.

During future execution, investigate only unresolved questions using primary
sources: GitHub Actions path-filter/job-condition/concurrency semantics; Rust
GNU target versus glibc symbol compatibility; crates.io trusted-publisher first
publication and npm/PyPI exact-version visibility. Inspect source and existing
logs first. No web research or network run was performed for this draft; no
recommendation to add a dependency or publication route is implied.

Before review, refresh relevant source and allocated0/1/5/6 findings, classify each
candidate keep/adjust/reject/already-done, and reconcile ownership against the
current master mapping. A fresh direct Sol-high reviewer checks substantive design
changes/source fit; Terra verifies prose/links and claim boundaries. Markdown
checks suffice for this deliverable; mandatory final source/installed/human gates
remain owed by their assigned slices. Handoff to9 contains actionable alternatives,
rough effort/risk and include/postpone recommendation;10 remains conditional.
