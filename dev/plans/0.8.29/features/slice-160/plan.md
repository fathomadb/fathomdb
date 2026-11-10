---
title: Slice 160 — Combined qualification and publication readiness
status: PROPOSED
target_release: 0.8.29
slice: 160
---

# Slice 160 — Combined qualification and publication readiness

## Maturity, goal and boundary

Close selected .29 engineering work with attributable composed evidence and
accurate release contracts. Former Slice 120 CLOSE maps here; new 120–150 scope
remains subject to checkpoint decisions. This draft authorizes no execution, environment change, cleanup, tag,
publication or release-complete declaration.

Inspected: [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md),
[master](../../../plan-0.8.29.md), [evidence](../../planning-evidence.md),
[.28 Slice95 status](../../../0.8.28/features/slice-95/status.md), its
[qualification inventory](../../../0.8.28/features/slice-95/qualification.md),
[.28 board](../../../runs/STATUS-0.8.28.md), and [release mechanics](../../../../design/release.md).
The .28 status supersedes stale board/procedural publication statements.
Published .28 is included at .29 baseline `6edc1155f`; format37 and existing
MCP/queue/Block1/B2 signals are inputs, not newly completed work. The .28 qualification was assembled: original full-run failures remain failures;
scoped corrections and unchanged-input comparisons close their stated findings.
SDK trusted publishing is already repaired and OIDC-tested; do not repeat that
repair or call its skipped build/publication jobs a product qualification pass.

## Draft needs, requirements and acceptance

Release-local IDs below are proposals; they add no canonical AC or threshold.

| Need / requirement | Falsifiable acceptance |
| --- | --- |
| N160-1/R160-1: selected scope is accounted for | A160-1: every selected slice has reviewed exit evidence; every excluded proposal has explicit HITL deferral, owner and residual consequence; unresolved mandatory work blocks readiness. |
| N160-2/R160-2: composed behavior holds | A160-2: source-bound full workspace, feature, required long/performance, security and affected integration evidence covers current inputs with truthful pass/fail/skip counts. |
| N160-3/R160-3: agents/SDKs see one contract | A160-3: both MCP formats pass supported conformance and composed lifecycle/queue/filter/flags/paging/budget/error/B2 controls; Rust/Python/TS native parity uses identified artifacts. |
| N160-4/R160-4: claims match shipped platforms | A160-4: current supported CPU/native rows and selected CUDA/platform/installed consumers satisfy approved support contracts; unavailable or unqualified cells remain explicit, not advertised. |
| N160-5/R160-5: delivery is reviewable | A160-5: version/notes/docs and immutable-candidate nonpublishing rehearsal agree; approved .29 GPU routing preserves protection policy; READY_TO_PUBLISH remains separate from publication authorization. |
| N160-6/R160-6: publication has real consumer proof | A160-6: only after separate HITL authorization and actual publication, registry-installed consumers prove persist/read, missing-record control, separate-process reopen, close and process exit before release completion. |

## Preparation, design and research fit

Draft `requirements.md`, `design.md`, `test-map.md` and `qualification.md` during
authorized execution; map requirements to existing oracles, source/artifact/configuration inputs, executors and gaps.
Read [active architecture](../../../../design/fathomdb-data-plane-architecture-v2.md),
current MCP architecture/interfaces and accepted slice ADRs before composing tests.
Propose reconciliation addenda for approved .29 contracts; the .28 architecture
profile is not evidence that integrated MCP behavior is absent.

Challenge attribution across shared fusion/cache/SDK/renderer seams and final
fork pins. Design an input-comparison/reuse table before expensive campaigns:
unchanged runtime, fixture/model/corpus, build features, toolchain and relevant
host inputs justify only their existing scope. Changed inputs trigger affected
reruns; broad corrections may require a new full phase. Preserve original exits,
failures, resumed scopes and limits; never relabel assembled evidence uninterrupted.

Use [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) methods
at owned seams: real-DB fault/reopen, resource release, boundary/property controls
and optimized/reference checks under approved identity. Defer exhaustive fuzzing
and platform/CUDA sweeps beyond approved scope. Format37, changed fusion identity,
final pins and new output/B2 contracts are the new evidence driving fit review.
Research only unresolved claims using primary MCP/SDK/toolchain/registry/NVIDIA
documentation; record retrieval dates and distinguish reported support from proof.
Consume Slice 120's [CUDA review](../../../../design/cuda-version-support-review.md)
and ratified policy outcome; do not infer fresh adoption data or policy approval.

## Prerequisites and sequencing

- Hard: approved 9/10 preparation, live .29 state/board and every selected 20–150
  exit or explicit HITL disposition. Included 140 pins precede affected 110/120
  integration and 130 final qualification; 160 consumes their final outcomes.
- Block1 follow-up exit after integrated 20–70 is a separate milestone before 90:
  full/feature/performance checks, both-format conformance, approved LoCoMo smoke
  and corrected item-4 remeasure. It never waits for 160; no90↔160 dependency.
- Consume that exit, then rerun invalidated composed scopes after 90/other changes.
  LoCoMo/item-4 cost, corpus, prompts and execution permission must be current;
  use approved budgets only. Changed ranking/identity must be stated in comparison.
- Missing checkout-owned `.venv`/pinned tools/native identity fails preflight;
  never borrow another checkout's environment or silently run system Python.
- Newly explicit delivery prerequisite: the first-tag CUDA bootstrap exception
  permits only v0.8.28. Obtain reviewed/approved .29 routing through 7/9/10 before
  publish readiness; preserve restricted runner/environment approvals.
- Serialize shared manifests/contracts/state/ledger writes and engine landings.
  Verify .28 ancestry/current integration tip before landing; isolate concurrent
  writers. Reserve approved resources; never time alongside builds, use K620,
  change R580/primary12.6 or assume Jetson reflash/venue access.

## Tasks and deliverables

1. Freeze selected-scope/decision and qualification inventories; reconcile inherited
   conformance failures against declared capabilities without an unstated waiver.
2. Prepare final clean source/configuration and generated views before broad runs.
   Run strict full `scripts/agent-verify.sh` once per final code phase, including
   workspace clippy/check; inventory `test-feature-complete.sh` targets and live
   prerequisites. Required long variants use `AGENT_LONG=1 scripts/check.sh`
   or reviewed exact selectors with unchanged accepted thresholds; report AC083
   tracked targets and scale results without creating numerical release gates.
3. Qualify integrated engine/filter/fusion/cache, MCP stdio/queue/shutdown/formats,
   B2 feedback/coverage/unknown spans, interruption/reopen and selected reset paths.
   Run approved final LoCoMo smoke/item-4 remeasure where changed inputs require it.
4. Complete appropriate five-target CPU/native installed-consumer/collector and
   distinct Windows WAL evidence, external Rust candidate consumer and selected
   Tegra/non-Tegra/CUDA artifact consumers. Bind exact bytes/features/pins and
   driver/library/PTX constraints; build success alone is not installed proof.
5. Reconcile release notes, maintained public/developer docs and examples: accepted
   non-exhaustive/source breaks, text default/structured opt-in, flags/filters,
   identity/ranking/provenance change, UTF-8 snippets, B2 coverage and CUDA support.
   Coordinate Axis W 0.8.29 with `scripts/set-version.sh`; Axis E stays independent.
6. Perform scoped package/auth/routing prerequisites, local rehearsal and separate
   immutable-candidate Release dry-run. Keep skipped publisher bodies explicit.
   Produce reviewed readiness evidence and release-state disposition, not a
   fabricated .29 publication receipt. READY_TO_PUBLISH requests separate HITL
   authorization before tag/publish; a `v*` push triggers real publication.
7. Under that later authorization, complete required publishers and fresh registry
   consumer lifecycle proof, then record actual release completion. Propagation
   failures retain their real exits and get bounded affected retries, not a new tag.

## Review, validation and stop points

Independent design/code review challenges scope completeness, identity semantics,
evidence reuse, installed consumers and publish boundaries before promotion.
Any behavioral repair gets its own failing test first and RED→GREEN evidence;
do not weaken tests or regenerate goldens. Preserve compiler diagnostics verbatim.
Use scoped Markdown/public-doc/release-state validators for metadata-only deltas.
Do not rerun the full matrix for documentation. Failed suites get scoped reruns;
repeat full qualification only for invalidating broad changes or an explicit gate.
Stop on missing setup/access, mandatory failure, unapproved cost/routing/policy,
unsupported claims or two repeated same-mode failures; record the issue for decision.
No cleanup, force-push, tag or publish follows from this plan.
