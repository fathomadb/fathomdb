---
title: 0.8.29 Slice 0 — environment and project infrastructure proposal
status: COMPLETE
target_release: 0.8.29
slice: 0
---

# Slice 0 — environment and project infrastructure proposal

## Goal, authority and maturity

Propose a reproducible, isolated release workspace and bounded platform/resource
inventory for Slice 9 disposition and Slice 10 preparation. This is a proposal-only
slice: after design review, its remaining work is finishing and validating these
documents, not provisioning the proposed environment. This prework changes no
Git state, tools, environments, runners, dependencies or product contracts.
The [planning brief](../../planning-brief.md) controls proposal-only sequencing;
the [release plan](../../../plan-0.8.29.md) places state/board activation in
Slice 10 after the prerequisite proposal/disposition work. The current
[direct execution mandate](../../direct-execution.md) supersedes earlier generic
waiting language; approved scope advances through the required reviews.
Existing MCP, queue, Block 1 and format-37 source remain the integrated baseline.
Owner seq-368 fixes .29 at existing format37 with no database migration. Owner
seq-367 confines140 to necessary110/120 fork repairs and their repair-release
versioning; broader Candle maintenance belongs to .30. Keep the query scaffold.
No previous .29 slice is certified complete. Existing Slice0 outputs are inherited
research from this chat, not a newly completed execution phase.

The current direct workflow uses one slice owner, an independent Astra-medium
design review before finishing the proposal, then independent Astra-medium output
review. The parent controls those transitions and repository imports. No Steward
or Orchestrator role or launcher is part of this slice.

## Current observations and inspected sources

The [environment register](environment-register.md) is the single local inventory.
It distinguishes inherited November10 probes (approximately13:35UTC), current
source inspection and future preparation. Those probes were not rerun for this
reconciliation. Live capacity, installed tools and remote access must be rechecked
in the execution checkout before use. No product or setup campaign ran here.

The archived checkout's shared root npm symlink and18GiB free-space observation
are historical. The replacement checkout's inherited inventory reports no local
venv/modules/target and about16GiB root free; neither figure establishes current
build headroom. Earlier read-only Git inspection recorded branch
`llm/0.8.29-chat` at product-source baseline
`6edc1155f039f87cd0a37660cb27ae4373bbce71`. A November10 follow-up recorded the
planning-only baseline `40d82c2e75d1d70fea61f327bf5b2ffcb8a1f479` on that branch;
product source/configuration did not change between those commits. Neither SHA
is a permanent live-HEAD claim: derive actual HEAD and branch from Git before
execution. Remote refs were not fetched for either inspection. Source inspection
at the earlier baseline confirms W0.8.28/E0.8.0, format37, Rust1.95,
Node25.9.0/npm11.12.1 and Ruff0.15.17/Pyright1.1.410 pins.

[Recorded decisions](../slice-9/decision-log.md) already approve task-owned
/mnt/data build/scratch, command-local Node/npm pins, owned Python/native setup,
contained Rustls/DOMPurify work and bounded state/docs/survey/route preparation.
Do not reask those scope choices or add generic final permission. Slice10 must
complete its own review gates and resolve exact paths, capacity, artifact identity
and operational checks before preparation. W-version coordination, actual fault venues, policy/toolkit changes,
priced work, optional experiments and publication retain their explicit gates.

Inspected [reconciliation](../../reconciliation-plan.md),
[forward integration](../../forward-integration.md),
[Slice95 status](../../../0.8.28/features/slice-95/status.md) and
[qualification matrix](../../../0.8.28/features/slice-95/qualification.md).
Tool contracts: [Cargo](../../../../../Cargo.toml),
[Python](../../../../../src/python/pyproject.toml),
[TS](../../../../../src/ts/package.json), [bootstrap](../../../../../scripts/bootstrap.sh),
[preflight](../../../../../scripts/preflight.sh),
[verification](../../../../../scripts/agent-verify.sh), and
[bindings](../../../../design/bindings.md). Memory index and verified entries
`windchill3-never-change-driver-or-primary-cuda.md`,
`agent-worktree-stale-base-trap.md`, `session-must-enter-own-worktree.md`,
`windows-vm-for-fathomdb-testing.md` and
`0.8.28-slice42-sqlite-bump-lessons.md` informed isolation and host constraints.

## Draft needs, requirements and acceptance

Release-local proposal IDs only; canonical acceptance is unchanged.

- N29-0-1: contributors need reproducible tools and native bindings without disturbing other sessions.
- N29-0-2: reviewers need a truthful platform/resource inventory before allocating qualification work.
- R29-0-1 / AC29-0-1: each version/tool axis names its source pin, dated observation or unknown, preparation owner and retain/prepare/conditional action; approved scope is distinguished from pending execution.
- R29-0-2 / AC29-0-2: specify a reproducible base/ownership/session check and source-feature-interpreter artifact identity check; reject stale bases and borrowed mutable environments.
- R29-0-3 / AC29-0-3: each platform row names its route, feature/artifact scope, selection owner and unmet prerequisite; historical access does not imply current readiness.
- R29-0-4 / AC29-0-4: record known/unknown capacity, corpus/model identity, preservation exclusions and stop conditions; missing measured phase budgets receive an explicit9 disposition and must be resolved by10 before affected workloads.
- R29-0-5 / AC29-0-5: map existing decisions to10 preparation order, including state bootstrap before strict dependent preflight; record remaining gates without an executed-preparation claim.

[Requirements and checks](requirements.md) bind these local IDs to release
N29-D/R29-REL-D and sampled canonical needs. No global AC or numerical gate is
created. Canonical format36 applicability text is stale against accepted format37;
its bounded correction already belongs to approved10, not this proposal.

## Architecture/design preparation and platform inventory

Keep one writer per checkout and checkout-owned venv/native outputs. Separate
shared immutable download/model caches from mutable installs, build outputs and
receipts; cache reuse needs exact inputs, never borrowed binding ownership.
Python package floor remains 3.10; developer interpreter must satisfy current
bootstrap/scripts, preferably >=3.11; retain Ruff 0.15.17/Pyright 1.1.410.
Propose noneditable wheel installation into the owned venv for linked worktrees;
resolve bootstrap's editable-install guidance against ownership rules before use.

- Browser applicability: the published JS binding targets Node native addons; propose Chrome/Firefox checks only for a selected documentation or tooling flow, and explicitly record other browser cells as not applicable rather than adding a product browser matrix.
- CPU native matrix: Linux x64/ARM64, macOS x64/ARM64 and Windows x64; use existing [CI](../../../../../.github/workflows/ci.yml) and [release](../../../../../.github/workflows/release.yml) routes, without dispatch or publication in prework.
- Recorded self-hosted venues: GPU 168/group 3 on windchill3, Windows 31 on `MEMEX-AFE8V4I00` using the existing `gh-runner-wonl-win11` VM, and Jetson 22 on `ubuntu`. Slice 95 records identity run 42726255745 success; that establishes historical delivery, not present access or qualification.
- [Runner-readiness workflow](../../../../../.github/workflows/runner-readiness.yml) is main-only, identity-only and three-minute bounded; do not repurpose it as .29 candidate qualification.
- Preserve standing GPU installation `/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`; use a separately owned path if `/mnt/data` is selected. No runner/configuration cleanup is implicit.
- GPU contract: R580 and primary 12.6 fixed; RTX 3090s are documented venues, never K620. Map selected GPU UUID/ordinal before future `cuda:0` work. No reset/fault injection belongs in this slice.
- MCP CUDA build proposal uses existing `fathomdb-mcp --features embed-cuda`; package remains `publish=false`. Distinguish compiled binary, runtime/provider smoke and installed-consumer evidence.
- Keep classic Tegra and genuine non-Tegra ARM driverless cells separate. CUDA13 library qualification does not establish a second product artifact; Q5 Tegra CUDA 13 remains unmeasured.

Read [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
apply truthful skips, artifact identity, lifecycle/resource checks and bounded
wrong/missing-tool controls; defer new broad fuzz/exhaustive environment matrices.
The [CUDA procedure](../../../../design/cuda-version-support-review.md) and
[accepted policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) constrain
host/artifact choices. Fresh quantitative adoption research remains incomplete;
propose primary NVIDIA/JetPack/tool/runtime data collection in 120, with accessible
raw sources and owner ratification, rather than inventing fresh shares here.

## Proposed tasks, dependencies and deliverables

1. Reconcile `environment-register.md`: source pins, dated inherited probes, current read-only checks, limits and preparation owners; retain Slice1 dependency ownership.
2. Reconcile `requirements.md` and `design.md` drafts mapping the local IDs to workspace ownership, platform routes, resource budgets and artifact/corpus identity checks.
3. Reconcile `preparation-proposal.md`: carry approved workspace/environment scope into10, distinguish existing-checkout state bootstrap from later dependent-worktree creation, and specify strict preflight and source-bound setup checks. Describe future provisioning only.
4. Reconcile `platform-resource-register.md`: required versus conditional CPU/GPU/Tegra/Windows cells, availability checks, exact data/model digests still needed, resource reservations and deferred venues.
5. Submit this bounded package for independent design review, finish/validate the proposal after PASS, then obtain independent output review. Carry unresolved preparation facts to9/10; no tool/environment execution belongs to0.

Hard edges: this proposal requires only current evidence;9 requires 0–8 outputs;
10 requires recorded9 dispositions, retaining existing approvals and resolving
only genuinely open scope choices through the direct review pipeline. State discovery must succeed before future
`preflight --worktree/--expect-closed` can pass. Conditional edges: approved
cleanup from 2 may supply disk; 1 supplies pin decisions; 7 supplies build/CI cost
recommendations. Coordinate GPU/VM/Jetson reservations with 30/100/110/120/130;
serialize mutable environment and release-state writes. Preserve other worktrees.

## Review, validation, stop points and completion

Proposal completion requires independent design and final output review, all five
AC29-0 checks with inspectable mappings, unknowns and decision owners, and resolved
relative links. The parent alone imports files and runs `scripts/agent-lint-md.sh`
with owned tooling. Until then scratch link/source checks are limited validation;
missing tooling is reported without installing it in0. No full product gate is due.
Slice0 is documentation-only; do not invent code or failing tests for it. Later
Slice10 script behavior changes require failing-test-first controls for missing
owned venv, wrong pins and stale base, followed by independent review and the
applicable final gate; mechanical version bumps retain their test exception.
Stop preparation on unknown workspace ownership, insufficient approved capacity,
unavailable corpus, borrowed native identity, failed strict preflight or changed
host CUDA invariants. Carry blocked platform cells as unmeasured to 9/160.
No new uninterrupted green claim is inherited from .28's assembled campaign.

## Direct execution completion

Proposal outputs completed after independent Astra-medium plan and final-output
PASS, with one corrected provenance finding. [Review and validation](review-log.md)
record the explicit tooling limitation and downstream prerequisites. No setup,
product code/test, live-platform readiness or full-workspace qualification is
claimed. Subsequent slices consume these completed proposal outputs.
