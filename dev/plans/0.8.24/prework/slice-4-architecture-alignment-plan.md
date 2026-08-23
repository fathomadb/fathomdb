---
title: 0.8.24 Slice 4 — architecture and code-alignment review plan
status: DRAFT
target_release: 0.8.24
---

# Slice 4 — architecture and code-alignment review plan

## Purpose

Test the Slice 0–3 proposals against both the authoritative architecture and
the code that actually ships. The result is a high-level, code-grounded
alignment review that distinguishes existing mechanisms from net-new work and
routes each discrepancy to an architecture change, a code change, or no
change.

## Required inputs

- The 0.8.24 plan and all completed Slice 0–2 records.
- Slice 3 product-document and architecture draft outputs.
- `dev/architecture.md`, `dev/design/`, the accepted ADR index, and
  `dev/interfaces/`.
- `.github/workflows/{ci,release}.yml`, release scripts, package manifests,
  binding crates/packages, the engine query/WAL paths, and their existing
  tests.
- The benchmark evidence index and the current performance-branch delta named
  by Slice 0; evidence is inspected, not re-run.

## Scope

1. Review and approve, reject, merge, or adjust each Slice 3 architecture CRUD
   proposal.
2. Map each proposed feature to the current architecture seam and concrete
   code/configuration anchors.
3. Record, for every candidate, what exists today, what is incomplete or
   inconsistent, and what would be net-new.
4. Identify code that contradicts accepted architecture, architecture that is
   stale against intentional shipped behavior, and proposals that require a
   new or successor decision.
5. Propose the smallest appropriate correction and allocate it to Slice 7 or a
   feature slice.

## Non-goals and no-implementation boundary

- Do not edit architecture, ADRs, interfaces, source, tests, workflows,
  manifests, release scripts, or release state.
- Do not implement a feature, dependency update, documentation correction, or
  test gap identified by the review.
- Do not run a benchmark, package build, hardware job, release dry run, or
  hosted workflow.
- Do not infer remote runner, registry, or external Memex facts that remain
  unavailable; retain them as unknown prerequisites.
- Do not claim a feature is implemented merely because a compatible seam or
  structural workflow guard exists.

## Review method

### 4.1 Establish authority and invariants

For each relevant subject, identify the controlling need/requirement, accepted
ADR, architecture section, interface contract, and test seam. Accepted ADRs
remain authoritative. A conflicting proposal requires a successor/amendment,
not an undocumented exception.

Carry these release invariants through every row:

- CPU installation and runtime behavior remain independently usable.
- CUDA selection is explicit and target-specific.
- Windows CUDA requires remote build/proof, not local compilation.
- Publisher retries never replace an immutable valid artifact.
- Real databases and installed packages provide product evidence.
- Windows WAL conclusions remain evidence-attributed and fail closed.
- CI stays informational and proportional unless the owner decides otherwise.

### 4.2 Perform the code-grounded pass

Inspect the actual implementation/configuration seam for each feature:

| Feature area | Minimum seam to inspect |
| --- | --- |
| Slice 10 CI | Workflow triggers/classifier, path routing, release interface, and contract tests |
| Slice 20 performance | Nominated engine query path, retained benchmark delta, correctness tests, and perf evidence contract |
| Slice 30 Tegra | Python build script, wheel metadata/name, compatibility docs, publisher route, and Jetson evidence workflow |
| Slice 40 Windows CUDA | PyO3 and N-API Windows surfaces, platform loader, remote executor selector, artifact transfer, and PowerShell smoke |
| Slice 50 WAL | Engine WAL/checkpoint behavior, Python installed path, Windows diagnostic jobs, and linked external evidence boundary |
| Slice 60 preservation | CPU artifact matrix, registry publishers, existing-version guards, and installed-package smoke scripts |
| Slice 70 integration | Release version/source-of-truth, release branch/main boundary, evidence retention, and publication authority |

Use file/symbol/test anchors for claims. Record a missing anchor as net-new or
unknown, never as an implicit implementation detail.

### 4.3 Classify alignment

Each reviewed item receives one classification:

- **Aligned/existing:** architecture and code agree; no change.
- **Aligned/extension seam:** the current design supports the proposal, but the
  feature is net-new work in its allocated slice.
- **Stale architecture/documentation:** intentional shipped behavior is sound,
  but an authoritative explanation needs an approved update.
- **Code defect:** shipped behavior contradicts an accepted contract and needs
  an owning implementation slice.
- **Decision required:** competing valid shapes or a signed-decision change
  requires owner disposition before design proceeds.
- **Insufficient evidence:** the review cannot attribute the discrepancy yet.

### 4.4 Propose and allocate corrections

For every non-no-change row, specify the smallest viable correction, affected
public/shared surfaces, decision prerequisite, verification implication, and
one primary destination. Public API changes require interface-document review
and, where an accepted decision changes, an ADR successor in the same owning
feature slice.

## Deliverable

`slice-4-architecture-alignment.md`, containing:

1. disposition of every Slice 3 architecture draft;
2. feature-by-feature architecture/code alignment matrix;
3. explicit exists-today versus net-new findings;
4. proposed code, architecture, ADR, or interface changes with allocation;
5. cross-cutting invariants and shared-file collision/serialization notes; and
6. a no-change statement for every reviewed surface that needs no correction.

## Completion and verification

Slice 4 is complete when:

- every Slice 3 architecture draft and every proposed feature slice appears in
  the alignment matrix;
- every implementation claim cites a current file/symbol/test anchor;
- every signed ADR/interface conflict is either withdrawn or routed to an
  explicit successor/update proposal;
- every gap distinguishes code work, architecture work, owner decision, or
  insufficient evidence and has exactly one primary allocation;
- no authoritative or implementation file changed; and
- scoped Markdown lint and `git diff --check` pass.

This is a high-level review, so compilation, tests, package builds, remote
execution, and hosted CI are neither required nor authorized.
