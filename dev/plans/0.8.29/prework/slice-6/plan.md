---
title: 0.8.29 Slice 6 — stale-document evidence review
status: PROPOSED
target_release: 0.8.29
slice: 6
---

# Slice 6 — stale-document evidence review

## Maturity, goal and boundary

- Proposal only: this draft identifies bounded observations and plans the remaining audit; neither the review nor release prework is complete.
- Give users, library consumers/developers and contributors a dated, source-grounded correction register before Slice 9 selects preparation.
- Follow the [brief](../../planning-brief.md), [evidence limits](../../planning-evidence.md) and [reconciliation](../../reconciliation-plan.md).
- Baseline: 6edc1155f on November 9, 2026 includes completed .28 and format 37; no .29 product publication or activated release state is inferred.
- Scope includes public guides/references, internal interfaces/designs, contributor setup and release evidence wording; comments enter only when they misstate a documented contract.
- Exclude corrective edits, canonical needs/requirements/acceptance changes, cleanup, installs, host configuration, product fixes and publication. Propose execution through 9/10 or the owning feature slice.

## Sources actually inspected and evidence limits

- Read repository [AGENTS.md](../../../../../AGENTS.md), the shared brief/evidence register and the memory index in the existing FathomDB memory store.
- Relevant memory entries read: `verify-design-against-code-not-just-architecture.md`, `agent-access-block1-design-state.md`, `acceptance-md-locked-no-feature-acs.md`, `physical-erasure-done-defects-only.md`.
- Inspected public [README](../../../../../README.md), [docs home](../../../../../docs/index.md), [contributor guide](../../../../../docs/contributing/index.md), [Node install](../../../../../docs/install/typescript.md), and targeted compatibility/reference/generated-map claims.
- Inspected MCP architecture/design preambles, interface launch/envelope sections, current MCP source flags/rendering, release-design publisher section and Slice 95 completion evidence.
- Source dates: MCP architecture/design/interface declare October 17, 2026 and .27 evaluation scope; visibility-coalescing ADR declares November 8, 2026 and format 37.
- November 9 reconciliation and current .29 baseline supersede earlier format-36/unmerged-.28 snapshots. Historical measurements retain their original dates and scope.
- Transcript evidence here is the bounded secondhand extraction in `planning-evidence.md`; this drafter did not independently read the original chat turns.
- “Complete Slice 90” (`01a1a80a-e240-7e82-9ced-d701d105a3be`): register reports four recent turns read November 9; SDK OIDC repair complete, run42750034084 authentication only, 35 build/publication jobs skipped.
- “Find unfinished 0.8.28 work” (`01a1a8aa-844e-7ed0-84bf-5f6669d85986`): register reports three turns read November 9; format 36 and disk relief describe an older snapshot, superseded by Git/format37/current disk observations.
- Original turn dates are not supplied in that extraction; record them when directly retrieved. Sidebar summaries and missing transcript access cannot establish product behavior.

## Draft needs, requirements and acceptance

| Local ID | Draft obligation | Proposal acceptance evidence |
| --- | --- | --- |
| S6-N1 / S6-R1 | Users can distinguish published instructions from prospective .29 behavior. | S6-A1: every sampled install/schema/release claim has audience, version, source date and current/historical/proposed disposition. |
| S6-N2 / S6-R2 | Library consumers/developers can identify the governing API/default and its implementation maturity. | S6-A2: each discrepancy links contract and source evidence; accepted direction and unimplemented behavior occupy separate fields. |
| S6-N3 / S6-R3 | Contributors can follow checkout/tool/verification instructions without relying on old release assumptions. | S6-A3: setup and generated-doc claims are compared with current manifests/scripts; blockers route to 0/7 rather than invented successful setup. |
| S6-N4 / S6-R4 | Reviewers can assess a correction without erasing history or repeating completed work. | S6-A4: each CRUD proposal has exact claim/location, evidence, replacement/disposition, owner, impact, decision need and downstream slice. |
| S6-N5 / S6-R5 | Transcript-based claims remain bounded and reproducible. | S6-A5: register distinguishes direct reads, shared extraction and unavailable evidence, with turn/date/baseline limits and supersession reason. |

- IDs are release-local drafts; allocate no new canonical AC IDs and do not modify locked contracts.
- A falsifiable pass requires all three audiences represented and every high-impact contradiction either routed or explicitly unresolved; counting files alone cannot pass.

## Slice preparation: architecture reading and audit design

- Read [MCP architecture](../../../../design/mcp-adapter/architecture.md), [MCP design](../../../../design/mcp-adapter/design.md) and [MCP interface](../../../../interfaces/mcp.md) alongside actual crate behavior.
- Read [visibility coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md), [release design](../../../../design/release.md) and [.28 Slice 95 status](../../../0.8.28/features/slice-95/status.md) before proposing “current” wording.
- Draft an evidence graph: claim → audience/document → dated accepted authority → current source/test/receipt → discrepancy → disposition/owner; preserve both historical and current nodes.
- Separate factual correction, historical annotation, contract amendment and documentation of future implementation. Conflicting accepted authorities require Slice 9 resolution, not editorial selection.
- Keep generated documentation subordinate to its generator/manifests; propose input/generator changes and regeneration, never hand edits to generated tables.
- [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) fit: apply traceability and truthful failure evidence; defer new broad assurance campaigns because this slice audits proposals.
- [CUDA review](../../../../design/cuda-version-support-review.md) fit: flag unqualified artifact/support claims; pass usage refresh and policy ratification to 120, without manufacturing adoption shares or reopening erasure enhancements.

## Concrete candidate CRUD and ownership

| Candidate | Proposed operation and evidence boundary | Proposed owner / landing |
| --- | --- | --- |
| User install/release/schema claims | READ/KEEP .28 exact-version examples and format37 claims in README/compatibility; .29 planning alone does not make them stale. UPDATE release promotion only against delivered artifacts. | Documentation implementer / 160 |
| MCP evaluation preambles | UPDATE current entry-point summaries or CREATE a clearly linked current-state note: .27 “SDK as-is” framing is dated historical architecture, while queue/hints and search view flags exist in current source. Preserve original evaluation evidence. | MCP owner with 4/60 / selected 10 preparation |
| MCP output mode | READ current dual structured/text envelope against approved text-default direction; draft the future contract delta without claiming text-default already ships. UPDATE interface/examples together with implementation and parity evidence. | MCP owner / 60 |
| Contributor setup | READ Node25/source manifests versus observed Node26 host and missing owned venv; CREATE bounded prerequisites guidance only if existing setup routes omit necessary checkout ownership. Keep supported product versions separate from active host versions. | Infrastructure/contributor-doc owner / 0/7/10 |
| Publisher recovery and qualification | KEEP current release-design statement that SDK OIDC is configured; DELETE any newly found live “still missing publisher” instruction after inbound-reference review. Preserve failed/assembled .28 runs and auth-only limitations. | Release owner / 7/10/160 |
| Obsolete duplicated instructions | DELETE or archive only after 2 supplies inbound references, replacement destination and historical-retention need; no blanket purge by age or `.28` match. | Cruft owner with documentation owner / 2 then selected 10 |

- These are candidate dispositions, not approved edits or an exhaustive inventory. Record exact file/section and counterevidence before promotion.
- CRUD refers to document lifecycle; exercise API examples against their real contract later without inventing unsupported product verbs.

## Challenges, planned research and fit revision

- Challenge: an old date may document a valid historical experiment; “proposed” headers can coexist with implemented portions. Classify at claim granularity.
- Challenge: .28 publication, a registry smoke, OIDC-only proof and an uninterrupted full gate are different evidence; preserve each scope and failed result.
- Planned primary-source work: inspect remaining audience entry points and linked contracts; use current source, accepted ADRs, manifests, generator inputs and retained qualification receipts as authorities.
- Retrieve bounded original chat pages only when a disputed claim needs them; record title/ID, actual turn range, message dates, read date and commit context. Do not copy credentials or raw transcripts.
- External version/support claims needing refresh belong to 1/120; retrieve official sources under existing egress rules, or retain an explicit evidence gap.
- Fit review compares each proposed edit with 3's contract drafts, 4's actual architecture and 5's test mapping. Revise disposition when current source or a later accepted decision contradicts the earlier reading.
- Produce a short fit-revision log identifying changed claim, new evidence and effect on scope; no fabricated review verdict.

## Dependencies, sequencing and newly discovered prerequisites

- Hard input: current checkout baseline, shared brief/evidence limits and accessible source documents. Research can run alongside 0–5 and 7; no completed-prework dependency is invented.
- Hard downstream: 9 needs the evidence/correction register; 10 may implement only recorded, reviewed preparation decisions. Product-dependent docs wait for their owning feature slice, with final convergence at 160.
- Conditional: deletion/archive proposals require 2's reference/retention analysis; contract deltas require 3/4 reconciliation and applicable owner decision; behavior claims need 5's existing-test evidence.
- Preferred: collect sources first, iterate 3/4/5, then score actionable corrections for 9. Neither full .28 matrix reruns nor new benchmark campaigns gate this proposal.
- Shared-file conflicts: README, MCP interface/designs, contributor docs, generated maps and release design may overlap 2/3/4/7. Assign one integration writer and one owning slice per proposed edit.
- New prerequisites for next reviewer: original transcript message dates/ranges remain unverified; MCP current-versus-evaluation entry-point ownership needs agreement; generated-page updates need source-input ownership.
- Transcript unavailability permits source-based proposals with a stated gap; missing behavior authority blocks the affected asserted correction, not all research.

## Tasks, deliverables and proposal completion

1. Produce `requirements.md` with the local needs/requirements/AC and an audience inventory; mark sampled and uninspected routes explicitly.
2. Produce `evidence-register.md` with dated source/turn provenance, superseded claims, current counterevidence and unresolved gaps.
3. Produce `design.md` with the claim graph, historical-retention rule, generator ownership and authority-conflict handling.
4. Produce `correction-register.md` with concrete before/after wording or keep/archive/delete rationale, owner, risk, decision need and slice allocation; provide example/test links where already verified.
5. Produce `validation-plan.md` linking S6-R/S6-A to review checks and later executable-example verification; provide a scored handoff to 9 without implementing it.
6. Independent read-only review checks all three audiences, source-vs-proposal distinction, transcript limits, historical preservation, ownership and dependency routing; resolve findings or retain them explicitly for 9.

- Completion means the proposal artifacts support a concrete HITL choice, all local acceptance rows have evidence, links resolve and independent review findings are dispositioned. No review has yet occurred.
- Validate planned repository integration with `scripts/agent-lint-md.sh`; later public-doc changes also use `scripts/agent-lint-docs.sh` and applicable generated-doc/MkDocs checks. A scoped pass is never a workspace-green claim.
- Later executable-example or product corrections follow RED → GREEN → refactor in the owning isolated worktree; document-only corrections use evidence and scoped validators, without unrelated product tests.
- Stop for conflicting accepted contracts, missing authority for an asserted correction, or edits that change behavior/support policy; send a bounded decision to 9 and continue unaffected research.
