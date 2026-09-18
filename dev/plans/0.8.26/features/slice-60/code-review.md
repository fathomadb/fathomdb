---
title: FathomDB 0.8.26 Slice 60 — independent implementation review
status: CHANGES_REQUESTED
reviewed_on: 2026-09-17
reviewed_tip: 6b238840
---

# Slice 60 independent implementation review

The independent read-only review returned **CHANGES REQUESTED**.

## Blocking finding

- **P1 — corruption recovery reachability.** `wire_recover` calls public
  `Engine::open` before dispatching any recovery action. The open path rejects
  a malformed WAL in `probe_wal_sidecar` with `E_CORRUPT_WAL_REPLAY`, so
  `recover --accept-data-loss --truncate-wal` cannot reach
  `Engine::truncate_wal` for the fault whose recovery hint directs operators to
  that action. `doctor safe-export` similarly uses public open, so a malformed
  header prevents the hinted export attempt. This is a product behavior versus
  accepted-authority conflict and activates the Slice 60 stop gate.

The reviewer found no accepted residual or exception. Product implementation
is outside the approved Slice 60 scope until the repository owner explicitly
authorizes expanding it.

## Documentation findings

The review also found two P2 and two P3 documentation defects:

1. fresh bootstrap was incorrectly described as returning no migration steps;
2. the rewrite removed six headings still cited by live source/tests;
3. one-object doctor output was not explicitly scoped to `--json`; and
4. the TDD record listed probe outcomes without the exact commands.

The follow-up diff corrects all four without changing product source. Required
anchors restored are `Writer / reader split`, `Close path`,
`Soft-fallback signal`, `Doctor-only flags`,
`Code-to-operator-action cross-reference`, and
`JSON shapes for other doctor verbs`.

## Confirmed evidence

- Product source diff against `b770de01`: empty.
- Doctor inventory: 15 commands; recovery inventory: five actions.
- Legacy-status ratchet: exactly three owners moved from `locked` to `ACTIVE`,
  justifying `LEGACY_BUDGET=43`.
- Successor ADR authorization, predecessor backlink, and decision-index entry:
  present.
- Pre-review design lifecycle, regression fixture, Markdown, diff, and focused
  semantic probes: pass.

Final review remains non-PASS until the P1 is explicitly scoped, fixed through
RED/GREEN, and independently rereviewed.
