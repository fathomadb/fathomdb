---
title: "0.8.29 Slice 10 — decision-bound release preparation"
status: PROPOSED
target_release: 0.8.29
slice: 10
---

# Slice 10 — decision-bound release preparation

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Goal, maturity and authority

Prepare only the workspace, tools, release metadata and bounded dependency/docs/CI
changes actually selected by Slice 9 after independent plan review and final HITL.
This draft grants no implementation authority. The [brief](../../planning-brief.md)
and [proposal register](../../proposal-register.md) control scope. November 10
Slice9 batches1/2 approved the bounded subset now specified in
[requirements](requirements.md), [design](design.md) and
[preparation tasks](preparation-tasks.md). Independent bounded review passed after3 FIX cycles;
final owner acceptance and implementation go-ahead remain pending. The original
drafting observations and unselected recommendations below are not current
approval or environment-readiness claims.
Do not create environments, switch branches, activate state, install, clean up,
change code/contracts, dispatch workflows, append ledgers or publish while drafting.

The [evidence register](../../planning-evidence.md) reports integrated .28 at
`6edc1155f`, format 37, W=0.8.28/E=0.8.0, and no live .29 state/board. Existing
MCP/queue/Block 1 and B2-2/3 plus partial B2-6 remain source baseline facts.
Node 26.8.2/npm 12.0.2 differ from Node 25/npm 11.12.1 contracts; owned venv and TS
modules are absent, root modules are shared by symlink, and 18GiB/98% disk is a
dated observation requiring refresh. No setup or product test was run for this draft.

## Inspected preparation sources

Read [Slice 0](../../prework/slice-0/plan.md), [Slice 1](../../prework/slice-1/plan.md)
and the Slice 7 scratch draft; Slice 9's plan/rulings were unavailable at drafting.
Read [preflight](../../../../../scripts/preflight.sh), [release discovery](../../../../../scripts/release-current.py),
[state renderer/checker](../../../../../scripts/check-release-state-views.sh), [version owner](../../../../../scripts/set-version.sh),
[root npm](../../../../../package.json), [TS manifest](../../../../../src/ts/package.json), [bootstrap install sites](../../../../../scripts/bootstrap.sh),
[bindings build contract](../../../../design/bindings.md), [release routing design](../../../../design/release.md) and
[CUDA tag guard](../../../../../scripts/release/check-tag-cuda-route.sh).
Memory index and verified `agent-worktree-stale-base-trap.md`, `session-must-enter-own-worktree.md`, `disk-cleanup-check-live-scratch.md` and
`full-gate-run-frequency.md` inform ownership, preservation and verification cost.

## Slice preparation: local needs, requirements and acceptance

Release-local draft labels only; no canonical AC additions.

| Need | Requirement | Falsifiable completion signal |
| --- | --- | --- |
| S10-N1 isolation | S10-R1 execute recorded scope in an owned workspace | S10-A1 names approved decisions, owner, checkout/branch/base/session cwd; other writers and existing work remain intact. |
| S10-N2 reproducibility | S10-R2 own tools, native bindings and resources | S10-A2 records selected pins, resolved paths, module/artifact identity, setup receipt and resource limits; borrowed installs or silent skips fail. |
| S10-N3 durable truth | S10-R3 bootstrap one .29 state authority | S10-A3 tracked state/board/views select exactly .29, preserve historical .28 and pass discovery/view/dependency checks without invented closures. |
| S10-N4 bounded change | S10-R4 apply individually selected preparation | S10-A4 every dependency/docs/CI diff maps to a ruling and required checks; unselected proposals have explicit dispositions. |
| S10-N5 safe handover | S10-R5 qualify preparation without release claims | S10-A5 independent review, actual scoped/full evidence and unmet platform/access prerequisites are recorded; downstream owners can reproduce setup. |

## Architecture reading, design draft and fit revision

Draft `requirements.md`, `design.md`, `test-mapping.md` and `preparation-register.md`
alongside this plan before implementation. Map each local requirement to exact
Slice 9 ruling, edit/install sites, owner, validation and stop condition. Reconcile
the master plan's older preparation allocation with the decision-bound ladder.
One writer owns release state and shared manifests/locks. Mutable venv, npm/native
outputs and receipts belong to the checkout; immutable caches need explicit input
identity. Do not run bootstrap wholesale through shared npm or editable bindings.
Resolve its editable-install guidance against worktree isolation first: propose
a wheel built from the exact target tree and installed noneditable into its own
venv; never repoint a shared venv or manually copy native libraries.
Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)'s
truthful skips, bounded failure controls and installed-package identity checks;
defer broad fuzz/coverage programs. [CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md)
and its [usage review](../../../../design/cuda-version-support-review.md) remain
authoritative; incomplete usage research and second-artifact delivery stay in 120.
Challenging fit points are state bootstrap before dependency preflight, native
ownership despite existing guidance, limited disk/swap, and protected first-tag
routing. Revise this draft from actual Slice 9 outcomes; widen no approval by inference.
Planned primary research is limited to unresolved selected package release/security
notes, supported tool installation instructions and GitHub runner/environment/OIDC
contracts; retain dates and uncertainty. No latest-version or access claim is made.

## Hard dependencies, conditional edges and conflicts

- Hard: actual 0–8 findings, interactive Slice 9 dispositions, reviewed decision-bound Slice 10
  plan, resolved material FIX-n findings (maximum four cycles) and final HITL.
- Hard before builds: approved workspace ownership/base, owned setup method,
  disk/scratch/concurrency budget and required tool access. A historical runner
  success proves neither current availability nor permission for the candidate.
- State bootstrap occurs in an approved existing isolated checkout before any
  release-dependent `--worktree`/`--expect-closed` preflight or new implementer spawn.
  Plain health/landing-only checks do not require a live state; do not weaken guards.
- Conditional: selected 2/6 cleanup/docs and 1 updates need itemized rulings; fork
  changes need 140 qualification before adoption. Selected 7 fixes precede affected
  20/30 checks; unrelated feature work is not gated on deferred preparation.
- Serialize state, version/lock, bootstrap and workflow writes. Reserve GPU/VM/
  Jetson resources with downstream owners; R580/primary 12.6 fixed, never K620.
- New prerequisites for 9: authorize an exact .29 first-tag route because current
  bootstrap accepts only `v0.8.28`; decide install-guidance conflict, shared-module
  replacement method and capacity. Preserve runner-group/environment reviewers.

## Proposed execution work and deliverables after approval

1. Freeze approved scope in the preparation register; refresh Git refs/ownership
   and capacity. Reuse only an explicitly assigned idle isolated .29 checkout.
   Another session's checkout requires a distinct workspace; never force a switch.
2. In that assigned checkout, bootstrap approved `release-state-0.8.29.json` and
   `STATUS-0.8.29.md`, declared views and baseline/dependency facts from Git; track
   them together, regenerate via the checker, verify discovery, then commit safely.
   Mark 9 accepted only from its real ruling; keep 10 open until its checks pass.
3. Run strict preflight before any new spawn; if selected, create a fresh `llm/`
   preparation branch/worktree from verified current .29 baseline, verify ancestry and required files, then switch the execution session cwd into it and recheck.
4. Provision approved command-local Node 25.9.0/npm 11.12.1, Rust 1.95 components,
   developer Python >=3.11 and owned venv/pip with Ruff 0.15.17/Pyright 1.1.410. Install owned root/TS modules from locks; build owned Python/TS native artifacts
   and record source SHA, import/module paths, versions, features and output digests.
5. Use approved owned build/scratch paths and numeric stop thresholds; measure
   restore/build/upload/cache-post peaks. Preserve live scratch, caches and standing runners; cleanup requires its own exact scope. Stop before exhausting capacity.
6. Run `set-version.sh --workspace 0.8.29` and `--check-files`; cover Cargo/lock,
   Python runtime/package, TS lock/platform packages. Keep independent E=0.8.0 unless separately justified and approved; do not publish or create release tags.
7. Apply only selected dependency/docs/CI items with their own evidence. Implement
   any bounded .29 routing decision with held GPU approval and unchanged protections; authenticated policy changes need their authorized actor, never a broad exception.

## Validation, independent review and exit

Draft/import validation is Markdown/link checking only. Metadata uses release-state and version checks; docs use Markdown plus public-doc checks when applicable.
Later behavior/script fixes require per-behavior visible failing tests first,
then green without changing the oracle; mechanical version bumps are exempt.
Use actionlint for workflows and targeted guard/package/native tests while editing.
Run required full `agent-verify.sh` once on final clean source/script phases with
owned tools, preserving exact diagnostics; ptrace denial needs an unchanged strict
rerun on a capable executor. Scoped passes and inherited .28 receipts are not full green.
Independent review checks decision-to-diff coverage, ownership, state reachability,
failure controls and bounded routing before safe .29 integration. Persist actual
receipts/dispositions and verify 10 closure from Git before downstream preflight.
Unknown ownership, failed setup/gate, public-contract expansion, unavailable access
or exceeded budget stops the affected work and returns its decision to 9; readiness
and eventual published-consumer qualification remain 160, publication separately authorized.
