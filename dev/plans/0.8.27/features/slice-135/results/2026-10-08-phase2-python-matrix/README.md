---
title: Slice 135 Phase 2 installed-Python query matrix
status: AUDITED_PAIRED_CONTRACT_MATRIX
target_release: 0.8.27
---

# Phase 2 installed-Python query matrix — 2026-10-08

The [frozen protocol](../../phase2-python-matrix-protocol.json), SHA-256
`e6b8502882b8d310b5f18b60147c629ae2d0573a522365709d732c40521b1212`,
was committed before either [recorded run](run-commands.json). It binds the
four-row repository-authored fixture, the authored filter and page contracts,
the collector bytes, exact 0.8.26 and 0.8.27 source identities, and the
installed Python wheel/native bytes. Each version wrote the same rows to its
own fresh SQLite database with default embedding disabled, then repeated all
four cases after close and reopen.

The [independent audit](audit.json) checked each version against the frozen
gold and inspected each retained database. Both versions matched the exact
kind-filtered hit, a successful empty result from the status filter, two
ordered cursor pages with correct termination, and a `PageError` with
`invalid_page_limit` at `/limit`. All four outcomes remained identical to
their independent expectations after reopen. Each database passed SQLite
integrity and held the four expected active canonical rows. Search-hit
provenance was checked; the page API returns `NodeRecord` without `source_id`,
so page items are checked for their public logical ID, kind, body and order.

Eight [negative controls](negative-controls.json) used copies of the raw
archives and were rejected: wrong filtered ID, error substituted for empty
success, empty success substituted for typed refusal, reordered reopened
pages, omitted case, wrong wheel, missing paired receipt and altered persisted
body. The original [baseline](raw-baseline/raw.json) and
[candidate](raw-candidate/raw.json) raw receipts and SQLite databases remain
unchanged. [SHA256SUMS](SHA256SUMS) binds the retained files.

The unchanged full `./scripts/agent-verify.sh` gate passed in a
ptrace-capable executor. Its [verification output](verification.stdout)
records AC-036 and live AC-037 passes, zero security violations, blockers or
downgrades, and 186/186 test suites passing. A temporary Git exclude hid
untracked Phase 1 raw outputs from clean-worktree test preconditions without
removing or altering those outputs.

This matrix extends deterministic contract coverage only. Graph/evidence,
erasure, vector fidelity, judged retrieval relevance, memory usefulness,
generated-answer quality and other SDK routes remain open under the
[Phase 2 plan](../../plan.md#phase-2-qualification-and-execution-order).
