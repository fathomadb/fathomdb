# Current/history follow-up experiments

Status: IMPLEMENTING. Production behavior and accepted ADRs remain unchanged.
See [scope](../next-poc-plan.md) and [production candidate](../../../design/current-history-search.md).

- P1: real Engine schema/writes; additive trigger-owned temporal ledger.
- P2: standalone candidate with eight real read-only SQLite workers, engine writer
  admission and native FTS. Performance is SCOUT, not integrated AC qualification.
- P3: private native FTS selected corpus with production query compilation.

P3 controls fixed before measurement: families node/edge/property; histories
0/4/16; callers 1/8; indexed payload 128/4096 UTF-8 bytes; views current,
corrected, prior, archive, inactive, superseded, both. Order is that nested order
(family outermost, view innermost). Exactly 1000 scheduled requests per cell;
600 seconds per cell, 7200 seconds campaign, process RSS stop 8 GiB. No replacement
attempts. Unfinished cells remain listed. Monitoring may overshoot RSS budget.
Raw measurements live under `/tmp/current-history-followups-results`.
