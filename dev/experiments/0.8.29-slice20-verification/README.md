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
