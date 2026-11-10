# Slice 10 decision-bound preparation requirements

Status: scope-approved subset from Slice9 batches1/2; final reviewed acceptance
and unresolved checkpoint intake dispositions remain pending. No preparation has
executed. This refines the original [plan](plan.md), not a product-green claim.

| Local requirement | Actual owner ruling / acceptance |
| --- | --- |
| S10-R1 owned execution | Batch1 approves task-owned /mnt/data build/scratch, Node25.9.0/npm11.12.1, owned Python/native environments. Missing/wrong tools fail before workload; exact interpreter/source/features/native identity documented. |
| S10-R2 contained dependencies | Batch1 approves current-lock Rustls.23.46 and Mermaid DOMPurify3.4.17, subject to affected compatibility/security checks. Preserve current qualified dependency family and model identity; no stale lock replacement. |
| S10-R3 state and strict preflight | Batch2 approves .29 state/board activation; state/views agree before strict dependent worktree preflight. Existing baseline6edc1155f includes completed .28. |
| S10-R4 bounded authority/docs | Batch2 approves identified stale applicability/workspace/package docs. Correct current claims, preserve locked IDs/thresholds/history/generator ownership. |
| S10-R5 complete survey scope | Batch2 approves exporter discovery and explicit tiering. Account for all recognized manifests; missing/untiered cases are observable failures or explicit classifications. |
| S10-R6 narrow first-tag route | Batch2 approves narrowly scoped .29 routing change preserving protections. Wrong tag/API failure/missing workflow/reviewer controls refuse readiness; external access changes remain separate. |
| S10-R7 preserve owner exclusions | Owner keeps query scaffold. Preserve other checkouts/data/standing GPU install; no cleanup, driver/global toolkit change, unpausing, package-major/fork migration or publication by inference. |
| S10-R8 existing database format | Owner seq-368: no database migration is required for .29. Retain existing format37; package W/version and release-state changes are metadata, not database migration. |

Axis W→0.8.29 coordination from the original plan is included in the final
preparation acceptance proposal; independent E remains0.8.0. Explicitly confirm
this receiving task with final package acceptance before changing manifests.
Evidence and exact rulings: [decision log](../../prework/slice-9/decision-log.md).
