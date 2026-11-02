# Current/history search POC

Status: design and hypotheses drafted; implementation not yet qualified.
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
checksums of returned identities/scores. These are synthetic, warm, serial
measurements on this host, not production service latency or memory bounds.
