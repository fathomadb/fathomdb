# Erasure availability mechanism experiment

Unpublished Slice 81 tooling. Production erasure semantics, global dependency
fences and public routing remain unchanged. The candidate is registered-FTS
sanitation after successful native source erasure; it is not a deferred public
erasure service or a deletion SLA.

Run from the repository root. Use an existing compatible Cargo cache if desired;
never run competing builds or tests during a timed campaign.

```sh
manifest=dev/experiments/erasure-availability/Cargo.toml
cargo test --offline --manifest-path "$manifest"
cargo clippy --offline --manifest-path "$manifest" --all-targets -- -D warnings
python3 -m unittest discover -s dev/experiments/erasure-availability -p test_campaign.py
cargo build --offline --release --manifest-path "$manifest" --bin probe
python3 dev/experiments/erasure-availability/campaign.py /absolute/path/to/release/probe /tmp/new-slice81-campaign
```

The controller requires a new output directory and 12 GiB free scratch. Its
predeclared 24 cells each contain three replicas; fixture preparation, controls,
physical inspection and shutdown are charged to the 45-minute core budget.
Each replica has 30 seconds and each cell has 90 seconds. It samples an 8-GiB RSS
and 12-GiB scratch ceiling every 100 ms; overshoot and interrupted attempts are
retained. A failed no-erasure service control makes that loaded comparison
inconclusive. Raw events and final JSON include every scheduled request outcome;
qualified percentiles require complete service and physical proof. The controller
retains master fixtures and records, then removes only its own disposable replica
SQLite copies. Repeating a campaign requires a new reviewed authorization/budget.

`probe fixture`, `probe legacy` and `probe replica` are controller entry points;
run the binary without arguments for their signatures. Fixture counts and primary
1024-byte payloads are checked on every actual large fixture and replica. Runtime
mode is explicitly performance before any SQLite connection, including cached-file
and fresh-file children. Node, edge, property and coexisting `search_index_v2`
content/shadow counts are recorded independently; an empty family is not a pass.

Safety tests cover positive shipping ghosts, A prospective secure deletion and
legacy rebuild, B negative-first/positive-continuation merging, rollback and real
process exit, original deadlines, busy checkpoint retry, registered-dependency
versus dependency-free native fencing, exact survivor content and score bits, and
1-MiB/common-term, Unicode and zero-token stress. The ignored `crash_child` test is
invoked by its parent recovery test, which verifies exit status and reopens the
actual database; it is not omitted coverage.

Limitations: foreground calls are ordinary Engine operations under a restricted
unrelated-source workload. Edge-fixture read load searches stable endpoint text,
not edge ranking. Same-source reingestion is excluded, not natively fenced by the
POC. Telemetry-enabled/unavailable sinks, scoped native admission, broad graph or
historical routing, dependency expansion and expiry scheduling are not qualified.
Fresh native erasure timings are call durations, not writer-lock measurements.
Only legacy A rebuild/B merge uses comparable lock-held instrumentation. Successful
terminal sanitation receipts are immutable observations under exclusive fixture
ownership; they do not detect later independent source ingestion.

See the [slice protocol](../../plans/0.8.28/features/slice-81/experiment-design.md)
and [requirements](../../plans/0.8.28/features/slice-81/requirements.md).

The reviewed continuation uses the unchanged probe and original masters:

```sh
python3 dev/experiments/erasure-availability/supplement.py /absolute/path/to/release/probe /tmp/fathomdb-slice81-campaign --plan
python3 -m unittest discover -s dev/experiments/erasure-availability -p test_supplement.py
python3 dev/experiments/erasure-availability/supplement.py /absolute/path/to/release/probe /tmp/fathomdb-slice81-campaign
```

`--plan` prints the exact never-started IDs and verified inputs without launching
children. Execution creates a new `supplement/` directory, charges validation/setup
to its separate 25-minute active budget, carries original cell time and counts all
retained original/new scratch files. Original outcomes and raw records are not
modified. This continuation requires the explicit protocol revision above; it is
not an automatic retry facility.
