---
title: FathomDB 0.8.27 prework Slice 2 - repository and documentation cruft
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 2 - repository and documentation cruft

## Plan and delta reconciliation

Outcome: classify relevant repository and documentation material as keep,
correct/deprecate, archive-in-place, or delete-after-proof. No file is moved,
renamed, archived, or deleted in prework. The draft's broad cleanup idea is
narrowed to items that improve the correction/refactor work directly.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-2A | Authoritative contracts and historical evidence retain stable paths. | Accepted/superseded ADRs, interfaces, release states/boards, frozen progress, and prior review receipts remain. |
| PW27-2B | Stale generated geometry is not used as implementation authority. | Old maps are snapshot-labelled; any deletion follows inbound-reader and unique-evidence checks. |
| PW27-2C | Cleanup cannot absorb unrelated repository history. | Slice 10 receives only approved exact candidates; bulk extension/path deletion is rejected. |

## Disposition

Keep:

- accepted/superseded ADRs, interfaces, needs/requirements/acceptance records,
  release-state/boards/statuses, 0.8.25/0.8.26 review receipts, and frozen
  `dev/progress/` history;
- the concise readability proposal, refactor proposals, engine-decomposition
  rationale, horizontal reconciliation, scanner methodology/thresholds, and
  append-only ledger; and
- experiment findings as evidence, never as branches to merge wholesale.

Correct or deprecate in place:

- the 0.8.27/0.8.28 draft scopes, roadmap, program sequencing, and plan index
  after Slice 9 rules every F27/D27 placement;
- the `ff4f07a0` line/range anchors in the readability/refactor records, which
  remain historical snapshots and must point to a current baseline before use;
- the stale `dev/platform-capabilities.json` release marker and any maintained
  text that still calls 0.8.26 active; and
- the old 0.9.0 placement if Slice 9 explicitly moves the accepted work.

Archive in place:

- completed experiment phase/commission/status records and prior-release
  corpora whose stable links matter.

Delete candidates, only after reader/reference/unique-content proof:

- the 113 reproducible `dev/plans/refactor-background-check/structure/*.txt`
  maps and reproducible `violations.{json,md}`;
- `vertical-review.json` and `horizontal-review.json` only if every unique
  conclusion is proven distilled (otherwise archive); and
- the duplicate long-file decomposition text only if byte/semantic comparison
  proves the Markdown owner supersedes it.

Current scale makes bulk cleanup especially unsafe: 4,641 tracked files, 2,640
under `dev/`, 1,915 under `dev/plans/`, 320 logs, and 941 JSON files. Reject
extension-based deletion, wholesale experiment-branch merges, re-splitting the
59 settled LEAVEs, or a line-count gate.

## Implementation, review, verification, and status

Implementation is this disposition record only. TDD and code review are not
applicable. An independent read-only inventory checked tracked paths, sizes,
status, and experiment ancestry. Independent package design review and
closeout verification passed.

Status is `COMPLETE`. No repository artifact was cleaned up. Next: Slice 3
contract drafting.
