# Current/history search POC

Status: COMPLETE — Astra reviewed and independently verified.
Qualified implementation: `4b6bc10531a28cfc415a5d038f7dc64ee9909090`.
Base: Slice 80, `dd8dbf6f218fbfa210ee972576874cd55dcbf08a`.

This experiment implements and tests three read meanings and compares three
ranking mechanisms. It is an isolated real SQLite store, not a production
FathomDB migration or new public API. Its SQLite dependency is pinned to the
same rusqlite release as the engine. Canonical history is separate from all
search projections. The design is [design.md](design.md); outcomes will be
recorded in [results.md](results.md).

Tools (all retained in this directory):

- `Cargo.toml`, `src/lib.rs`: disposable bitemporal assertion store and rankers.
- `tests/hypotheses.rs`: independent behavioral expectations, RED before GREEN.
- `src/main.rs`: deterministic cost sweep and machine-readable observations.
- `results.json`: bounded sweep output, including every sampled cell.

Commands from this directory (no production database is opened):

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo run --release --locked -- --records 128,512 --history 0,4,16 --samples 15
```

The sweep varies accepted logical records and excluded revisions while keeping
current content and query terms fixed. It reports per-backend p50/p95 latency,
logical current/history projection counts, SQLite page bytes, write time and
returned identities/reference scores and maximum backend score delta. These are synthetic, warm, serial
measurements on this host, not production service latency or memory bounds.

The axis N represents N node claims plus N edge claims, with two properties
per node. The JSON has 54 certified-current cells and nine additional
corrected-history/prior-belief/clock-fallback cells, all with 15 samples.
Nearest-rank p95 is the maximum sample at this count. Storage/write totals cover
all coexisting POC indexes. `native_rank_only_attribution` includes preselected
native ranking and Rust result assembly, not pure SQLite function time.

The POC establishes score/order isolation, not history-independent end-to-end
latency: canonical selection/certification still scans history. See the results
for the observed cost and the production implementation sequence it justifies.
No new worktree or branch was needed; the release ladder is unchanged.
