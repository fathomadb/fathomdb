---
title: FathomDB 0.8.26 Slice 55 — executable SDK parity oracle
status: APPROVED
target_release: 0.8.26
---

# Slice 55 plan — executable SDK parity oracle

## Outcome

Complete `N26-55`: replace the governed-surface subset checks with an oracle
that fails when a live Python or TypeScript operation exists on only one
binding, disappears from one binding, is misspelled, or is represented as live
only by a reserved manifest entry. The code-grounded inventory found one
existing implementation defect: signed and documented operation `rerank` is
live only in Python. This slice restores its already-approved TypeScript peer;
it adds no canonical operation and leaves the signed 69-spelling allowlist
byte-identical.

## Reconciliation since the draft

The draft landed at `65f69ca1`. No later repository change altered Slice 55 or
its assigned inputs before execution. The entry review found:

1. The signed allowlist, locked Python and TypeScript interfaces, and accepted
   parity ADR all classify `rerank` as governed and live, but only Python
   exports it. Commit `94732bb5` introduced the Python-only implementation and
   relied on the subset oracle; the later 0.8.5 design preserved that gap.
2. The signed `governed-surface-allowlist.json` is raw-byte pinned. Rewriting
   it into the proposed canonical schema would correctly fail the pin gate and
   is rejected. A companion canonical map must carry separate signed tokens
   whose union is exactly its unchanged 69 members.
3. The real discovery roots are package-level functions, `Engine` static and
   instance methods, and the `admin`, `read`, and `graph` namespaces. The draft
   omitted package and graph roots and therefore could not detect `rerank` or a
   graph-only drift.
4. Identical idiomatic spellings across bindings are valid for the same
   canonical operation. Uniqueness is per binding and locator; one spelling
   may not identify two canonical operations.
5. No Slice 8 or later draft item is additionally allocated here. Retrieval,
   recovery, and engine-owner reconciliation remains Slice 60; generalized
   lifecycle guards and candidate requalification remain Slice 65.

Disposition: approve the narrow companion-map/oracle design, restore only the
missing TypeScript peer required by existing authority, and reject any in-place
allowlist rewrite, new operation, broad binding cleanup, or release
requalification.

## Scope

In scope:

- represent governed surface entries as canonical operations with explicit
  Python and TypeScript locators/spellings and a `live` or `reserved` state in
  a companion map mechanically bound to the unchanged signed allowlist;
- introspect both real binding surfaces, map their spellings to canonical
  operations, and assert exact equality of the live canonical sets;
- retain a documented exclusion set for non-command runtime attributes;
- add RED mutation fixtures for a one-sided removal, one-sided addition,
  misspelling, duplicate spelling, and reserved-as-live error;
- update `dev/design/bindings.md` and affected conformance guidance so their
  parity claims describe the executable oracle exactly; and
- restore the already-governed package-level TypeScript `rerank` operation with
  the Python identity, validation, result, and feature-off contracts; and
- preserve the five-name SDK recovery denylist. The CLI name `doctor` remains
  SDK-absent through the positive governed-surface contract.

Out of scope:

- adding, removing, or renaming a canonical governed operation;
- adding recovery, repair, or diagnostic operations to either SDK;
- rewriting retrieval, recovery, or engine design owners; and
- broad release requalification, which belongs to Slice 65.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-55A: canonical operation model | AC26-55A: one shared companion map identifies every governed operation canonically, records its signed token(s) separately from both binding locators and runtime spellings, rejects duplicate or incomplete live mappings, distinguishes live from reserved entries, and flattens all signed tokens exactly to the unchanged 69-member allowlist. |
| R26-55B: real-surface equality | AC26-55B: Python and TypeScript tests introspect package, Engine, admin, read, and graph command roots, map their exact idiomatic spellings to canonical operations, and assert exact equality with each other and the live companion-map set. |
| R26-55C: non-vacuous RED proof | AC26-55C: committed or staged RED fixtures demonstrate that each one-sided removal/addition, misspelling, duplicate spelling, and reserved-as-live mutation fails for the intended reason before GREEN. |
| R26-55D: approved surface preservation | AC26-55D: the before/after canonical live-operation set is identical; any discovered surface disagreement stops the slice instead of silently selecting one binding as authority. |
| R26-55E: focused verification | AC26-55E: focused manifest, Python, and TypeScript surface suites plus `agent-verify` pass without weakening exclusions or denylist assertions. |
| R26-55F: existing rerank parity | AC26-55F: TypeScript exports the already-approved standalone `rerank` peer; depth zero and empty input are model-free identity paths, malformed input fails before native work, feature-off behavior matches Python, and the result preserves `id`, blended `score`, and nullable `ceScore`. |

## Execution sequence

1. Commit a code-grounded inventory of all 69 signed tokens, their canonical
   IDs, and per-binding discovery locators/runtime spellings. Mechanically prove
   the signed-token union equals the legacy membership without changing that
   file.
2. RED: add one pure validator used by the conformance gate and mutation tests;
   preserve failures for one-sided removal/addition, misspelling, duplicate
   spelling, reserved-as-live substitution, signed-flattening drift, and the
   actual missing TypeScript `rerank` peer.
3. GREEN: add the companion map and exact Python/TypeScript introspection over
   package, Engine, admin, read, and graph roots. Keep reserved entries outside
   the live equality set and exclusions explicit.
4. RED/GREEN: add the TypeScript `rerank` contract test first, then the smallest
   N-API plus TypeScript wrapper needed to match the existing Python contract.
5. Update the maintained binding design and interface wording to match the
   executable behavior, then obtain independent code review.
6. Run focused validator, Rust/N-API, Python, and TypeScript suites plus
   `agent-verify`; record exact evidence in `status.md`.

## Downstream reconciliation

Slice 60 may amend the canonical mapping or binding prose if its code-grounded
retrieval/recovery inventory finds a real approved operation was classified
incorrectly. Such an amendment must add or adjust a RED fixture and rerun the
Slice 55 focused suites; it may not bypass exact equality.

## Stop gates

Stop on any further unresolved public-surface disagreement, a required ADR
successor, a proposed new canonical operation, signed-allowlist byte change,
recovery-name leakage, or a test strategy that cannot demonstrate a concrete
failing mutation.
