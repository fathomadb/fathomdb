---
title: Inspector 0.2.0 Slice 0 — proposed environment boundary
date: 2026-11-19
status: DRAFT
---

# Slice 0 environment design

## Existing substrate and proposed delta

[Scope](../scope.md) authorizes connected Inspector exploration on one unchanged
0.8.29 Engine/current-format build. The inspected planning checkout at
`609dd85c41fb7980e05faa1132a248d376d8ec89` has the record-browser foundation,
Rust workspace membership, a default CPU Inspector package, a tar builder and test tools.
Future execution first creates and enters the release checkout under the owner
ruling, then proposes other environment choices; it neither builds nor configures them.

The [HLD](../../../design/high-level-design.md) selects a Rust supervisor/local HTTP
shell and isolated worker with bundled browser assets. Installed runtime has no Node,
Python, TS build, development server or GPU dependency. Developer/test environments need
those tools where their gates apply. Keep that distinction in reports and support docs.
Missing tools are prerequisites, not grounds to narrow mandatory positive acceptance.

## Checkout and ownership boundary

The [owner ruling](checkout-decision.md) and [plan](plan.md) require future Slice 0
first to create and enter `inspector/dev/0.2.0` at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0` from the approved final
planning HEAD containing integrated port/scope/plans and ancestry to `6edc1155f`.
Reports for 0–9 then use that release checkout; 10 implements only individually
approved items there. Other environment/infra actions remain proposals. Recording
this decision performs no checkout creation or entry now. Generic preflight
assumptions about `origin/main` must not change selected 0.8.29 ancestry: inspect applicability
and report incompatible guards for individual 9/10 proposals, without bypass or Engine drift.
Each slice has one directly commissioned writer; serial slices reuse the selected checkout.
Concurrent mutation requires extra isolation; the shared release checkout is preserved.

## Proposed environment layers

| Layer | Existing evidence / challenge | Proposed choice for decision, not applied |
| --- | --- | --- |
| Rust/build | Toolchain 1.95.0; Inspector empty defaults; fixture-control test-only | Preserve locked source and selected default build. Inventory compiler/linker/C tools and target provenance; no Engine or registry pin change. |
| Python/native gates | Source checkout lacks executable `.venv/bin/python`; full verify fails before tests | Propose local Python >=3.11 with pip/pinned tools. Resolve native-binding installation constraints before approved setup in 10; never share/repoint another checkout's venv. |
| Node/TS/test tools | Historical Node 26.8.2; TS engines >=25 <26; CI Node 25.9.0; Node typings 26 | Separate Inspector JS/browser tools from TS SDK gate. Select a compatible developer runtime or propose declaration change for 1/7/9; no silent unsupported skip. |
| Browser/display | HLD Chromium 156.0.8078.4 and Playwright 1.62.1/build 1248 | Recheck exact available executable/tool pair later; keep keyboard/pointer/browser coverage with one declared pairing. |
| Linux artifact ABI | Historical Ubuntu 24.04.5/glibc 2.39 binary requires GLIBC2.39 | Propose 22.04-compatible CPU x86_64 build executor and installed target test. GNU triple alone is insufficient evidence. |
| Resources/trace | Four cores, 8 GiB available RAM, scratch >=2x fixture; native probes need ptrace | Inventory actual host constraints; use ordinary shared-host runs and preserve measured noise. Propose suitable executor if ptrace denied, without weakening strict checks. |
| CI/packaging | CI Rust filter omits src/inspector; package/tool provenance exists | Hand evidence to 7, then 9/10 for any routing/tool remedy; 70 owns installed qualification. No CI edit in 0. |
| External compute | Repository contains GCP/Jetson build helpers; no new Inspector host commissioned | Prefer existing suitable local CPU builder. Describe external alternative's necessity, permission/cost/data boundaries; no provisioning, connection, GPU or new support platform. |

## Formal release-planning policy and architecture fit

Review the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) and
[CUDA procedure](../../../../design/cuda-version-support-review.md). The
[accepted CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) has a recurring
review obligation despite this CPU-only product. Record its applicability honestly, reference
an applicable completed review if one exists, otherwise allocate the read-only review and
owner decision. Do not label absent research complete or implicitly amend policy. No NVIDIA
driver, `/usr/local/cuda`, toolkit install, artifact support or importance rating changes here.

Real databases, independent lexical/L2/adjacency expectations, property limits, bounded faults,
resource/cancel/close probes and browser/keyboard/human tasks fit this architecture. Global
MC/DC, generic Engine fuzzing, exhaustive allocation campaigns and GPU benchmark expansion
remain deferred. Format 37 and the real queue-cache defect make targeted freshness/boundedness
qualification relevant; they do not justify an unrelated assurance framework.

## Source qualification and downstream prerequisites

Public callable routes must be separated from private/test-only helpers before 20–50 use them.
The old HLD helper→`EmbedderChoice::Caller` path is stale: production open exposes Default/None;
Caller is test/evaluation-contained. Slice 4/40 must investigate supported public cache-only CPU
routes without changing Engine/API scope. This environment design does not certify model-route
feasibility, lexical rank evidence, full edge detail or historical snippet-gap claims.

Future primary-source questions cover Ubuntu/glibc builder ABI, supported browser/tool pairing,
Node/TS compatibility, Python native packaging and policy review applicability; no web research
is performed for this draft. Review actual design changes with fresh Sol-high, then Terra checks
source fidelity and document scope. No fake runtime tests or RED/GREEN for prose. Slice 9 receives
finite include/postpone proposals and unresolved prerequisites; 10 applies approved individual
items only. Final installed artifact provenance/ABI, offline launch, all views and close remain 70.
