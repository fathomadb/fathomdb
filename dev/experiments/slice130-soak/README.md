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

## Real SDK workers and lifecycle cases

The installed-worker bindings are [Python](python_consumer.py) and
[Node](node_consumer.mjs); Rust has the ignored
[source-SDK fixture](../../../src/rust/crates/fathomdb-sdk/tests/slice130_lifecycle.rs).
They use real databases/models. Source controls do not qualify native hardware.
Both device policies must be fixed to `cuda:0` before worker import; caches are
offline. No worker requests GC, synchronization or trim.

Prepare a worker JSON with `command` (absolute executable plus argument array),
`cwd` (fresh installed consumer) and `environment` containing the selected
`FATHOMDB_EMBED_DEVICE`, `FATHOMDB_RERANK_DEVICE`, model cache and owned tool
settings. Python must run its owned installed venv's interpreter; Node must use
its installed consumer via `--consumer-root /absolute/consumer`. Append
`--lifecycle` to either worker command for D; leave it absent for E.

```sh
python3 scripts/slice130_soak.py --lifecycle /absolute/python-worker.json \
  --case max --output /absolute/new-python-max
```

Run `max`, `zero`, `exit-alive`, `cls` and `reranker` separately for Python/Node;
Python also selects `drop`. Node explicitly refuses deterministic implicit drop.
Each invocation creates fresh engine DBs and reaps a fresh child within120s.
After close/drop, passive samples retain the actual release transition and
5s bound; no later forward is used to manufacture final zero. Unclosed and
singleton children require bounded normal exit, not final zero.

For Rust, compile the fixture with
`embed-cuda,rerank-cuda,cuda-pool-observer`, then explicitly select one ignored
`lifecycle_max`, `lifecycle_zero`, `lifecycle_drop`, `lifecycle_exit_alive`,
`lifecycle_cls_retains` or `lifecycle_reranker_retains` under the approved Orin
reservation. The parent fixture bounds and reaps its actual fresh child.

## Collect the existing two-arm protocol

The `--collect` spec contains `workers` keyed by `python` and `node`, each using
the worker shape above, plus mandatory positive caller-selected `limits`:
`mem_floor_bytes`, `free_floor_bytes`, `storage_limit_bytes`, `arm_timeout_s`
and `request_timeout_s`. Requests are bounded by120s; the arm watchdog must
include warm-up plus the full3600s schedule. These values and the reserved
host/GPU window require actual execution authorization; there are no automatic
resource defaults or new run authority from this README.

```sh
python3 scripts/slice130_soak.py --collect /absolute/selected-workers.json \
  --output /absolute/new-campaign
```

The supervisor launches both SDKs concurrently in pool-on, then pool-off after
both on measurements pass. It streams worker stdout/stderr and JSONL events,
keeps only baseline/60 snapshots in memory, stops on errors/deadlines/resource
floors and reaps owned process groups. Failed on prevents off. The resulting
`python-pair.json` and `node-pair.json` use the existing evaluator; incomplete
controlled-process tests are never accepted as real arms. A cleanup kill cannot
be a healthy exit or evidence of product teardown.

Shipping wrappers accept `--qualification-observer` only for the separate D/E
counterparts, appending `cuda-pool-observer` to their unchanged shipping tuple.
Use separate owned target/output directories. Both preserve locked resolution; counterparts additionally force the selected
offline cache. Select `CARGO_NET_OFFLINE=true` for local shipping qualification; source/toolchain/model equivalence and actual shipping installed
controls remain mandatory. The existing Python installed smoke accepts
`--local-wheel FILE VERSION+tegra SHA256 CANDIDATE_SHA NEW_OUTPUT`; its existing
Pages route remains available. Local input never invents Pages deployment
provenance. Neither route publishes an artifact.

## Slice 50 default Node diagnostic

The explicit `profile: "node-default-10m"` collection spec selects only the
`node` worker, sixty 10-second samples, the last thirty as the terminal window,
and a 5/6-second schedule tolerance. Its arm watchdog is at most750s and each
request remains at most120s. The collector injects `--node-default-profile` and
checks its ready handshake, ordinary Node flags and passive JS/V8 measurements.
No heap flags, forced GC, synchronization or trim are used. Historical hour
profiles retain their existing cadence and stop rules.

The numerical bounds stay unchanged. A complete, valid numerical-only failure
still permits the matched OFF diagnostic and signed pair deltas; operation,
counter, resource, deadline or close failures do not. This ten-minute diagnostic
does not establish an hour plateau or shipping qualification. The selected
Orin entry point is [the existing-peer guarded driver](../../tools/slice50_orin_profile.py).
Its output retains actual samples, natural exits and the unprivileged fuser
visibility limit; it cannot establish host-wide peer absence or total GPU bytes.
