---
title: Inspector 0.2.0 Slice 1 — library and pin review plan
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
---

# Slice 1 — dependency evidence and proposals

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Goal and authority

Produce a review-only library/Dependabot/pin sweep, with actionable proposals for Slice 9 and conditional Slice 10. Do not apply upgrades, change dependency policy or resume bots. [Adopted scope](../scope.md) governs; see the future [master plan](../plan.md), [workflow](../slice-workflow.md) and local [design](design.md). This draft records source inspection, not a completed sweep or clean advisory claim.

Inspected checkout: `/home/coreyt/projects/fathomdb-worktrees/0.8.29-inspector-additional`, branch `llm/0.8.29-inspector-additional`; planning base `829625818`, unchanged Engine base `6edc1155f`, workspace packages 0.8.28, Inspector 0.1.0, compiled format 37. Scope adoption is active; features and detailed designs are not implemented by adoption.

In: tracked-manifest/lock coverage, relevant resolved versions, ownership and feature closures, known pin reasons, Dependabot posture, advisory/compatibility research and ranked proposed actions. Inspect repository-wide dependencies as evidence, naming Inspector, shared tooling, Engine/SDK/MCP or fork ownership explicitly.

Out: installing tools; resolving/updating lockfiles; modifying manifests, source, overrides, policy or bot configuration; changing public pins, Engine/SDK/MCP/schema/ranker/API behavior; model downloads/cache repair; CUDA/driver changes; release tags or publication. A required external change is an escalated scope proposal, not an implicit Slice 10 implementation item.

## Known baseline to revalidate

Versions below are actual manifest/lock facts, not available-update claims. Sources: [Inspector manifest](../../../../../src/inspector/Cargo.toml), [workspace manifest](../../../../../Cargo.toml), [Cargo lock](../../../../../Cargo.lock), [Engine manifest](../../../../../src/rust/crates/fathomdb-engine/Cargo.toml).

| Inspector dependency / role | Declared requirement → locked version | Current meaning / review question |
| --- | --- | --- |
| Engine + schema / runtime | local path / workspace → both 0.8.28 | Engine uses `default-features = false`; use unchanged selected 0.8.29 source, not inferred package compatibility. |
| rusqlite / runtime | `0.41`, bundled/hooks/blob → 0.41.1 | Shared Engine SQLite link boundary; blob is existing Inspector-local bounded read wiring. Locked `libsqlite3-sys` 0.39.1; quality roadmap identifies SQLite 3.54.1. |
| sqlite-vec / runtime | `=0.1.9` → 0.1.9 | Shared exact extension pin; no routine Inspector bump. |
| serde / runtime | `1` + derive → 1.0.228 | Review actual serialization/API impact, not major number alone. |
| serde_json / runtime + build | `1`, runtime raw_value → 1.0.149 | Payload precision/raw-value and build-description consumers matter. |
| sha2 / runtime + build | `0.11` → 0.11.0 | Inspector build digest and runtime hashing; lock also contains 0.10.9 for other consumers. |
| getrandom / runtime | `0.3` → 0.3.4 | Lock also contains 0.2.17 and 0.4.2; distinguish actual consumers before deduplication proposals. |
| libc / runtime | `0.2` → 0.2.186 | Native/process safety and target ABI; no inferred advisory status. |
| tempfile / test | `3` → 3.27.0 | Test-only closure; installed product does not need a development runtime. |
| proptest / test | `1` → 1.11.0 | Existing property harness; no new framework needed for this prose slice. |

Relevant shared facts from [embedder manifest](../../../../../src/rust/crates/fathomdb-embedder/Cargo.toml), [pin policy](../../../../../scripts/pinned-override-rot.json), [root npm manifest](../../../../../package.json), [TS manifest](../../../../../src/ts/package.json) and [Python manifest](../../../../../src/python/pyproject.toml):

- Optional `candle-core-fathomdb`, `candle-nn-fathomdb`, `candle-transformers-fathomdb` are exactly 0.11.0; `cudarc-fathomdb` exactly 0.19.10 is CUDA-feature gated. Fork qualification, CPU AArch64 behavior and pool/device interfaces are reasons to preserve their reviewed coupling.
- Root `candle-kernels` patch uses `coreyt/candle-fathomdb` revision `182ceab0547348993b6dbec4b1aed610dcc8b23f`; lock package version remains 0.10.2. Rationale: qualified static CUDART linkage with dynamic driver discovery. Root patches do not propagate to published downstream Rust consumers. The pin checker marks external Rust advisory posture unassessed.
- Optional tokenizer requirement is `0.20` (lock 0.20.4); lock also contains 0.22.2. Optional eval-only ONNX `ort` is exactly 2.0.0-rc.10: manifest records rc.12 ort/ort-sys VitisAI mismatch and separately tracked stable adoption. Neither is automatically an Inspector runtime dependency.
- CPU semantic work at Slice 40 must select/revalidate its actual feature closure. Current Inspector manifest does not enable model features; the HLD plans a cache-only public CPU constructor. Do not classify every lockfile model/GPU package as installed Inspector code.
- Root npm locks markdownlint-cli2 0.23.3, markdown-it 15.0.1, js-yaml 5.4.1, linkify-it 6.1.0 and smol-toml 1.8.0; root overrides and policy `npm_overrides` are empty. The checked-in advisory snapshot is dated 2026-08-04, not current registry proof.
- TS dev requirements/lock: TypeScript `^6.0.3`/6.0.3, `@types/node ^26.1.0`/26.1.0, `@napi-rs/cli ^2.18.4`/2.18.4. Its Node engine is `>=25 <26`; historical Inspector development Node 26 is a different tool context, requiring Slice 0/7 allocation rather than an automatic TS change.
- Python runtime dependencies are empty. Tooling pins Ruff 0.15.17 and Pyright 1.1.410 require explicit reproducibility review and matching guards/tests; maturin `>=1.9,<2` records measured PEP-639 packaging compatibility. These are shared development/binding obligations, not installed Inspector dependencies.
- [Build description](../../../../../src/inspector/build.rs) hashes Cargo.lock, Inspector and selected Engine/schema/query/embedder sources; selected BGE-small revision/assets are read from unchanged source and drift-checked. Model asset pins are compatibility evidence, not library upgrades.

[Dependabot configuration](../../../../../.github/dependabot.yml) has weekly pip `/src/python`, Cargo `/`, npm `/src/ts`, npm `/`, and GitHub Actions `/` entries, each with PR limit 0. Cargo `/` now includes Inspector as a workspace member. The pause survives its original recovery reason by explicit standing owner decision; propose any posture change individually. Config comments cite historical 2026-09-02 checks, not current advisory results. Intentional exclusions include ignored evaluation environments, corpus/research requirements and release-skew fixtures. Reconcile tracked manifests rather than adding ignored environments as noise.

[sqlite-vec policy](../../../../deps/sqlite-vec.md) requires ADR amendment for bumps; [accepted ADR](../../../../adr/ADR-0.6.0-sqlite-vec-acceptance.md) accepts sole-maintainer risk, with no fallback or periodic activity-monitoring program. Preserve that ruling. Historical dependency notes may contain obsolete paths or terminology; source/manifest facts win for current usage, accepted decisions retain authority.

## Local DRAFT requirements, acceptance and design allocation

No new user need is proposed: this maintenance review supports existing [needs](../../../needs.md), [requirements](../../../requirements.md) and [acceptance](../../../acceptance.md). The identifiers below are slice-local draft work items, not additions to the canonical acceptance register.

| Draft item | Existing allocation supported | Observable completion check | Owner |
| --- | --- | --- | --- |
| S1-R1 / AC1: attributable inventory | UN-01/09; V01-N06; INS-X09 | Every tracked production/tool manifest family is classified; Inspector direct dependencies and shared/native/model closure are separated; requirement and resolution are distinct. | 1 evidence; 0/7 environment and packaging |
| S1-R2 / AC2: justified pin assessment | V01-N06/N07; INS-X01/X09 | Each relevant exact pin/patch has source, reason, owner, freshness and retain/research/change proposal; missing rationale stays unknown. | 1 evidence; 9 decision |
| S1-R3 / AC3: required vs optional decisions | INS-04/08/09; INS-X07/X09 | Every candidate gives evidence, applicability, compatibility/blast radius, ownership, urgency and validation cost; no required positive capability is waived as unavailable. | 1 proposals; 9 ruling; conditional 10 |
| S1-D1 / AC4: composition boundary | HLD A/Q/R/K and V01-S02 | Report distinguishes public callable, private and test-only routes; no proposed upgrade imports a private ranker or widens product features. | 1 design; 4 review; 20/30/40/50 qualification |
| S1-R4 / AC5: posture and uncertainty | INS-X07/X09 | Dependabot coverage/pause and advisory source/date/limits are explicit; no old/offline result is described as current clean. Repository implementation/configuration diff remains empty. | 1 review; 9 posture ruling |

Keep all scope changes local until approved; do not edit global or Inspector canonical acceptance files in Slice 1. Substantive canonical changes, if needed, become individually identified Slice 9 proposals.

## Prerequisites and future preparation

1. Slice 0 creates and enters the release checkout under the owner decision; reuse it sequentially. This slice creates no checkout. No working .venv is required to read manifests, but missing checkout-owned tools constrain any claimed checks.
2. Re-read adopted scope, this draft/design, relevant actual memory and accepted dependency ADRs; inspect actual manifests, locks, pin metadata, public interfaces and relevant code at the execution SHA.
3. Enumerate changes since draft: source/lock/config/tool versions, approved feature allocations, prior dependency remediation and external ownership. Record keep/adjust/reject/already-done with reason and source. Update only the review method/evidence affected; do not repeat settled research.
4. Obtain read access to current registry/advisory/Dependabot evidence through available authorized tools; honor repository egress policy. Missing access is an evidence gap, not a finding of no advisories. Do not install tools or modify local caches/configuration merely to manufacture a sweep result.

## Future execution method and deliverables

1. Build one concise `review.md` in this slice directory: dated baseline, coverage/inventory, pin-reason table, advisory and compatibility findings, candidate decisions. Read-only enumeration may span the repo; use existing manifests/locks and existing tools rather than creating a new audit framework.
2. Reconcile actual tracked manifest families with Dependabot paths, including workspace members, action SHA pins and Inspector browser automation imports. Separate untracked local tools/ignored eval artifacts from distributed code. Unknown automation-tool version/provisioning belongs to 0/7.
3. Research only questions affecting decisions: official release notes/MSRV/ABI/API changes; RustSec/GHSA/vendor advisories and fixed versions; npm/PyPI/crates metadata; upstream/fork qualification records for pin provenance. Record date, exact affected package/source/version and reachable feature/target. Do not invent a latest version from a version range or a historic report.
4. If using existing read-only audit tooling, preserve its unmodified findings and scope. Distinguish current advisory data from offline cached results; do not run a resolver/update command or accept automatic fixes. No current sweep/research is executed by this planning draft.
5. Classify candidates as REQUIRED for approved Inspector acceptance/security/packaging with concrete reachability evidence; OPTIONAL for evidenced benefit without a required blocker; EXTERNAL for Engine/SDK/MCP/schema/public pin/fork ownership. Add RETAIN when a pin remains justified and UNKNOWN where data is missing. An EXTERNAL item may also be a required blocker; do not hide it under optionality.
6. For each change proposal give current→researched target, reason/advisory, affected closure and files, API/MSRV/native/model/cache/distribution compatibility, pin/ADR constraints, retain/defer alternative, owner, risk, effort and specific future validation. A version being newer alone does not make a REQUIRED item.
7. Send decision-ready individual proposals to 9; link overlapping 0/2/4/6/7 findings instead of duplicating implementation. No preselected upgrade bundle. Slice 9 records human choices and any required separate scope; 10 implements only separately approved in-scope items. Rejection/defer of a mandatory blocker leaves release acceptance unresolved.
8. Perform a single coherent review of report/design changes with fresh directly commissioned Sol-high; Terra verifies source attribution, inventory/coverage, proposal traceability and Markdown/link correctness. These are document reviews, not runtime qualification or automatic approval.

Candidate research decisions already identifiable: preserve or explicitly amend sqlite-vec coupling; confirm fork/static-link pin reasons still apply to selected CPU closure; assess fresh advisories without misusing the npm-only snapshot; reconcile current paused Dependabot posture; identify browser tooling provenance; assess shared tool pins without forcing them into Inspector runtime. No update target or vulnerability is asserted here.

## Verification, status and cleanup

Check report rows against actual manifests/locks and official research; sample every REQUIRED or EXTERNAL blocker, all exact pins and bot entries. Verify no dependency/source/config edits were applied. Run relevant Markdown validation (`scripts/agent-lint-md.sh`) for the documentation deliverable using the commissioned checkout's tools. Do not run the full workspace gate solely for prose; record missing prerequisites and unrelated gate failures truthfully, with scoped reruns only for fixes in the blast radius.

No artificial RED/GREEN is required for this review-only slice. Conditional future implementation gets its own behavior-specific failing RED commit and separate GREEN, unchanged intent/oracles, independent Sol-high code review and Terra verification; mechanical-only version changes follow the repository exception. Feature tests use real databases, independent lexical/numeric/adjacency oracles, bounded property/resource/cancellation/browser checks as allocated to their feature owners. Defer global MC/DC, engine fuzzing and broad allocation campaigns. The quality roadmap supports focused dependency compatibility and installed tests; the current thin Inspector closure and format-37 port evidence refine reachability, not release qualification.

Only during execution write concise `status.md` with actual work, source/research dates, proposals, checks and unresolved blockers. COMPLETE means the review deliverable and required scoped checks are complete, never that all dependencies or runtime gates are green. No mandatory positive result becomes an unavailable pass. No borrowed .venv. Preserve release worktree and unrelated changes; integrate and clean only any slice-owned extra branch/worktree after verified integration. No hook bypass, force push, tag or publication.

## Future execution prompt

This is PREWORK: “implement the slice” below means produce/review the dependency assessment and proposals only. It does not authorize applying any proposed upgrade, cleanup, environment change, canonical contract edit or bot posture change. Reviews apply to actual document/design changes; the generic TDD/code language does not create runtime work for this prose slice. The commissioning parent directly spawns this slice's sole writer; reviewers remain read-only. Reuse the selected report checkout sequentially, never main. No Steward or Orchestrator.

```text
Work on and complete Slice 1 for Inspector inspector/dev/0.2.0 (and in a worktree).

Ensure the slice has planned to do any needed work regarding requirements+acceptance criteria, design, implementation, and testing; each should be slice-complete and correct. The slice might have slice-related "change plan" documentation for guidance. The slice's plan should:
- identify and enumerate changes since slice's plan was written (review drafted slice plan, review in-repo related work done, review assigned feature/functions, review allocated draft items).
- Evaluate the enumerated output, and if needed, update/approve/reject/adjust the drafted plan, ensure it is complete but not overbuilt.
- add/adjust needs, requirements, acceptance criteria.
- write/update design (design additions and changes should be reviewed by a design review subagent (gpt-6.1-sol-high).
- The plan should then describe implementation with TDD (RED/GREEN method) and code review subagent (gpt-6.1-sol-high).
- Use a subagent (terra) to test/verify.
- Write Slice status record. Clean-up after itself: While a wt or branch should not be necessary for a slice, if a new worktree or branch was used, merge back and clean-up the worktree.

Keep ceremony to a minimum. Stay focused on the work-at-hand. When resolving a specific finding, do not run full regressions unless the changes are comprehensive enough to warrant the overhead. Generally, solve for the work outlined in the slice; check for impacts in the blast radius; be pragmatic. (do not use Steward or Orchestrator.)

After the planning for requirements+acceptance criteria, design, and implementation is planned: implement the slice according to the plan.
```
