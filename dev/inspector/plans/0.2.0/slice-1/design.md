---
title: Inspector 0.2.0 Slice 1 — dependency review design
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
---

# Slice 1 — review architecture

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Boundary and baseline

This is a design for a review deliverable, not a replacement dependency architecture. [Scope](../scope.md), [plan](plan.md) and [Inspector HLD](../../../design/high-level-design.md) remain authoritative. Source inspection used the unpublished Inspector 0.1.0 record-browser composition on unchanged 0.8.29 Engine source `6edc1155f`, packages 0.8.28/current format 37, planning base `829625818` on 2026-11-19. No sweep, network research, audit or upgrade has run for this draft.

The current product is a Rust executable with bundled standard DOM/SVG/ES-module assets. No UI framework, graph library, Node runtime, Python package or MCP server is required by the installed product. Its Engine dependency disables default features. Existing worker/session/read-only storage and compiled build-description boundaries are real; cached CPU semantic composition, lexical/vector/graph views and integrated qualification remain feature work.

## Review model and information flow

```text
Tracked manifests + locks + bot config + pin policies + actual feature/source facts
    -> attributable inventory and ownership/closure classification
    -> current primary-source advisory/compatibility research during execution
    -> individual retain/required/optional/external/unknown proposal items
    -> Slice 9 human ruling -> conditional Slice 10 approved in-scope changes
```

`review.md` is one compact report, not a new canonical acceptance register, dependency database or recurring automation. Each proposal row records package/source, declared and resolved versions, runtime/test/build/optional closure, owner, pin reason/authority, dated evidence and confidence, researched target if any, urgency, compatibility risk, effort and validation. Use links to existing records; do not create parallel hash inventories or reconstruct historical receipts.

Two dimensions are independent: necessity (required/optional/unknown/retain) and implementation ownership (Inspector/shared tooling/Engine/SDK/MCP/schema/fork). Thus a required external security or capability blocker remains visible without authorizing external implementation. Newer-version availability alone is insufficient evidence of necessity.

## Existing mechanisms and design allocations

| Local allocation | Existing source-backed mechanism | Design consequence |
| --- | --- | --- |
| S1-D1 ownership/closure | Root Cargo workspace includes `src/inspector`; one shared lock; Inspector engine defaults off. | Start from direct dependencies and actual target/features. A package in Cargo.lock is not proof it is linked into the product. Future Slice 40 CPU features require a separate closure assessment. |
| S1-R2 pin rationale | Exact sqlite-vec in Inspector/Engine; governed Candle registry pins and root kernel patch; empty root npm overrides. | Preserve qualified coupling and accepted decisions. Native SQLite/vec changes can alter format/query/bounded-read behavior even if APIs compile. |
| S1-R4 posture | Five weekly Dependabot ecosystems all have PR limit 0 under standing owner decision. | Root Cargo coverage already includes Inspector. Report real gaps/stale comments separately from a proposed bot-policy change; do not unpause it. |
| S1-R2 provenance | `build.rs` hashes the lock and selected source and checks Engine/model identity alignment. | A future dependency/feature change invalidates prior composition evidence and requires affected feature/install qualification; stable schema number alone is insufficient. |
| S1-R3 route feasibility | HLD public query adapter/private named-storage split; source now exposes snippet APIs absent from historical HLD claims. | First revalidate unchanged public routes. Do not propose a library bump to solve a stale design assumption; 4/30 own route review and feature feasibility. |

The [pin checker](../../../../../scripts/check-pinned-override-rot.py) authenticates declared governed sources and offline metadata. It is not a current universal advisory scanner. The npm snapshot lists four historical GHSA records and explicitly excludes Rust advisory assessment; no override currently exists. A future live override fails closed by policy. Review provenance and missing data instead of treating a successful offline predicate as “all safe.”

## Genuine challenges and planned research questions

- **Required security versus frozen dependencies:** Is a newly reported advisory applicable to this exact source/target/feature? Is there a compatible fixed release, a workaround that preserves acceptance, or a required external scope decision? Use RustSec/GHSA/package-vendor advisories and affected ranges; document unknowns. Do not assume the historical zero-advisory comments apply today.
- **Native/shared coupling:** Which rusqlite/libsqlite3-sys/sqlite-vec combinations are officially supported, and which changes affect extension loading, FTS behavior, incremental blob reads, bundled SQLite or the selected Linux distribution? Use official crate/release/SQLite documentation. Future implementation would need real format-37 lexical/numeric/value/lifecycle checks, not a compile-only claim.
- **CPU model closure:** Can Slice 40 use the current public cache-only constructor without download/default-loader/reranker features or GPU linkage? Inspect actual cfg/API and fork qualification evidence before reading official release notes for potential changes. Do not import loader-test-hooks, private vector-stage helpers or benchmark-only routes into production.
- **Tooling versus artifact:** Which browser/Playwright tools are actually provisioned and pinned, and does a proposed tool update change only tests or distribution assumptions? Coordinate with 0/7. Existing HLD names Playwright 1.62.1/Chromium 156.0.8078.4; this is historical selection evidence, not a newly verified installed pairing.
- **Fork governance:** Does a proposed dependency change preserve the qualified fork items/static-CUDART boundary and pin chain? Use exact fork source/publication records and primary upstream notes; external code changes belong in their owning projects with separate authority. No CUDA driver/support change follows from Inspector's CPU scope; 0 owns formal policy review.

These are future research questions. This draft makes no latest-version, vulnerability or upgrade-recommendation claim. Generic upstream activity monitoring, alternate vector backends and unsupported GPU/ONNX expansion would contradict or exceed current accepted scope.

## Review prerequisites and exit evidence

Before review, refresh the baseline/allocated-item delta, confirm Slice 0's release-checkout entry under the owner decision, and make source/lock/policy references resolvable. Read actual memory files `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`, `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md` and dependency/fork entries indexed in the real memory store. The direct-agent mandate specializes the older library-Steward pattern; no such role is commissioned here.

Sol-high reviews actual report/design changes for scope, pin reasoning and architectural fit; Terra verifies coverage, critical evidence and scoped Markdown checks. No artificial runtime test or RED/GREEN is created. Reuse existing tools; if new measurement tooling is later approved, retain it in the repository with purpose and rerun command rather than leaving scratch-only gates. No new tool is needed to write this review.

Exit: every relevant pin has an attributable reason or explicit gap; required/optional and external ownership are distinct; every proposed change has a validation/decision path; Slice 9 can decide individual items without assuming a bundle. Slice 10 remains conditional. Required positive Inspector features and final installed gates stay open until their owning slices actually qualify them.
