# Erasure availability mechanism experiment

Unpublished Slice 81 tooling. Slice 81 changed no production erasure semantics,
global dependency fences or public routing. Its candidate is registered-FTS
sanitation after successful native source erasure; it is not a deferred public
erasure service or a deletion SLA.

## Slice 82 production qualification

`phase1 MASTER DISPOSABLE_OUTPUT` copies a frozen master, profiles the linked
checkout's actual native erase and reports SQL counts/time, native elapsed time,
exact report counts and physical markers while the Engine remains open.
Nested FTS shadow statement times overlap parent times: never sum both as stage
time. `phase1 fixture OUTPUT FAMILY OWNERS` prepares 1k/5k controls with the
original five-revision/10%-target generator. `phase1 mutations OUTPUT FAMILY
POLICY` compares 1,000 ordinary writes followed by four superseding revisions;
FAMILY is node/property and POLICY default/secure. Fixture construction is timed
separately. There is no new production cleanup algorithm in this driver.

Build with `cargo build --release --manifest-path
dev/experiments/erasure-availability/Cargo.toml --bin phase1`. Freeze baseline and
candidate executables before comparisons. Use distinct output paths, sequential
commands, unchanged hashed masters and no simultaneous builds/tests. The
[Slice 82 design](../../plans/0.8.28/features/slice-82/design.md) records commands,
inputs and the bounded qualification envelope; its status records actual results.
The original Slice 81 POC tests/results remain historical baseline evidence:
reproduce them against their recorded pre-repair source, not against Slice 82's
changed native physical semantics and root-only fence.

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
inconclusive. Complete replicas reconcile every scheduled request ID. Interrupted records retain
observed outcomes, missing dispositions and unfinished calls; their qualified
percentiles remain null. Qualified percentiles require complete service and physical proof. The controller
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

The reviewed monitor-recovery segment adds `--recover-unstarted` to the continuation
command (and optionally `--plan`). It writes a new `supplement-recovery/`, selects
only IDs still never started across both immutable prior segments, and subtracts
the first continuation's elapsed time from its original 1500-second allowance.
It is explicitly reviewed recovery from a resource-observer exit race, not an
attempt-retry or deadline-reset mode.

See [RESULTS](RESULTS.md) for frozen evidence, all 72 dispositions, qualification
limits and successor recommendations.


## Slice 84 bounded paired campaign

The [Slice 84 plan](../../plans/0.8.28/features/slice-84/plan.md) fixes 54 cells,
a shared 45-minute active budget, 90-second cell limit, and unchanged masters.
Build the instrumented final-Slice-83 baseline before changing production behavior;
freeze that binary separately. After reviewed implementation, freeze the candidate.
Both use the same driver, observer and provider. `idle` runs actual erase without
foreground offers; unknown modes refuse. Baseline and candidate use identical
100-Hz reads, 100-Hz node/edge writes and 20-Hz property writes.

```sh
cargo build --release --manifest-path dev/experiments/erasure-availability/Cargo.toml --bin phase2 --features phase2-metrics
python3 -m unittest discover -s dev/experiments/erasure-availability -p test_slice84_campaign.py
python3 dev/experiments/erasure-availability/slice84_campaign.py profile BASELINE PHASE82 MASTERS OUTPUT OWNED_ROOT
python3 dev/experiments/erasure-availability/slice84_campaign.py finish CANDIDATE OUTPUT
```

Use absolute paths; `OUTPUT` must not already exist and must be within the
common `OWNED_ROOT` containing all campaign binaries and generated scratch.
The profile command performs three baseline diagnostics, then pauses for
implementation/review. Finish performs the remaining cells exactly once. The
manifest preserves segments, source hashes, all dispositions and resource limits;
no automatic retries or qualification from missing controls are allowed. Keep a
source-commit/build/runtime record next to the manifest. Results distinguish
core paired service qualification from all-cell campaign completion. Stage
elapsed times overlap and must not be added or described as CPU measurements.
