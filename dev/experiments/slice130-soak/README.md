# Slice 130 instrumented soak tools

[Protocol](../../plans/0.8.29/features/slice-130/campaign-protocol.md) governs
these portable tools. They do not launch a campaign, import an SDK, select a
venue or establish whole-E/shipping qualification. The historical .28 sampler
and Q39 outcomes are unchanged.

- [Sampler and evaluator](../../../scripts/slice130_soak.py)
- [Hand-computed measurement controls](../../../scripts/tests/test_slice130_soak.py)

Run the focused, native-free controls:

```sh
python3 -m unittest discover -s scripts/tests -p test_slice130_soak.py
```

Evaluate a retained instrumented pair for one SDK:

```sh
python3 scripts/slice130_soak.py /absolute/path/to/pair.json
```

The input is an object with `on` and `off` arm records. Each arm supplies `sdk`,
`pool_mode`, actual host `started_monotonic_s`, a separate `baseline` (`k=0`),
`samples` (`k=1` through `60`), actual supervised `exit_code` and `aborted`.
Each point supplies actual `elapsed_s`, `rss_bytes`, `vm_swap_bytes`, cumulative
`pswpin`/`pswpout`, completed `iterations`, `errors` and `mismatches`. On-pool
points require `counter_status="ok"` and checked `private_used_bytes` and
`private_reserved_bytes`. Off-pool points require named
`counter_status="no_private_pool"` and null private counters. Keep the complete
raw observation and its lossless identities in each point as well.

`collect_arm` accepts a real workload iteration, passive snapshot and streaming
JSONL event sink. Preload models before calling it; it performs one complete
warm-up before baseline, records sampler cost, streams every iteration's
latency/output data and retains only the baseline plus at most 60 snapshots.
`read_linux_memory` supplies RSS/anonymous/file/shared/swap bytes, MemAvailable
and native swap page counters. Add the actual D observation adapter and Node
heap diagnostics at the snapshot boundary. No native API name is assumed here.
The workload must retain the protocol's single embed, 32-item batch and rerank
fixtures, record exact model/input identities and return per-operation
`latency_ms`, output records and a per-iteration `mismatches` count.

The caller must use fresh real databases, preserve source/model/artifact and
host/session identities, stream raw events without buffering the full hour,
close actual SDK resources, and obtain exit status from the reaping supervisor.
The supervisor owns process timeouts, whole-arm ordering and real exit evidence;
the sampler cannot interrupt a stuck native forward. A missed sampling deadline
stops without fabricated catch-up rows. Missing completion/deadline/swap makes
the experiment UNMEASURED; actual operation/counter/numerical failures yield FAIL.

The CLI reports `scope="instrumented_measurement_only"`. It checks each SDK's
numerical on/off pair, including the signed median of same-k terminal RSS
differences. Actual concurrent Node/Python execution in each sequential arm,
reviewed build equivalence, shipping lifecycle/witness proof, D's real passive
observer, and the authorized Orin reservation remain prerequisites for E. No
live soak, native SDK smoke or shipping numerical plateau has been measured by
these portable controls.
