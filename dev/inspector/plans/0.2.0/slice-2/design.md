---
title: Inspector 0.2.0 Slice 2 — classification design
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: DRAFT
desc: Evidence, disposition and approval boundaries for a read-only cruft proposal
---

# Slice 2 — high-level draft design

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Intent and source fit

Produce a repo-wide evidence-backed disposition proposal under the [plan](plan.md) and [adopted scope](../scope.md). Planning inspection used `llm/0.8.29-inspector-additional`, proposal `829625818`, unchanged Engine `6edc1155f`, package 0.8.28/format 37, on 2026-11-19 while adoption edits were in flight. Reconcile actual revision and intervening changes at execution before accepting this draft.

Existing mechanisms: Inspector has [canonical document owners](../../../requirements.md#document-ownership), [HLD](../../../design/high-level-design.md), legacy slice statuses and a single verification matrix. Engine has [design lifecycle ownership](../../../../design/README.md), `dev/design/document-lifecycle.json`, ADR authority, document indices and archive conventions. A new audit database, retention framework or competing source-of-truth catalog is unnecessary. Net-new work is a bounded inventory/proposal, not a product component or runtime route.

The historical S16 architecture and private Inspector worker source were inspected: existing record operations do not include future text/vector/graph features; fixture-control is explicitly test-gated. Public Engine API exposure, private helpers and test-only writers cannot be conflated when evaluating dead-code claims. Engine/shared code is read-only evidence and any proposed remedy needs separate ownership/commission.

## Corpus and minimum evidence model

Enumerate every tracked path once and account for untracked/ignored/external material as exclusions with owner/reason, without traversing external targets or exposing secrets. Coverage groups span program/dev needs/requirements/AC/traceability, architecture/ADRs/interfaces, implementation/tests/fixtures/tools, design, release plans/status/evidence, notes/intermediate/research, public docs/navigation, and root/build/CI/license/config. A directory may inherit keep status only when exceptions and coverage counts remain explicit; non-keep proposals name exact paths.

Use the smallest useful proposal table rather than per-file receipts. Each non-keep row records:

| Field | Purpose |
| --- | --- |
| Finding key; revision/path/section | Stable evidence address; one key may link both 2 and 6. |
| Product/subsystem owner; current lifecycle/authority | Distinguish Inspector, Engine/shared, program, third-party and unknown ownership. |
| Current claim/use and unique retained value | Separate stale factual claim, duplicate content, intentional redirect, historical evidence and reproducibility input. |
| References/consumers and search scope | Inbound links/anchors, redirects, navigation, code reads versus writes, module/test/build/package/CI use, unresolved external links. |
| Proposed disposition; successor/target | One of four dispositions; archive target only after rules permit it. Never overwrite history. |
| Retention/rights; risk/effort/dependencies | License/attribution, fixture/source rights, record/evidence value, legal/owner uncertainty and exact future edits/checks. |
| Approval state and handoff | Proposed only; Slice 9 decision and conditional Slice 10 row, or external commission. |

Do not collect hashes, inventories of secrets, execution diaries or full scan logs by default. Record enough revision/command/evidence detail to reproduce a finding. Use existing re-runnable tools; any new executable needs separate approval, TDD and maintained ownership.

## Decision rules

| Disposition | Required evidence and constraints |
| --- | --- |
| **Keep** | Current authority, active consumer, useful redirect, unique rationale, historical decision/status, reproducibility input, required tool/oracle, or uncertain retention/ownership. Historical keep may be archived in place already. |
| **Deprecate-in-place** | Material risks being mistaken for current guidance, an exact successor/current pointer exists, and the proposed limited notice preserves original content and linked anchors. This is a documentation lifecycle proposal, not API compatibility shims or product deprecation. |
| **Archive** | Retired non-current material retains historical value, local lifecycle rules allow relocation, and explicit target/reference/manifest/catalog changes preserve discoverability. Completed release plans/prompts/run evidence default to in-place archival; a move conflicts with that rule and needs an explicit human exception. |
| **Delete** | Exact material is genuinely disposable/redundant with no unique decision/evidence/oracle/reader/rights/retention duty; all known dependencies and unresolved external-use risks are addressed. Filename/age, small audience, no text hits or Git recoverability alone are insufficient. No current file is designated deletable by this draft. |

Unknown ownership, rights, legal retention or external consumer → keep pending a named question, not a guessed destructive recommendation. Do not invent retention obligations or use a general legal warning to stall unrelated classification. Preserve licenses/attribution, approved ADR history, frozen global registers, append-only ledgers, generated-state authority, RED/GREEN, status and reproducible gate evidence. Catalog/ledger changes, if later approved, use their existing mechanisms; do not hand-edit ledgers or generated regions.

The archive README's restored `expb-joint-tune-output.json` and `gate2-oracle-output.json` show why a raw-output filename cannot establish disposability: tracked generator code READS them. Preserve `src/inspector/tools/` measurement tools, examples and required fixtures until their actual consumers and replacements are established. Ignored long/native tests represent outstanding coverage, not unused assets.

## Shared evidence and approval flow

Slice 2 owns corpus coverage and disposition. Slice 6 owns stale-doc claims and remedies for users, consuming developers and contributors. Join on revision/path/claim key, link the original evidence, attach separate disposition and text-remedy fields, assign one owner and count effort once. For example, legacy 0.1 allocation wording may need a current navigation pointer while the old plan/status remains valid historical evidence. Shared evidence never transfers canonical ownership.

Slices 3/4/5 own canonical scope/architecture/test-adequacy review. Local D2-N/R/A/D in the plan protect UN-08/09, INS-01–09 and all carried INS/V01/X/F/T obligations without adding canonical needs or acceptance IDs. The proposal does not qualify HLD feasibility, erase missing positives, or reopen settled physical-erasure policy.

Flow: inventory → evidence and classification → coherent Sol-high design/proposal review → Terra scope/reference verification → Slice 9 human decision → individually approved conditional Slice 10 implementation. A completed review does not advance an item to approved. Include-vs-postpone and at most four FIX-n refinement cycles belong to 9; no elapsed-time approval or predetermined cleanup bundle.

## Feasibility, questions and prerequisites for the next reviewer

Static read-only inspection is feasible with existing `rg`, Git path/history queries and catalogs. Dynamic references and external links cannot be disproved by grep; remain explicit uncertainty. If needed during execution, research only the relevant official MkDocs/Git/package documentation or original license terms. No broad web research, installs, runtime tests, code sweeping or cleanup is performed now.

Before execution/review: confirm Slice 0 created and entered the release checkout under the owner decision, consume final 0/1 outputs and latest scope/master/workflow, enumerate changes since drafting and keep/adjust/reject/already-done with reasons, agree shared finding keys with 6, and confirm inventory boundaries/owners. Re-read [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): retain independent lexical/numeric/adjacency oracles, property/resource/fault/cancel fixtures and browser/keyboard/human tasks; defer unapproved engine fuzzing/MC/DC campaigns. CPU-only Inspector scope changes no GPU/driver/support policy; Slice 0 owns its formal review.

Verify coverage and all destructive proposals plus representative keep groups; inspect source readers and links, run document-scoped gates and report their actual results. Missing mandatory evidence cannot become an unavailable pass. Prose-only deliverables need no artificial RED/GREEN or full runtime gate. Create status only during future execution. Preserve the selected report checkout and unrelated work; clean only separately owned extra isolation after verified integration, with no hooks bypass, force push, tag or publication.
