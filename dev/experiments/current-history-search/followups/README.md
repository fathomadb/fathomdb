# Current/history follow-up experiments

Status: P1/P2 scoped qualification complete; P3 measured PARTIAL. Production behavior and accepted ADRs remain unchanged.
See [scope](../next-poc-plan.md) and [production candidate](../../../design/current-history-search.md).

- P1: real Engine schema/writes; additive trigger-owned temporal ledger.
- P2: standalone candidate with eight real read-only SQLite workers, engine writer
  admission and native FTS. Performance is SCOUT, not integrated AC qualification.
- P3: private native FTS selected corpus with production query compilation.

P3 controls fixed before measurement: families node/edge/property; histories
0/4/16; callers 1/8; indexed payload 128/4096 UTF-8 bytes; views current,
corrected, prior, archive, inactive, superseded, both. Order, fixed before any measurement, is history → payload → callers → view → family
(family innermost, so budget exhaustion preserves breadth). Exactly 1000 scheduled requests per cell;
600 seconds per cell, 7200 seconds campaign, process RSS stop 8 GiB. No replacement
attempts. Unfinished cells remain listed. Monitoring may overshoot RSS budget.
Raw measurements live under `/tmp/current-history-followups-results`.

P3 uses native contentless FTS5 (`detail=full`, `columnsize=1`), streamed inserts, explicit private-connection memory-only temp storage, and exact snapshot hydration. The controller includes fixture construction in cell/campaign time, retains every planned cell, and samples child-process RSS every 50 ms. A killed process may leave an incomplete fixture; such a fixture must not be reused as complete.


See [results and limitations](RESULTS.md), [P1](P1.md), [P2](P2.md) and [P3](P3.md).
The original P3 grid was interrupted for a runtime-profile correction; the
approved revised stress diagnostic exhausted the original combined budget.

## Rerun tools

Run from the repository root; set `manifest=dev/experiments/current-history-search/followups/Cargo.toml`.

- Default suite: `cargo test --offline --manifest-path "$manifest"`.
- P2 fresh cost fixture: `cargo run --offline --release --manifest-path "$manifest" --bin current_cost -- /tmp/new-p2-results`.
- P2 exact isolation replay on copies: `cargo run --offline --release --manifest-path "$manifest" --bin current_isolation -- /tmp/current-history-followups-results/p2-final /tmp/new-isolation-results`.
- P3 build: `cargo build --offline --release --manifest-path "$manifest" --bin history_cell`; controller: `python3 dev/experiments/current-history-search/followups/campaign.py dev/experiments/current-history-search/followups/target/release/history_cell /tmp/new-p3-results --stress --prior-debit 560`. Each run requires its own budget authorization; this command reproduces the recorded diagnostic controls.
- Controller tests: `python3 -m unittest discover -s dev/experiments/current-history-search/followups -p test_campaign.py`.
- Candidate erasure: `cargo test --offline --manifest-path "$manifest" --test current_erasure`.
- Shipping physical-erasure diagnostic: `cargo test --offline --manifest-path "$manifest" --test erasure_probe -- --ignored --nocapture`. This is a known FAIL, separate from the default suite; production source is unchanged.

Raw logs remain in `/tmp/current-history-followups-results`; generated databases
and per-query records are not tracked. The replay refuses existing outputs and
requires checkpointed originals. Fresh fixed candidate fixtures are required for
erasure qualification; older experimental trigger schemas are not upgraded.
