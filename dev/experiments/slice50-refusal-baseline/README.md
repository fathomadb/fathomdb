# Slice 50 refusal baseline

Owner-requested characterization of the unchanged engine at `ac83688a1`;
D50-1 and D50-2 remain unimplemented. These are current-behavior baseline
assertions, not RED tests specifying a yet-unapproved replacement behavior.

## Tools and method

- `src/rust/crates/fathomdb-engine/tests/slice50_refusal_baseline.rs`: ignored
  real-database matrix. Nine operations, four states, three repeats by default.
- `run.py`: executes the matrix, preserves raw output, records provenance,
  and checks that every observation and recovery was captured exactly once.
- Existing `slice30_embedding_readiness` and `tc68_probe_fingerprint_cache`
  tests supply the original refusal and cached-drift controls.

Run from the checkout root (Rust dependencies must be cached):

```sh
python3 dev/experiments/slice50-refusal-baseline/run.py \
  --out dev/experiments/slice50-refusal-baseline/results --repeats 3 --threads 4
cargo test --offline -p fathomdb-engine --features test-hooks,operator \
  --test slice30_embedding_readiness --test tc68_probe_fingerprint_cache
```

No credentials, model download, GPU, production database, or engine change.
`REFUSAL_REPEATS` also controls repeats if invoking the ignored Rust tests
directly. The public drain uses a 3-second deadline; internal operations use
the unchanged 30-second deadlines. Parallel tests own separate temporary stores.
These are timeout characterizations, not machine-comparable microbenchmarks.

Each store has a faithfully projected anchor, a non-vector provenance-bearing
node to mutate, and (except the empty control) one pending body edge. The
accepted runtime is first verified with the stored identity. Refused cases
explicitly invalidate the verification cache, then use deliberately different
vectors under the same identity. This induces a known mismatch; it does not
measure how often a production CPU or CUDA runtime would be refused.

States: refused with pending work; refused without pending work; absent with
pending work; accepted and fully drained before the measured operation. The
last is a ready-state control, not a test of rebuild versus active workers.
The configuration call re-applies the current declarations unchanged.

Every failed operation must preserve a sorted, typed-row SHA-256 snapshot of
all application tables (including projection tables and durable audit state),
and preserve the pending count. Every case closes and reopens with the original
faithful embedder and must drain to zero pending rows. The observations record
readiness, refusal reason, embedding calls, elapsed time, results and snapshots.
Snapshots are logical table contents, not file bytes or a physical-erasure proof.

## Results and interpretation

Results and historical decision analysis are recorded after the run. No release
completion or full-workspace green claim follows from this experiment.
