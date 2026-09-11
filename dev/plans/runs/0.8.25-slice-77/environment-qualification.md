# Slice 77 executor qualification

The Slice 77 run did not retain the protocol-required contemporaneous executor
preflight. CPU affinity/quota, competing load, swap activity, and thermal state
during each profile/timing process are therefore **unverified**. The seven
anchor timings are diagnostic confirmation of the continuing AC-020 failure,
not a release-qualified comparison for selecting or accepting a treatment.

Stable facts recoverable from the hash-bound source and audited host are:

- host `windchill3`, Linux x86-64, AMD Ryzen Threadripper PRO 5945WX (12 cores,
  24 logical CPUs);
- Rust/Cargo 1.95.0 from `rust-toolchain.toml`;
- `rusqlite` 0.40.1, `libsqlite3-sys` 0.38.1 with bundled SQLite 3.53.2, and
  `sqlite-vec` 0.1.9 from the exact `c53d24a1` lock/source inputs;
- timing binary `86495537...d6083`, profiling binary
  `bf87764a...9b67`, and relevant-tree hash `0c93c881...18ba`.

The actual inner timing command used by the collector was:

```text
env AGENT_LONG=1 /tmp/fathomdb-slice77/bin/A/perf_gates ac_020_reads_do_not_serialize_on_a_single_reader_connection --exact --nocapture --test-threads=1
```

For labels `IA1` through `IA7`, the outer collector was
`python3 dev/tools/slice76_ac020.py run` with configuration `A`, the sealed
binary and manifest above, the shared observations file, and the corresponding
retained log path. The collector did not impose the manifest's stated
600-second timeout. Every retained process completed in less than 0.4 seconds,
but that does not retroactively satisfy the missing control.

This qualification does not change the 0/7 registered verdicts. It prevents
using the timing series as treatment-attribution evidence and is another reason
that Slice 77 authorizes no implementation.
