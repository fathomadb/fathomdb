---
title: 0.8.25 Slice 60 independent verification — cycle 5
status: FAIL
candidate: dc488bc7b210a9ebe65499615311b83af71c9266
---

# Slice 60 independent verification — cycle 5

## Verdict

**FAIL.** The first corrected Windows run removed the invalid non-Linux RSS
witness, then exposed a test-only lifecycle oracle defect. Production behaved
as specified: source rows were durably deleted, but the at-rest WAL truncation
remained BUSY and returned the typed fail-closed result rather than falsely
reporting success.

## Exact evidence

The exact-source archive SHA-256 was:

```text
6e8cc0541f2a5e89302808340f32f9a309275669bceb24ee5efeaee363369e1e
```

The Windows Rust route passed 15 tests before this exact failure:

```text
slice60_fix3_runtime::graph_expand_executes_erase_and_excise_disappearance

ErasureIncomplete {
  stage: "wal_checkpoint",
  detail: "`excise_source` deleted its rows, but
  `wal_checkpoint(TRUNCATE)` reported BUSY on all 5 attempts
  (318 frames still in the log)"
}
```

No stale Cargo or test process was active. One exact unchanged isolated
reproduction passed 1/1 with two filtered in 0.22 seconds. The outcome and
holder remain unattributed.

Independent review classified the `unwrap()` as a P2 oracle defect. The
fixture's purpose is committed graph disappearance, while the public lifecycle
contract explicitly allows a typed WAL-checkpoint refusal after deletion. The
test-only correction accepts only success or that exact refusal, rejects every
other error or stage, and retains both real disappearance assertions.

The Windows run stopped before Python and Node, and its exact owned artifacts
were removed. No broad gate, registry, package promotion, tag, publication, or
merge ran.
