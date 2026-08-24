---
title: 0.8.24 Slice 10 — current-main CI and release interface design
status: ACCEPTED
target_release: 0.8.24
---

# Slice 10 — current-main CI and release interface design

**Reference:** `origin/main` at
`5e2a05e281571024a3e7bb305373915597a54078`, observed 2026-08-24.

## Design decision

Accept the current-main implementation without workflow or script changes.
The architecture has two separate entry surfaces: proportional informational
feedback for ordinary development and an explicit release control plane for
artifact rehearsal/publication. Target-specific CUDA extensions belong to the
slice that decides their identity, executor, artifacts, and proof.

## Trigger, job, runner, permission, and artifact matrix

| Surface | Trigger | Jobs / routing | Runner and permissions | Artifact boundary |
| --- | --- | --- | --- | --- |
| Main CI baseline | `pull_request`; `push` to `main` | Always-on current-tree/security and repository-integrity jobs plus `changes` | Hosted runners; top-level `contents: read`, `pull-requests: read` | Diagnostic artifacts only where a named job fails or reports. |
| Main CI verification | Classifier outputs; trusted exact `[ci-lite]` affects expensive scoped jobs only | `verify-fast` is the non-Markdown baseline; `verify`, security, embedder, WAL, wheel, and native jobs follow dependency-accurate categories | Ubuntu, Ubuntu 22.04, hosted Windows, and five hosted CPU-native matrix runners as declared | Spill logs, race report, and WAL diagnostics; no release artifact transfer. |
| Release control plane | `v*` tag push or explicit `workflow_dispatch` | Candidate/tag gates, normal CPU artifact matrices, existing Linux x64 CUDA route, publishers, installed smokes, npm promotion, GitHub release | Top-level `actions: read`, `contents: write`, `id-token: write`, narrowed per job; publisher jobs opt into OIDC as needed | Named build artifacts flow to publishers/smokes; dry-run CUDA receipts/witnesses stay same-run and non-publishing. |
| Existing self-hosted CUDA | Explicit dry-run or canonical release conditions | Hosted `verify-cuda-trusted-route` precedes self-hosted preflight/rehearsal; canonical build has its own condition | `[self-hosted, Linux, X64, gpu, cuda-12]`, `cuda-unmerged-preflight` environment, candidate jobs `contents: read` | Main-owned receipt, preflight witness/package bytes, then verified rehearsal bundle or canonical artifacts. |

## Classifier and fast/heavy ownership

- The `changes` job owns Rust, Python, TypeScript, Windows-WAL, CI workflow,
  verifier, Rust-test, security, and native-artifact categories.
- An exact `[ci-lite]` marker is trusted for an owner/member/collaborator
  same-repository PR, a direct push candidate commit, or a merge push's second
  parent. It does not affect release workflow execution.
- `verify-fast` owns the full developer bootstrap plus cheap suites.
- `verify` owns heavy Rust/Python/TypeScript suites and uses
  `bootstrap-heavy.sh`; changes to that helper route through the verifier
  harness category.
- Workflow changes deliberately select all scoped jobs. Documentation-only
  changes retain lightweight documentation/repository checks without forcing
  the heavy matrix.

## Self-hosted security boundary

The existing CUDA path establishes the reusable boundary pattern but not proof
for a future target:

1. `release.yml` must be the main-owned workflow; the hosted route check uses
   `github.workflow_sha`, not candidate code.
2. A dry-run candidate must be an immutable lowercase full SHA and an ancestor
   of `origin/main`.
3. The hosted job emits a same-run receipt before self-hosted candidate
   checkout.
4. The self-hosted job first checks out the main-owned control plane with
   persisted credentials disabled, verifies the receipt, and only then checks
   out candidate code with `contents: read` and persisted credentials disabled.
5. Runner labels and the `cuda-unmerged-preflight` environment constrain the
   existing route. Their presence is structural evidence, not proof that a GPU,
   runner, or environment approval is currently available.

Slices 30 and 40 may reuse or revise this pattern only after choosing their
target route. Slice 10 does not authorize a label, environment, or candidate
execution for either target.

## Architecture fit and target seams

| Owning slice | Interface handed off | Explicit non-claim |
| ---: | --- | --- |
| 30 | Start from the explicit release control plane; decide Tegra public identity, publisher, Jetson executor, artifact path, and smoke. Revise the plan if shared workflow code is required. | Existing Linux x64 CUDA and generic ARM64 CPU jobs do not prove Tegra publication or Jetson execution. |
| 40 | Decide Windows SDK matrix and remote CUDA executor before defining build, loader, transfer, and installed smoke. Revise the plan if shared workflow code is required. | Hosted `windows-latest` CPU/WAL jobs do not prove Windows CUDA. |
| 70 | Consume the Slice 10 SHA/interface record with feature evidence; preserve explicit release authorization. | CI structure, a green check, or an artifact upload alone is not release completion. |

The design aligns with `dev/architecture.md`, `dev/design/release.md`, the
Tier-1 platform ADR, and the active simplified CI design. It changes no public
Rust, Python, TypeScript, CLI, wire, package-identity, or runtime contract.

## Challenging aspects and resolution

| Challenge | Resolution |
| --- | --- |
| A future target could expose a route gap. | Do not create a dependency cycle. Close the current interface; the target slice owns a later explicit plan revision and test-first change. |
| Structural YAML can be mistaken for executor proof. | Label every local result structural and require target-native evidence in Slices 30/40/60. |
| Low-ceremony policy can be confused with live GitHub settings. | Record it as owner-approved repository policy; make no unqueried claim about branch protection or rulesets. |
| Release workflow carries broad top-level permission. | Record actual per-job narrowing and the main-owned/read-only self-hosted boundary; do not change it without an evidenced target design. |

## Design review and revision

The independent reviewer first returned **NEEDS-REVISION**. The initial plan
incorrectly depended on future Slice 30/40 ready designs and repeated stale
`[ci-lite]` prose. The revision:

- closes Slice 10 from current main with no dependency on future ready designs;
- assigns any later route gap to the target slice through explicit plan
  revision;
- records trusted PR, direct-push, and merge-second-parent lite semantics;
- converts the draft statements into accepted slice-local contracts; and
- records the real self-hosted security and evidence boundaries.

The reviewer then accepted **PASS for a no-workflow-change completion**.

## Implementation and TDD disposition

No behavior change exists to drive RED/GREEN. The existing routing,
fast/heavy, and WAL contracts are mutation-sensitive and pass against current
main. Creating a failing test that demands a speculative route would encode an
unauthorized design and provide no product value. Therefore TDD is correctly
inapplicable to this documentation-only integration decision. Any later
behavior mutation remains bound to RED, implementation, GREEN, and
`actionlint`.
