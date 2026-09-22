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

## GREEN tool contract

The repository tool then implemented the common normalized row model, all six
Rust feature identities, independent Python export/registration/stub adapters,
NAPI and TypeScript declaration adapters, package metadata/runtime exports,
metadata fail-closed comparison, duplicate rejection, and immutable compare.
The focused contract passed:

```text
ok    slice30-surface-comparator
```

The real heavy capture and reviewed baseline remained a separate next step so
the capture-source commit could be clean and contain the tool but no baseline.

## Real-output parser RED

The first real Rust matrix exposed a legitimate `cargo public-api` shape that
the minimal fixture lacked: distinct blanket impls can emit the same associated
type path with different signatures. A focused fixture reproduced the failure:

```text
SurfaceError: duplicate normalized key in rust-engine-default:
type fathomdb_engine::Thing::Error
```

Exact duplicate lines must still fail closed; distinct signatures sharing a
display path require deterministic disambiguation.
