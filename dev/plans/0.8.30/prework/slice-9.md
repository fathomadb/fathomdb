---
title: Slice 9 scored proposals and interactive decisions
status: ACTIVE
target_release: 0.8.30
slice: 9
date: 2026-11-18
---

# Slice 9 scored proposals and interactive decisions

Interactive review is in progress. These are recommendations, not HITL rulings.
No proposal below is approved by silence. Source basis is the
[completed prework](README.md), reviewed [master](../plan.md) and subsequent
mainline reconciliation `b86b19847852e9beaeea32b98dae03b537d8fea4`.
The .30 execution state has not been activated; these local proposal IDs are
not canonical ACs or a replacement for the published .29 decision arrays.

## Score meanings

- Understanding:3 = mechanism/scope supported by specific source or execution
  evidence;2 = concrete lead with material design/causal uncertainty;1 = weakly
  bounded. It is not a priority score or certainty that a product bug exists.
- Risk:L = contained/reversible;M = cross-component behavior or regression
  exposure;H = native/runtime/API/support or major integration change.
- Effort:S = focused changes;M = several coordinated changes and scoped proof;
  L = substantial implementation, migration or multi-platform qualification.
  These are relative planning estimates, not measured person-days.

## Proposal register

All new choices have status PENDING. Include means in .30's selected scope,
with stated prerequisites; it does not authorize publication, live resets or
new external venues. Each eventual ruling will reference its ledger entry.

| ID | Proposal / receiving owner | U | Risk | Effort | Recommendation and reason |
| --- | --- | --- | --- | --- | --- |
| P9-01 | Owned Python3.12 setup, Rust1.95 baseline, deliberate project npm pin and existing compute;10 |3 | L | S | Include. Removes demonstrated setup ambiguity without expanding platforms or provisioning new hosts. Node choice is separate below. |
| P9-02 | Narrow Markdown parser, TLS root bundle and Tokio advisory fixes;10 |3 | M | M | Include all three. Patch the recorded affected resolutions with separately reviewable lock changes and affected controls. Preserve native/model pins and inspect reachability/compatibility rather than claiming exploitability. |
| P9-03 | Coordinated Node26LTS candidate and compatible npm/package/toolchain/support qualification;10/70 |2 | H | L | Include migration and qualification. Prework identifies Node25 EOL; avoid extending an unsupported baseline by default. Exact versions and support claims change only after the selected native/package proof. |
| P9-04 | ONNX exporter security cohort, including certifi and major Transformers/build dependencies;10 |2 | H | L | Triage reachable advisories now; postpone a broad exporter/model re-export migration. Permit a narrowly selected fix only with compatibility and accepted output/identity proof. A necessary model-byte change needs an explicit separate decision. |
| P9-05 | Mermaid/rendering advisory graph and Puppeteer/Zod major changes;10 |2 | H | M | Resolve necessary advisory exposure with a supported graph; postpone broad browser/parser majors without a compatibility reason. DOMPurify's reported PR fix is already present at the prework source and must not be applied twice. |
| P9-06 | Other version-only Rust/Python/TS/native binding and Action upgrades;10/60 |2 | M | M | Postpone general refresh. Include only a required security, compatibility or consumed delivery fix. Preserve immutable Action/source pins and qualify each selected delta. Existing fork allocation is separate. |
| P9-07 | Targeted stale user/developer/contributor guidance and authority pointers;10/owning feature |3 | L | S | Include. Correct implemented record identity, nullable hit provenance, owned setup and long/ptrace guidance. Preserve historical evidence; do not repeat .29 metadata repairs already completed upstream. |
| P9-08 | Local needs/requirements/acceptance and architecture refinements from prework3;10/20–80 |3 | M | M | Include the release-local refinements and allocations. Detailed API/design decisions remain in their owning reviewed design; accepted ADRs and canonical ACs are not rewritten wholesale. |
| P9-09 | CI failure retention, truthful terminal-state checks, normal OIDC/visibility, exact-tag admission and persisted registry-consumer flows;20 |3 | M | L | Include. These address actual failures or confirmed missing proof. Keep existing security, trust and real platform controls, and separate source/candidate/registry execution. |
| P9-10 | Measured Windows static-check, tool-only bootstrap and duplicate-build reductions;20 |2 | M | M | Include bounded optimization of the measured costs. Preserve the actual wrong-workflow/runtime oracles; no general CI framework rewrite. |
| P9-11 | Two bounded hunts with remediation plus selected state/property/reference/fault and resource oracles;30/50, accounted for70 |2 | M | L | Include. Cover saved S50 attribution, record/coverage/error composition, capacity/close/reopen and default Node growth. No bug quota or speculative rewrite. Retire/replace the trivial query scaffold without duplicating existing in-crate properties. |
| P9-12 | Current latency attribution and one demonstrated bottleneck treatment;40 |2 | M | L | Include. Use matched/interleaved inputs, actual useful tails/throughput/errors/resources and fixed-state correctness. Neutral/inconclusive results do not justify adoption; no cache/backend/schema expansion by inference. |
| P9-13 | Broad cruft deletion, physical archive moves, generated-map removal or structural refactor;10/feature owners |2 | M | M | Postpone broad work. Retain linked history and unique evidence. Small adjacent factual fixes are P9-07, not permission for bulk deletion or size-driven extraction. |
| P9-14 | Integrate the separately governed Inspector branch into database .30 |2 | H | L | Keep separate for now. It adds its own product, transport, packaging and pending qualification. Include only if the owner explicitly selects a concrete integration scope/owner. Branch naming is not integration authority. |

Evidence: [environment](slice-0.md), [library/PR/advisory inventory](slice-1.md),
[cruft](slice-2.md), [draft contracts](slice-3.md), [architecture/Inspector](slice-4.md),
[test adequacy](slice-5.md), [documentation](slice-6.md), [CI/delivery](slice-7.md).
P9-04/05 triage and P9-06 exceptions preserve security response without treating
all available major versions as required upgrades.

## Already assigned or required, not being reopened

- Fork B/C placement in .30 is already ruled at seq-367. Keep F30-REBASE,
  MARKER and MULTIARCH in60; selected upstream, actual target set and final
  pins still need their design/qualification decisions. Do not ask again
  whether to place that work in .30. [Allocation](../fork-maintenance-plan.md).
- Precise feedback review, reset review and S50 investigation are already
  assigned by seq-375/376/377/393. Preserve those scopes and their limits.
  Actual reset execution still needs concrete venue/session/resource authority.
  [Receiving allocations](../release-followups.md).
- B30-01 module-model release retains its existing .30 placement and safe-drop
  prerequisite. A blocked prerequisite holds the unsafe path, not independent
  safe work. [Roadmap](../../../ROADMAP.md#0830-backlog).
- Real source/security/long/platform/artifact qualification and separately
  authorized registry consumers remain required70/80 work, not optional votes.
- Latest main `b86b19847` closes the .29 supplemental Tegra public delivery and
  updates the stale publication-capability/archive metadata. Those prework
  observations are now resolved upstream. The all-LANDED checker/preflight
  assumptions remain .30 tooling work. Do not repeat successful .29 delivery.
- No global MC/DC, exhaustive VFS/OOM, new paid/model campaign, CUDA13 delivery,
  ANN/chunking, host reset, driver/primary-toolkit change or new platform claim
  is added. Previously ruled allocation is not blanket run authority.

## Interactive sequence and pending rulings

1. Security and basic preparation: P9-02, then P9-01/P9-07.
2. Runtime and dependency breadth: P9-03–06.
3. Engineering scope and test/performance limits: P9-08–13.
4. Inspector integration and any remaining consequential choices: P9-14.

Start with P9-02. Options are include all three recommended fixes; include only
TLS/Tokio and postpone the Markdown tool update; or postpone all three.
Including all three addresses the recorded advisory set in one bounded
preparation phase. Restricting the scope saves tooling qualification but leaves
that recorded parser issue outstanding. Reassess a particular fix if the exact
resolved feature/API path or candidate compatibility contradicts prework.
These choices block the affected10 work only; scoring and independent proposals
can proceed. No irreversible external operation is behind this decision.

**HITL answers recorded: none yet.** Subsequent actual rulings are recorded via
ledgerwrite, with proposal rows referencing the resulting seq and no invented
approval. Preserve the current release-state ownership; do not write .30
answers into the closed .29 ladder. Once scope rulings are collected, draft10,
obtain the requested subagent review with up to four FIX-n cycles, present the
result interactively, and capture the final owner decision before implementation.
