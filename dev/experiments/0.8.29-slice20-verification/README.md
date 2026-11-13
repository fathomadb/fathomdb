# Slice20 verification resources

resource_guard.py reuses the committed Slice60 one-child/session monitor with
unchanged root/taskfs/RAM/swap/owned limits. It additionally reserves the full
attached64GiB image in every owned-byte sample and optionally counts the one
serialized sensitivity checkout. It never mutates or terminates other jobs.

```bash
S20_SOURCE=/mnt/data/worktrees/fathomdb-0.8.29-s20-approved-20261113 S20_TASK=/mnt/data/tmp2/fathomdb-0829-continuation/slice20 python3 dev/experiments/0.8.29-slice20-verification/resource_guard.py approved-PHASE COMMAND ARGS
```

Set S20_CONTROL_SOURCE to the parent-created control checkout once it exists.
All candidate/control targets and caches live inside S20_TASK and are counted
once. Run only one heavy command; reuse one private control target serially for
two stale-predicate and five constant mutations, with exact restored files between
cases. Do not run a full verifier on a negative candidate. Logs/receipts use the
selected phase under the owned task; all child TMPDIR/cache variables also point
there. The200GiB cap includes64GiB image, leaving136GiB for Linux inputs/outputs.


sensitivity.py implements the seven approved isolated negative controls and
records source mutations, unchanged assertions, actual executable digests and
raw logs. Set S20_CONTROL_SOURCE to the exact parent-created ef1f8d137 checkout;
it refuses a dirty/wrong base, runs one private control target and restores each
file in finally. Only an observed one-test runtime failure with the named oracle
qualifies; compiler/zero-test failures do not. No model/GPU or full negative gate.

```bash
python3 dev/experiments/0.8.29-slice20-verification/resource_guard.py approved-sensitivity python3 dev/experiments/0.8.29-slice20-verification/sensitivity.py
```

The monitor also retains read-only descriptors for canonical capped spill logs
before their normal successful deletion, using the existing20 collector method.
It saves complete passing native diagnostics/results; no runner output or exit
is changed. Collector failure remains a failure rather than a green claim.

verify_sensitivity_results.py is the retained consumer regression for case-result
loss after monitor completion. sensitivity.py writes approved-sensitivity-cases.json;
the monitor alone writes approved-sensitivity.json. recover_sensitivity.py recovers
the original seven records from retained raw output once, preserving the original
monitor receipt and all source/patch/artifact hashes. No passing control rerun.
The original executables arrays list Cargo-built artifacts; separate
runtime_attribution identifies the actual test executable via profile.test and
the exact Cargo Running line plus one-test harness output. Server artifacts are
never labeled executed merely because Cargo built them.

```bash
python3 dev/experiments/0.8.29-slice20-verification/recover_sensitivity.py
python3 dev/experiments/0.8.29-slice20-verification/verify_sensitivity_results.py
```
