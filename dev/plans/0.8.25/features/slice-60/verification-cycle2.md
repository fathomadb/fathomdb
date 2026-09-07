---
title: 0.8.25 Slice 60 independent verification — cycle 2
status: FAIL
candidate: 654d2687c9577f9f7a34d258b6f0d4acf75d8ef2
---

# Slice 60 independent verification — cycle 2

## Verdict

**FAIL.** The exact clean candidate reproducibly fails the mandatory
applicable-feature strict-Clippy gate. The private `default-reranker`
`Singleton` enum stores a `CandleTinyBertReranker` inline, making its loaded
variant at least 1,520 bytes while the next-largest variant is 24 bytes.
Clippy 1.95.0 rejects this under `-D warnings` as `large_enum_variant`.

The defect predates Slice 60 at `f748a47d3`, but the Slice 60 design selects
the default-embedder/default-reranker feature profile and requires its strict
gate. The minimum owner is the function-private enum in
`src/rust/crates/fathomdb-engine/src/lib.rs`. Boxing only the loaded model is
the narrow indicated correction; suppressing the lint would leave the
diagnosed layout defect in place.

## Exact diagnostic

Both the selected feature command and a narrow unchanged reproduction failed:

```text
$ cargo clippy -p fathomdb-engine --features test-hooks,operator,default-embedder,default-reranker --all-targets -- -D warnings
$ cargo clippy -p fathomdb-engine --features default-reranker --lib -- -D warnings
error: large size difference between variants
     --> src/rust/crates/fathomdb-engine/src/lib.rs:17104:5
      |
17104 | /     enum Singleton {
17105 | |         Loaded(fathomdb_embedder::CandleTinyBertReranker),
      | |         ------------------------------------------------- the largest variant contains at least 1520 bytes
17106 | |         Unavailable,
17107 | |         DevicePolicy(RerankerDevicePolicyError),
      | |         --------------------------------------- the second-largest variant contains at least 24 bytes
17108 | |     }
      | |_____^
      |
      = note: `-D clippy::large-enum-variant` implied by `-D warnings`
help: consider boxing the large fields or introducing indirection in some other way to reduce the total size of the enum
```

The environment used `rustc 1.95.0` and `clippy 0.1.95`.

## Passing evidence before the blocker

- All 15 Slice 60 engine binaries passed 50/50 under `test-hooks,operator` and
  50/50 again with `default-embedder,default-reranker` added.
- Slice 20/35/55 compatibility passed 110 tests; one documented Slice 55
  release-performance test remained intentionally ignored.
- Slice 55/60 facade tests passed 4/4.
- The projection correction passed, AC-050a passed for Rust, Python, and
  TypeScript, and the old source alias was absent.
- Default relevant-crate strict Clippy, Rustfmt, release-state views,
  Markdown/design/reference gates, and `git diff --check` passed.
- The applicable-feature query-seed refusal proved that graph expansion does
  not enter the dense arm, so GPU/CUDA remains inapplicable.

Fresh Linux installed artifacts, fast/heavy/all, complete `check.sh`, parallel
reporting, and Windows verification were stopped fail-closed. No artifact was
created or copied to Windows.

## Cleanup and final state

No verifier-owned temporary artifact or process remained. The worktree stayed
clean at the exact candidate. Free disk moved from about 150 GiB to 149 GiB;
the reusable mixed Cargo target grew from 24 GiB to 25 GiB and was retained
because exclusive ownership was not proven. No ref, push, merge, release
package, registry, tag, or publication action occurred.
