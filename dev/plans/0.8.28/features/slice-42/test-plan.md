---
title: FathomDB 0.8.28 Slice 42 — test plan
status: PLANNED (R1 APPROVE-WITH-FIXES applied)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 test plan

Unit A adds no test: the RED case is the existing test run in parallel.
Unit B adds no tracked test; its probes are scratch code (requirements
REQ42-B4).

## 1. Build

```bash
cd src/rust
cargo test -p fathomdb-engine --features test-hooks --test slice135_robustness_first --no-run
```

Run the binary that this prints (`target/debug/deps/slice135_robustness_first-*`)
directly, so the 30 runs measure the tests and not the build. Record the
host, `nproc` and the commit with each loop.

## 2. Unit A

### 2.1 RED baseline (AC42-02)

At the slice base `d10ad65d5`, before the edit:

```bash
fails=0
for i in $(seq 30); do
  timeout 300 "$BIN" --nocapture >"$LOG/run-$i.log" 2>&1 || fails=$((fails + 1))
done
echo "failures=$fails/30"
```

Expected: failures, near 9 in 10, all on line 437. A run that fails on
another line is reported, not counted.

### 2.2 GREEN (AC42-01, AC42-05)

After the ruled edit (design § 1.4), rebuild and repeat § 2.1. Expected
`failures=0/30`. Then:

- the test alone, 5 times:
  `"$BIN" --exact slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`;
- the binary with `--test-threads=1`, 3 times.

All pass. The `SLICE135_ROBUSTNESS` record of the persistent case shows
`"fd_before":0,"fd_after":0` on Linux.

### 2.3 Leak checks (AC42-03, AC42-04)

Each is an uncommitted edit, run with the test alone, then reverted with
`git restore src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`
and the GREEN edit re-applied from its commit:

| Check | Edit | Expected |
| --- | --- | --- |
| Leaked file | Before line 429: `std::mem::forget(std::fs::File::open(&path).expect("leak"));` | FAIL on line 437, `left: Some(0)`, `right: Some(1)` |
| Leaked engine | Line 416: `std::mem::forget(reopened.engine);` instead of `close()` | FAIL on line 437, `right` above 0 |

Record the panic lines in status.md. Confirm with `git status` that
neither edit is left in the tree.

### 2.4 Scope (AC42-06)

`git diff d10ad65d5 -- src/` shows only the helper and lines 366 and 429.

### 2.5 Gate

`./scripts/agent-lint.sh` (clippy and rustfmt cover the test file), then
the full `./scripts/agent-verify.sh` once on the final clean tree. The
release-state suites `test-check-board-currency` and
`test-commission-manifest` failed on this release's state files before
the slice; record their result and whether it changed.

## 3. Unit B

### 3.1 Scratch consumer (AC42-07, AC42-08)

In the session scratch directory, never in the repository:

```text
<scratch>/slice42-consumer/
  Cargo.toml   # [dependencies] fathomdb-engine = { path = "<worktree>/src/rust/crates/fathomdb-engine", features = ["test-hooks"] }
               # plus the dev-dependencies the copied test needs
  tests/slice40_reader_page_cache.rs   # copied unchanged
```

```bash
cargo tree -i libsqlite3-sys          # must name the registry source
cargo test --test slice40_reader_page_cache                    # expected: AC40-95..97 FAIL
LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT \
  cargo test --test slice40_reader_page_cache                  # expected: PASS
```

Then the `[env]` variant (design § 2.2 E3): set `LIBSQLITE3_FLAGS` in
`<scratch>/slice42-consumer/.cargo/config.toml`, clean the
`libsqlite3-sys` build, rerun: expected PASS.

### 3.2 Installed CLI (AC42-07)

```bash
cargo install fathomdb-cli --version '=0.8.27' --locked --root "$SCRATCH/cli-mm"
strings "$SCRATCH/cli-mm/bin/fathomdb" | grep -x ENABLE_MEMORY_MANAGEMENT   # expected: found
LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT \
  cargo install fathomdb-cli --version '=0.8.27' --locked --root "$SCRATCH/cli-pc"
strings "$SCRATCH/cli-pc/bin/fathomdb" | grep -x ENABLE_MEMORY_MANAGEMENT   # expected: absent
```

Then the no-file form, expected absent too:
`cargo install fathomdb-cli --version '=0.8.27' --locked --root "$SCRATCH/cli-cfg" --config 'env.LIBSQLITE3_FLAGS="-USQLITE_ENABLE_MEMORY_MANAGEMENT"'`.

Allowed by owner ruling Q-B2: scratch directory only, no publish, no tag.

### 3.3 Route measurements (AC42-10)

As design § 2.5, with the C3 precision in design § 2.4: at least 3
runs per cell, median p50, mode 1 and the route on the same host, binary
and session, counters alongside, on an idle host. Each run records host,
commit, profile, mode, load average, `nproc` and command. The engine timing report runs with `--release --ignored
--exact reader_page_cache_timing_report` and `FATHOMDB_TC5_INPUTS` set.

### 3.4 Scope (AC42-12)

`git diff --stat` for the Unit B commits lists only
`dev/plans/0.8.28/features/slice-42/`. `./scripts/agent-lint-md.sh`
passes.
