# AC42-11 — registry-crate consumer of the Unit I tests

[`run.sh`](run.sh) builds a crate outside the workspace with path dependencies
on an exported tree's `fathomdb-engine` (`test-hooks`) and `fathomdb-schema`.
No root `[patch]` reaches it, and it gets a fresh lockfile, so Cargo resolves
`libsqlite3-sys` from crates.io as a consumer of the published crates does. It
copies `tests/slice40_reader_page_cache.rs` unchanged (AC40-95 to AC40-97) and
adds a test that prints the linked SQLite version and whether
`ENABLE_MEMORY_MANAGEMENT` is compiled in. `LIBSQLITE3_FLAGS` must be unset.

```bash
git archive <sha> | tar -x -C "$TREE"
bash run.sh "$TREE" 0.41 "$OUT"   # 0.40 for trees before the bump
```

## Results, 2026-10-31, windchill3 (x86_64, nproc 24)

| Tree | `libsqlite3-sys` (lock) | SQLite | `ENABLE_MEMORY_MANAGEMENT` | AC40-95 | AC40-96 | AC40-97 |
| --- | --- | --- | --- | --- | --- | --- |
| base `59079d5c5` (`rusqlite` 0.40.2) | 0.38.2, crates.io, checksum `f1d20bef…` | 3.53.2 (3053002) | 1 | FAIL: `left: 1`, `right: 0` | FAIL: reader 0 holds 4,483,192 bytes after the heavy read, above 2,252,800 | FAIL: readers lost cached pages, `(reader, misses)` `[(1, 108), (2, 108), (3, 108), (4, 16)]` |
| final `2af70d3bc` (`rusqlite` 0.41.1) | 0.39.1, crates.io, checksum `14a5f01b…` | 3.54.1 (3054001) | 0 | pass | pass | pass |

The base run is what `cargo install fathomdb-cli` and published-crate
consumers got before Slice 42; the final run is what they get after it. The
harness was re-run from this script on the final tree with the same result.
