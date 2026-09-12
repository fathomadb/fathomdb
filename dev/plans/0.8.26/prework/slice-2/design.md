---
title: 0.8.26 Slice 2 — cruft review draft notes
status: COMPLETE
---

# Slice 2 draft design notes

## Initial proposal seeds

| ID | Surface | Initial action hypothesis | Evidence required before Slice 8 |
| --- | --- | --- | --- |
| C26-01 | `dev/plans/0.8.26-draft-scope.md` | Deprecate-in-place after accepted 0.8.26 plan supersedes it | Every D26 item has a durable disposition. |
| C26-02 | Completed `dev/plans/0.8.25/**` execution corpus | Keep/archive-in-place with clearer historical navigation | Preserve reviews, receipts, and experiment evidence; measure active-index noise. |
| C26-03 | Multiple historical `dev/plans/release-state-*.json` files versus the documented single-writer wording | Keep records but correct authority/navigation model | Identify generator behavior and which file, if any, is active. |
| C26-04 | `dev/progress/` | Archive-in-place/keep frozen | Confirm no active writer and retain history. |
| C26-05 | Public-doc truth checker expecting 0.8.23 after 0.8.25 publication | Update test/fixture, not product truth | Reproduce from clean release worktree and trace expected-version source. |
| C26-06 | Post-tag 0.8.25 documentation commits | Keep; clarify tag/site chronology if confusing | Confirm current public site and release-note authority. |
| C26-07 | Old graph/evidence designs superseded by accepted 0.8.25 contracts | Deprecate-in-place with successor pointers | Verify all current inbound links and unique rationale. |
| C26-08 | Generated build, package, cache, corpus, and model artifacts | Delete only if tracked accidentally and reproducible | Prove ownership and no unique evidence. |

## Review method

Use `rg --files`, tracked-file classification, inbound-reference searches,
public index comparison, test target enumeration, and source-symbol reachability.
Do not use modification time as authority and do not bulk-move historical
records. Final Slice 2 output expands this seed table to the full repository.

## Completed domain disposition

The census contains 4,435 tracked files, including 2,503 under `dev/`, 1,814
under `dev/plans/`, 935 run records, 320 tracked logs, and 921 JSON files.
Volume alone is not evidence of cruft.

| Domain | Proposal | Reason and guard |
| --- | --- | --- |
| accepted and superseded ADRs; global needs, requirements, acceptance criteria, architecture, interfaces, tests, and traceability | keep | Authority and historical rationale remain load-bearing. |
| product source, migrations, scripts, fixtures, public examples, and package manifests | keep | This pass found no reachability evidence supporting deletion. Correctness updates belong to later slices. |
| 0.8.25 plans, reviews, receipts, run evidence, and preserved probe executable | archive-in-place | Preserve stable links and release evidence. The probe has an explicit preservation receipt. |
| historical release-state files, boards, `dev/progress`, `dev/archive`, and experiment/performance evidence | keep/archive-in-place | They are historical inputs, not competing live authorities. Improve navigation instead of deleting evidence. |
| maintained current-release pointers, `dev/platform-capabilities.json`, `src/ts/README.md`, and superseded designs without successor links | deprecate-in-place/update | Correct stale authority, platform, and successor navigation after Slice 8 approves exact edits. |
| `dev/plans/0.8.26-draft-scope.md` | deprecate-in-place later | Only after every D26 item has a durable disposition in the accepted plan. |
| 318 tracked run logs and duplicate machine outputs | deletion candidate only | First distill unique Slice 6/7 evidence and prove no inbound reader or historical obligation. No bulk extension-based deletion. |
| `.prettierrc.json` and `.prettierignore` | deletion candidate only | Reconcile the stated toolchain contract and prove there is no active consumer before removal. |

The 0.8.5 cleanup map remains useful methodology but is not a current census.
Every individual deletion requires an inbound-reference and reader check in
the implementation slice. This review authorizes no rename, move, or delete.

## Finding register

| ID | Exact target or reproducible pattern | References/authority/evidence | Risk | Proposal |
| --- | --- | --- | --- | --- |
| C26-01 | `dev/plans/0.8.26-draft-scope.md` | Indexed intake; D26 items remain allocation inputs | medium: premature deprecation can drop scope | deprecate-in-place only after complete disposition |
| C26-02 | `dev/plans/0.8.25/**`, `dev/plans/runs/STATUS-0.8.25.md`, and `dev/plans/runs/0.8.25-slice-*/**` | release plan/status links and unique execution receipts | high historical/audit value | archive-in-place/keep |
| C26-03 | `dev/plans/release-state-0.8.{20,21,22,23,25}.json` and matching boards | release selector/checker inputs and historical authority | high: deletion breaks state views | keep; correct selector documentation |
| C26-04 | `dev/progress/**`, `dev/archive/**`, `dev/experiments-ledger.md`, performance baselines | explicitly frozen/history and machine readers | high | keep/archive-in-place |
| C26-05 | `scripts/check-public-doc-truth.py`, its tests, and 0.8.25 state/board | current verifier and release lifecycle authority; source finding S6-02 | high correctness | Slice 8 places update; never delete |
| C26-06 | commits `983bec68`, `9ef87489`, `a642e391`, `a563362d` and their maintained public/release-plan paths, reproducible with `git log v0.8.25..a563362d -- README.md docs/ dev/plans/` | current published truth and chronology | high | keep/no action; clarify chronology only if needed |
| C26-07 | superseded `dev/design/*.md` with accepted successor ADR/design links | inbound historical links; unique rationale not copied forward | medium | deprecate-in-place with verified successor pointers |
| C26-08 | tracked `dev/plans/runs/**/*.log` group (320 total tracked logs; 318 candidate run logs) | S6/S7 source evidence and possible script readers | high until distilled | candidate-delete only after per-file reader/reference/value proof |
| C26-09 | `dev/plans/runs/0.8.25-slice-71/71b/attribution/build/slice71b-probe` | adjacent explicit preservation receipt; unique executable witness | high | keep |
| C26-10 | `.prettierrc.json` and `.prettierignore` | no active consumer found; instructions/manifests disagree about retained Prettier | medium toolchain-doc drift | candidate-delete after contract reconciliation and `rg` proof |
| C26-11 | `dev/platform-capabilities.json`, `src/ts/README.md`, `dev/plans/README.md`, current-authority prose in `AGENTS.md` | maintained indexes/public developer guidance | high stale-truth risk | update/deprecate-in-place through owning slice |
| C26-12 | `src/**`, `scripts/**`, migrations, tests, fixtures, examples, package manifests | compiled/tested or published surfaces; no unreachable target proven | high product risk | keep/no finding |

The patterns above are reproducible with `git ls-files` and `rg`; future
candidate deletion must enumerate the concrete expansion at execution time so
the proof cannot go stale between review and action.
