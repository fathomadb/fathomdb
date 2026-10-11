---
title: Slice 135 importance and confidence lookup error repair
status: CONFIRMED_DEFECT_REPAIRED_FOCUSED
target_release: 0.8.27
---

# Importance and confidence lookup error repair

`build_importance_confidence_maps` ignored statement and row failures. On
current schema, that can turn a failed stored-value lookup into an apparently
absent importance or confidence value. Explained search then reports a
neutral value; the opt-in reweight path can silently change ranking.

The [real-database test](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_importance_explanation_row_error.rs)
first asserts that a valid stored importance value of `0.5` appears in a
node's explanation. With the engine open, it changes that SQLite column to
an undecodable BLOB and requires `EngineError::Storage`. The previous code
returned success ([RED stdout](red.stdout), [stderr](red.stderr), exit 101).
The repaired lookup distinguishes an absent row or NULL from SQL and decode
errors, propagating the latter through both graph and ordinary fusion paths.
The focused GREEN run includes 10 existing importance tests, this regression,
the edge explanation regression and all three rank-stream cases: 15 passed,
zero failed ([stdout](green.stdout), [stderr](green.stderr)). The new target
also compiled with default features and ran zero tests without `test-hooks`.
Focused all-target engine Clippy with `test-hooks,default-embedder` and denied
warnings passed.

The same fallible lookup logic handles edge confidence, but this receipt
injects an importance decode error only. It does not establish all provider,
FFI or persistent-fault paths. The current source file SHA-256 values are
`fusion.rs` `f1f8fd091d3f8fa915839549047cfb06bb7d02c04fbb915845e474d2e3f77095`
and `search.rs` `170c07b70df7e8a2c9a4075915f82fcdb07dc761a5533c86e08bed57d306bef0`;
the test hash is
`ca62aa6e60ae3b70cd9f940cfc232bb4f71eeb7f4254c44f09e6393e8bb8bb80`.
This changes candidate product bytes again. Older latency and coverage
receipts remain valid for their exact source identities but require a
final-candidate refresh for the Phase 1 checkpoint. The full repository gate
has not been rerun.
