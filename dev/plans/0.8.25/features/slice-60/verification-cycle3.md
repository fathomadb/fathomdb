---
title: 0.8.25 Slice 60 independent verification — cycle 3
status: FAIL
candidate: 0d64230f5cd3550dfa5014908359bc59eb77eff4
---

# Slice 60 independent verification — cycle 3

## Verdict

**FAIL.** The required complete CI-equivalent route reproducibly fails the
AGENT_LONG AC-059b cursor-race fixture's fixed 30-second wall-clock fuse. The
full `scripts/check.sh` run reached iteration 989 of 1,000; the one unchanged
isolated reproduction reached iteration 962. Neither run reported a cursor
invariant violation, hang, or deadlock.

The fixture and deadline predate Slice 60. Closure requires preserving its
1,000-search race oracle while making the required route complete within an
evidence-based bounded operational fuse. Skipping, waiving, or substituting a
shorter fixture is not permitted.

## Exact failure

```text
thread 'projection_cursor_bounds_observed_row_count' panicked at
src/rust/crates/fathomdb-engine/tests/cursor_read_after_write.rs:79:13:
cursor invariant test exceeded 30 s wall clock at iteration 989
```

The focused command was:

```text
AGENT_LONG=1 cargo test -p fathomdb-engine \
  --test cursor_read_after_write \
  projection_cursor_bounds_observed_row_count \
  -- --exact --nocapture --test-threads=1
```

It failed at the same 30-second fuse at iteration 962. Per the same-failure
retry rule, no third unchanged attempt was made.

## Passing evidence before the blocker

- Verification FIX-7 and FIX-8 hashes, diffs, and behavior were independently
  confirmed.
- Affected FIX-8 targets passed 20 tests with one intentional long-test
  ignore.
- Both Slice 60 feature matrices passed 50/50; compatibility passed 110 tests
  with one documented performance ignore; facade passed 4/4.
- Fresh Linux installed verification passed Python 24/24 from wheel
  `0027ad35da578a0adf791f0ada91509f67cf9b97b4d1f8151d2da15e69fafab9`
  and Node 15/15 from N-API module
  `44e4a2bc6737af96e2d4327a4814d0a5bd1519fe493a2e356823b18a700943fa`.
- Rustfmt, default full-workspace strict Clippy/check, selected-feature strict
  Clippy/check, and agent fast/heavy/all tiers passed.
- Dense-arm refusal passed, retaining GPU/CUDA as inapplicable.

Literal workspace `--all-features` is not an applicable Linux configuration:
it simultaneously selects Apple-only `objc2` and CUDA requiring `nvcc`. The
selected applicable feature profile passed strictly.

The failed complete route stopped the parallel reporter and Windows matrix
fail-closed.

## Cleanup and final state

The verifier removed its exact Linux wheel, virtual environment, TypeScript
output, and N-API fixture. No verifier process remained. The worktree stayed
clean at the exact candidate. Free disk was about 123 GiB; the approximately
49 GiB mixed durable Cargo target remained because exclusive ownership was not
proven. No ref, push, merge, release package, registry, tag, or publication
action occurred.
