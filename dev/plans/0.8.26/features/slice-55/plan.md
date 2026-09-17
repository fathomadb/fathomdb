---
title: FathomDB 0.8.26 Slice 55 — executable SDK parity oracle
status: DRAFT
target_release: 0.8.26
---

# Slice 55 plan — executable SDK parity oracle

## Outcome

Replace the governed-surface subset checks with an oracle that fails when a
live Python or TypeScript operation exists on only one binding, disappears
from one binding, is misspelled, or is represented as live only by a reserved
manifest entry. This slice changes conformance machinery and its binding-design
description; it does not add, remove, or rename a public operation.

## Scope

In scope:

- represent governed surface entries as canonical operations with explicit
  Python and TypeScript spellings and a `live` or `reserved` state;
- introspect both real binding surfaces, map their spellings to canonical
  operations, and assert exact equality of the live canonical sets;
- retain a documented exclusion set for non-command runtime attributes;
- add RED mutation fixtures for a one-sided removal, one-sided addition,
  misspelling, duplicate spelling, and reserved-as-live error;
- update `dev/design/bindings.md` and affected conformance guidance so their
  parity claims describe the executable oracle exactly; and
- preserve the five-name SDK recovery denylist. The CLI name `doctor` remains
  SDK-absent through the positive governed-surface contract.

Out of scope:

- changing product behavior or the approved SDK surface;
- adding recovery, repair, or diagnostic operations to either SDK;
- rewriting retrieval, recovery, or engine design owners; and
- broad release requalification, which belongs to Slice 65.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-55A: canonical operation model | AC26-55A: one shared manifest identifies every governed operation canonically, records both binding spellings, rejects duplicate or incomplete live mappings, and distinguishes live from reserved entries. |
| R26-55B: real-surface equality | AC26-55B: Python and TypeScript tests introspect their real exported command surfaces, map them to canonical operations, and assert exact equality with each other and the live manifest set. |
| R26-55C: non-vacuous RED proof | AC26-55C: committed or staged RED fixtures demonstrate that each one-sided removal/addition, misspelling, duplicate spelling, and reserved-as-live mutation fails for the intended reason before GREEN. |
| R26-55D: approved surface preservation | AC26-55D: the before/after canonical live-operation set is identical; any discovered surface disagreement stops the slice instead of silently selecting one binding as authority. |
| R26-55E: focused verification | AC26-55E: focused manifest, Python, and TypeScript surface suites plus `agent-verify` pass without weakening exclusions or denylist assertions. |

## Execution sequence

1. Inventory the current Python and TypeScript public command surfaces and map
   each approved spelling to a canonical operation. Reconcile any ambiguity
   against accepted ADRs and interface documents before changing tests.
2. Add the mutation fixtures and preserve their RED output. A test that only
   compares a literal with another copy of itself is not an oracle.
3. Implement the smallest manifest/schema and introspection changes that make
   all RED arms GREEN. Keep reserved entries outside the live equality set.
4. Update the maintained binding design and conformance comments to match the
   executable behavior.
5. Run focused suites and `agent-verify`; record exact evidence in `status.md`.

## Downstream reconciliation

Slice 60 may amend the canonical mapping or binding prose if its code-grounded
retrieval/recovery inventory finds a real approved operation was classified
incorrectly. Such an amendment must add or adjust a RED fixture and rerun the
Slice 55 focused suites; it may not bypass exact equality.

## Stop gates

Stop on an unresolved public-surface disagreement, a required ADR/interface
change, a proposed new operation, recovery-name leakage, or a test strategy
that cannot demonstrate a concrete failing mutation.
