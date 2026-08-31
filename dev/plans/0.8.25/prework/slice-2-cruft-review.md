---
title: 0.8.25 Slice 2 — repository cruft review
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 2 — repository cruft review

## Outcome

The repository has no tracked build/cache directory that should be removed
immediately. Its principal cruft is stale authority/navigation prose and a
large historical execution corpus that is still valuable but too visible as
active context. This slice proposes narrow classification and pointer fixes;
it does not rename, move, deprecate, archive, or delete anything.

## Proposal register

| ID | Exact surface | Evidence and authority | Proposed action | Risk / destination |
| --- | --- | --- | --- | --- |
| C25-01 | `dev/plans/plan-0.8.25.md` setup record | Still names `/tmp/fathomdb-release-0.8.25`; the durable worktree is `/home/coreyt/projects/fathomdb-worktrees/release-0.8.25` | **Deprecate stale statement in place** by correcting the active plan | Low; Slice 7 documentation hygiene |
| C25-02 | `0.8.23-release-todo.md` at repository root | Declares 0.8.23 unpublished and blocked, contradicting the shipped 0.8.23 state and current 0.8.25 authority | **Archive in place** with a prominent historical/superseded banner and pointer to the 0.8.23 state | Medium misinformation risk; Slice 7 |
| C25-03 | `dev/plans/README.md` 0.8.24 staleness row and `dev/doc-index/plans.md` 0.8.24 rows | Describe 0.8.24 as feature implementation/proposed despite tag `v0.8.24` and current 0.8.25 release state | **Deprecate stale statements in place**; mark 0.8.24 shipped/history | Medium navigation risk; Slice 7 |
| C25-04 | `dev/design/fathomdb-data-plane-architecture-v1.md` and `dev/plans/fathomdb-data-plane-foldback-v1.md` | Both are `ACTIVE`; v2 is the proposed successor and the 0.8.25 plan already points to v2 | **Keep until Slice 6**, then deprecate in place only if v2 is accepted; never delete the reviewed baseline | Low if sequenced; Slice 7 after HITL |
| C25-05 | `dev/doc-index/{design,plans}.md` v1 rows | Call v1 active without explaining proposed-v2 precedence | **Keep for now**; update with the Slice 6 architecture decision | Low; Slice 7 if v2 accepted |
| C25-06 | `dev/plans/runs/**` historical logs and `*-output.json` | 150 tracked `.log` files and 77 output receipts remain, while the plans index says transient logs/outputs were pruned | **Archive in place** as historical evidence; exclude cohorts from active navigation/lint where appropriate. Do not bulk-delete without a separate reference/recoverability proof | High evidence-loss risk; postpone cleanup implementation beyond Slice 7 unless a bounded manifest is approved |
| C25-07 | `experiments/runs/**` | 191 tracked receipt/config/metrics artifacts include repeated preflights and failed witnesses; performance PROGRAM uses them as accepted or negative evidence | **Keep** all registered result receipts. A later evidence manifest may distinguish accepted, rejected, and diagnostic runs; no deletion in 0.8.25 prework | High scientific-record value |
| C25-08 | `dev/performance-benchmarking/**` | Contains plans, preregistrations, decisions, and closeouts for the completed program; several documents retain ephemeral absolute paths | **Archive in place** as completed-program evidence. Preserve paths in historical receipts; correct only active authority links | High evidence value; no Slice 7 bulk rewrite |
| C25-09 | `dev/design/chunking-strategy-and-test-guidance.md` | Active guidance links to vanished `/tmp` worktree files rather than committed experiment receipts | **Deprecate broken citations in place** by replacing them with committed receipt/result paths without changing conclusions | Medium verifiability risk; Slice 7 |
| C25-10 | `.github/dependabot.yml` commentary | Alert counts and the repository URL are stale; functional coverage/pause remains intentional | **Deprecate stale commentary in place**, retain paused configuration unless HITL changes policy | Low functional risk; Slice 7 after authenticated inventory |
| C25-11 | `dev/DOC-INDEX.md`, `dev/doc-index/*.md`, `dev/design/README.md` | Valuable authority maps, but some “current” prose predates release-state selection and v2 planning | **Keep**; make only bounded current-authority corrections, not a wholesale rewrite | Medium churn risk; Slice 7 |
| C25-12 | `dev/progress/**`, shipped plans/prompts, old absolute worktree paths | Explicitly frozen/historical and cross-referenced; paths document execution context | **Archive in place / keep** under existing convention. Do not normalize historical paths | High reference-break risk |
| C25-13 | `scripts/repo-prune/**` and historical prune ledgers | Existing machinery documents why artifacts were retained or pruned | **Keep** as the proper basis for any future evidence-prune campaign | Low |
| C25-14 | `src/**` compatibility and legacy branches found by textual scan | Most are intentional schema/recovery behavior (`_legacy:` cohort, ABI compatibility, accepted error forms), not dead code | **Keep** unless an owning feature slice proves a branch unreachable with tests. No repo-wide deletion proposal | High product/data risk |
| C25-15 | `scripts/tests/fixtures/dorny-paths-filter-v4/dist/index.js` | Only tracked `dist/` path; it is the committed runtime payload for a workflow fixture | **Keep**; it is source fixture evidence, not build output | Low |
| C25-16 | Untracked ignored `experiments/__pycache__` and shared `.venv`, `node_modules`, `data` symlinks | Correctly ignored; Slice 0 establishes that shared links are planning-only | **Keep ignored locally**; do not commit or use as release proof | Low; environment discipline |

## Category conclusions

- **Program and release records:** retain as historical evidence; repair only
  statements that still claim current authority.
- **Architecture and design:** preserve accepted ADRs and as-built topic docs.
  The v1-to-v2 transition needs one explicit Slice 6 decision, not duplicate
  active architectures.
- **Source and tests:** no deletion is justified by textual “legacy” matches.
  Compatibility branches protect real old-schema and cross-platform behavior.
- **Developer artifacts:** historical logs are expensive context but may carry
  unique review evidence. Any deletion requires a generated inbound-reference
  and retained-result manifest first.
- **Public docs:** no direct public-doc contradiction was found in this pass;
  feature-specific public changes remain with Slices 15–75.

## Proposed Slice 7 scope

Only C25-01, C25-02, C25-03, C25-09, C25-10, and the Slice 6-selected v1/v2
authority pointer are small enough for prework implementation. The historical
logs/results corpus is deliberately not a Slice 7 cleanup project.

## Evidence

- 3,336 tracked files: 1,712 under `dev/`, 583 under `src/`, 383 under
  `scripts/`, 286 under `experiments/`, and 285 under `tests/`.
- Tracked-output, absolute-path, TODO/deprecation, current-authority, inbound
  reference, symlink, and ignored-cache scans.
- `dev/DOC-INDEX.md`, scoped doc indexes, release-state authority, the
  historical archive convention, and current 0.8.25 plan.
