---
title: Inspector 0.2.0 repo-wide cruft classification
date: 2026-11-19
status: REVIEW
---

# Corpus classification and proposals

Executed inventory at planning source `aef5b2eac19dbcf5850713081ccd427896909eea`
after reading Slice 2 plan/design/filled prompt, 0/1 outputs, lifecycle/archive
rules and owner/index documents. Since draft `829625818`, Inspector scope and
0.2.0 plans/navigation were adopted; retain scope/retention rules, adjust draft
source/executor and do not repeat already-updated specification redirect.

## Coverage and disposition

`git ls-files -z` enumerated **14,097 tracked paths**, partitioned once by
first matching prefix below. No missing tracked regular paths; 21 symlinks
were inventoried as links without traversing their targets for classification.
These links are retained benchmark provenance under the Slice 135 pilot.
Untracked output consists of this commission's reports; ignored developer
caches/build environments, external corpus/symlink targets and secrets are
excluded from cleanup authority. No orphaned checkout, ignored data or other
session artifact was proposed disposable.

| Exclusive group | Count | Disposition / checked ownership or sample |
| --- | ---: | --- |
| dev/plans | 9,614 | Keep in place: release selectors, boards, prompts, execution evidence; archive convention and raw-input generator checked. No content-wide correctness audit claimed. |
| dev/inspector | 83 | Keep; canonical contracts, HLD, matrix, historical statuses/reviews and current plans. Exact current-pointer proposals below. |
| dev/adr / dev/interfaces | 96 / 8 | Keep accepted/superseded decisions and public contracts; ADR index checked. |
| dev/design | 241 | Keep lifecycle-catalog-owned current/reference/experiment/historical documents. Architecture v2 supersedes retained architecture snapshot. |
| dev/notes / dev/archive / dev/progress | 40 / 56 / 5 | Keep supporting/historical value, archive correction and frozen progress policy. No age-based deletion. |
| Remaining dev | 599 | Keep pending owning subsystem review: corpus/research/prototypes/performance/tools/ledgers/needs/requirements/acceptance/traceability. Unique authority and retained reproducibility take priority over suspicion. |
| src/inspector / src/rust / src/python / src/ts / other src | 79 / 673 / 260 / 138 / 7 | Keep implementation/tests/fixtures/public bindings/conformance. Manifests, worker and tool imports checked; test-only/ignored routes do not prove dead code. |
| scripts / tests / test | 1,069 / 329 / 5 | Keep dev-loop, CI, security, packaging, fixture and independent test inputs. Native/long cases remain required evidence, not unused assets. |
| docs | 45 | Keep public guides/navigation; current Inspector entry distinguishes adopted scope from unpublished baseline. 6 owns factual wording review. |
| experiments | 686 | Keep external program ownership and measurement/replay policy; no separate experiment cleanup commissioned. |
| data / tools | 4 / 5 | Keep corpus/rights declarations, tracked tooling and docs requirements. External corpus contents excluded. |
| .github / .claude / .codex | 13 / 10 / 6 | Keep CI/security/action controls and tracked agent/skill entry points; local settings excluded. |
| Root files | 26 | Keep manifests/locks/licenses/build/config/AGENTS and indices. |
| **Total** | **14,097** | Every tracked path classified; keep includes explicit uncertain/external ownership, not an assertion every file is current. |

## Exact candidate dispositions, shared with Slice 6

| Key / paths at source HEAD | Observed use/value; proposed disposition | Owner, dependencies, risk/effort |
| --- | --- | --- |
| D2-D6-NAV: dev/inspector/design/README.md; dev/inspector/reviews/README.md; requirements.md execution-allocation section | Design index still sends feature drafts only to plans/0.1; reviews index remains target_release 0.1 and links old plan; requirements allocation still names old 20–60 ladder. Keep historical links/content; update current navigation/label in place to plans/0.2.0/plan.md, preserving historical scope and matrix. Existing source/docs links consume these owner pages; no anchors/moves needed. | Inspector docs; 6 exact text remedy, 3 ownership reconciliation; low/small, UNDECIDED. |
| D2-D6-HLD: dev/inspector/design/high-level-design.md lexical evidence and Q model route | Maintained HLD includes stale missing-snippet/current Caller assumptions. Keep architecture/accepted historical decisions; replace only current factual guidance after 4 source qualification. Detailed text owned by 4/6, counted once. | Inspector architecture; 4 prerequisite, medium/small, UNDECIDED. |
| KEEP-REDIRECT: dev/inspector/needs-requirements-acceptance.md | Already current 0.2.0 scope pointer and separate canonical owners; explicitly preserves incoming links. Keep unchanged, not duplicate cruft. | Inspector, already done. |
| KEEP-INPUTS: dev/plans/runs/expb-joint-tune-output.json; gate2-oracle-output.json | build_registry.py reads both to regenerate registry; archive README documents prior incorrect deletion. Keep exact paths and evidence. | External prototype/program, no proposal. |
| KEEP-HISTORY: dev/inspector/plans/plan-0.1.md and 0.1 slice statuses/designs; dev/architecture.md | Historical plan/status and superseded architecture preserve decisions/test provenance. Current adoption does not turn old ACTIVE work complete. Keep/archive in place under existing rules, no relocation. | Inspector history / external architecture owner. |

Repository-wide reference searches inspected basenames/path aliases for these
candidates and actual generator reads rather than inferring from filenames.
Dynamic/external consumers remain uncertain; that uncertainty supports keep.
No archive relocation or deletion is justified by the evidence, so there is no
destructive row to approve. Preserve license/attribution and retained fixtures;
no invented legal hold or expiry. 6 reuses D2-D6 keys; 9 counts effort once.

No files were deleted, moved, deprecated or rewritten. Canonical registers,
ledgers/generated regions/source and runtime remain unchanged. Independent classification review and document-check limits are in [the shared record](../slice-9/prework-verification.md).
