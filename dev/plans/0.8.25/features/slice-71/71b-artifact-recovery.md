---
title: Slice 71B — recovered preliminary evidence
status: QUALIFIED_NON_ACCEPTANCE_EVIDENCE
date: 2026-09-08
---

# Slice 71B recovered preliminary evidence

## Recovery boundary

The owner authorized one bounded, read-only attempt to recover the scratch
claims quoted in the Slice 71B plan. The attempt succeeded for the script,
its output, the temporary real diagnostic source, its invocation, and its
output. None of these artifacts was preregistered. They are retained here only
to inform the prospective protocol; they are not acceptance evidence and do
not set a performance bound.

The source session was
`90816219-d3ca-4779-af54-565696e34b18`, recorded at:

`/home/coreyt/.claude/projects/-home-coreyt-projects-fathomdb/90816219-d3ca-4779-af54-565696e34b18.jsonl`

At recovery time that 768,147-byte JSONL had SHA-256
`fb8609a56fc045b12c0685869fe93ad79b47158b6b2ba54ccc65479a6a5bc419`.
The transcript identifies branch `release/0.8.25`, checkout
`/home/coreyt/projects/fathomdb-worktrees/release-0.8.25`, and initial HEAD
`784cdfb6` (`docs(slice71): pause with remaining work outline`).

## Synthetic SQLite script

The recovered 4,490-byte script remains at the session scratch path:

`/tmp/claude-1000/-home-coreyt-projects-fathomdb/90816219-d3ca-4779-af54-565696e34b18/scratchpad/trigger_bench.py`

Its SHA-256 is
`fac3e42c63124fc350355f912c1733e5c323bac75e4c97683f4f53bbb165f7cc`.
The recorded invocation was `python3 trigger_bench.py`; the separately recorded
runtime probe reported SQLite 3.45.1. The script used Python `sqlite3`, WAL,
`synchronous=NORMAL`, 10,000 synthetic rows, batches of 1,024, three inserts
per row, and three repetitions for each trigger-body/preparation cross-product.
Its `fresh` arm deliberately changed SQL text with comments to defeat Python's
statement cache. That is a synthetic approximation, not a measurement of
rusqlite preparation behavior or the production Engine/projector interaction.

The recovered output was:

```text
none  cached: median     43.1 ms  [42.9, 43.1, 43.4]
none  fresh : median    125.1 ms  [124.7, 125.1, 127.4]
gen   cached: median     66.9 ms  [66.2, 66.9, 67.3]
gen   fresh : median    278.3 ms  [277.5, 278.3, 278.9]
nonce cached: median     77.3 ms  [76.2, 77.3, 80.1]
nonce fresh : median    322.5 ms  [317.1, 322.5, 322.9]
```

These numbers support retaining trigger execution, nonce work, and statement
preparation as hypotheses. They do not establish per-fire production costs,
the fraction of the real regression explained, or a preferred correction.

## Real one-run diagnostic

The transcript contains the complete temporary test body. It appended
`slice71_seed_only_diagnostic` to
`src/rust/crates/fathomdb-engine/tests/perf_gates.rs`, called the exact AC-013
seeder, read the ending generation, and printed its two durations. It was run
as:

```text
SLICE71_SEED_ONLY=1 AC013_VECTOR_DIM=384 cargo test --release \
  -p fathomdb-engine --test perf_gates slice71_seed_only_diagnostic \
  -- --nocapture
```

The recovered output was:

```text
SLICE71_SEED_ONLY n=10000 seed_write_ms=2130 drain_ms=2893 visibility_generation=50005
test slice71_seed_only_diagnostic ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 5.08s
```

The diagnostic edits were then reverted and never committed. No raw standalone
log, start generation, environment observation, database digest, or executable
digest was retained. Source inspection indicates it ran after the session's
initial `784cdfb6` identity and before any committed source change in that
session, but the output itself did not print the commit. It is therefore a
single corroborating observation, not an admissible cell. In particular,
`50005` is an ending value across setup, foreground writes, and projection
drain; it is not an exact trigger-fire count for any one phase.

## Resulting protocol decisions

- Keep acknowledgement and total ingest-to-drained as separate primary
  metrics; do not infer exclusive foreground/background time by subtraction
  when the projector overlaps acknowledgement.
- Observe generation at setup, acknowledgement, and drain boundaries.
- Use production, generation-only, and no-op-body arms before conditionally
  extending to preparation reuse.
- Bind an identical external probe to exact product checkouts and retain every
  raw cell. Synthetic or temporary diagnostics remain explanatory only.
