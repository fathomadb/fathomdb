# Slice 30 TDD chronology

## RED

The focused comparator contract was added before the repository tool existed.
It covers deterministic fixture capture, unchanged comparison, every required
surface mutation, combined Rust feature-row identity, metadata mismatch, and
duplicate-key rejection. The valid RED command was:

```text
python3 scripts/tests/test_slice30_surface_comparator.py
```

It exited 1 while loading the intentionally absent entry point:

```text
FileNotFoundError: [Errno 2] No such file or directory:
'/home/coreyt/projects/fathomdb-worktrees/release-0.8.27/dev/tools/surface_comparator.py'
```

The production tool and tracked baseline did not exist at this checkpoint.
