---
title: FathomDB 0.8.27 prework Slice 6 - stale documentation evidence
status: ACTIVE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 6 - stale documentation evidence

## Plan and delta reconciliation

Outcome: identify stale user, SDK, developer, contributor, architecture, and
refactor evidence and allocate exact corrections without editing those owning
documents. The draft is narrowed away from wholesale cleanup: historical
records remain historical, and only maintained/current truth is corrected.

Changes since the draft intake are material: 0.8.26 is published, schema 34 and
the fresh-database boundary are current, the five target files have grown to
34,184/5,943/5,406/4,439/2,846 lines, and the large-file pilot branches contain
real extraction/routing/disk lessons. The old `ff4f07a0` inventory is therefore
a snapshot, not an executable map.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-6A | Maintained public entry points report exact released 0.8.26 facts. | Public-doc truth and MkDocs checks cover the root/site/install/API entry points and pass after the allocated correction. |
| PW27-6B | Current design navigation names one authority and avoids silently stale mutable line citations. | Owner/lifecycle checks pass; current records use symbol, test, ADR, or commit-bound anchors. |
| PW27-6C | Historical refactor evidence remains useful without masquerading as current instructions. | Old line maps are snapshot-labelled; reproducible generated output is removed only after inbound-reader and unique-content checks. |

## Findings and dispositions

Update in Slice 10:

- `README.md`, `src/ts/README.md`, the public docs landing/getting-started,
  concepts, operations, reference, embedder, compatibility, and install pages;
- add the 0.8.26 release note while preserving historical release notes;
- `dev/plans/README.md`, `dev/plans/prompts/README.md`, and the maintained
  document indexes; and
- broaden `scripts/check-public-doc-truth.py` only enough to cover current
  public entry points with focused fixtures.

Current truth is: release 0.8.26, schema 34, fresh-database boundary, five
published native CPU targets, npm `latest` and `next` at 0.8.26, and a published
Rust 0.8.26 package. The root public-document truth check currently fails
because the README still presents 0.8.25.

Keep the existing current-owner authority (`document-lifecycle.json`,
`current-owner-authority.json`, and its checker); do not create another
ownership system. Update maintained design citations such as the displaced
schema/`OpenReport` links in `dev/design/embedder.md` and the incorrect engine
line in `dev/design/errors.md` when their owning slices touch them. Preserve
historical citations unchanged and label their commit boundary.

Deprecate `dev/design/0.9.0-readability-refactor-proposal.md` as current
authority if Slice 9 accepts the 0.8.27 placement, while keeping its rationale.
Keep the concise synthesis, advisory scanner/thresholds, and append-only
ledger. Archive completed experiment phase/status records in place. Consider
deleting the 113 reproducible `structure/*.txt` maps and
`violations.{json,md}` only after reference and unique-content checks; retain
the horizontal/vertical review JSON unless its unique conclusions are proven
distilled.

Reject wholesale merging of experiment branches, re-reviewing/splitting the 59
settled LEAVEs, creating a new contributor guide without a demonstrated gap,
or turning line count into a correctness gate.

## Implementation, review, verification, and status

Implementation is this evidence/disposition record only. Behavioral TDD and
code review are not applicable. An independent read-only audit supplied the
file inventory and disposition; final package design review and verification
remain pending. No document named above was edited by this slice.

Status remains `REVIEW_PENDING`. No cleanup or temporary workspace was created.
Next: Slice 7 delivery evidence.
