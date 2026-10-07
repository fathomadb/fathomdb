---
title: Slice 135 off-ladder embedder-close focused qualification
status: PARTIAL_PHASE1_CHECK
target_release: 0.8.27
---

# Corrected embedder-close landing — focused Phase 1 check

The off-ladder merge `96796fe04` is an ancestor of candidate
`ba56f2e2f1209c23bb98da2214bfaa6c3265f0de`. The current candidate's
`Cargo.toml`, `Cargo.lock`, `src/rust` and `src/python` bytes match the
installed Python timing product source `8587b0571f4e6477045e60ddd341b0cd28f5a3cb`.
The merge added the ownership/lock behavior and focused regression tests for
embedder release after close. This is a local source/binary check, not a
final-release artifact qualification.

| Focused command from `src/rust/` | Result | Log |
| --- | --- | --- |
| `cargo test -p fathomdb-engine --test projection_runtime close_ -- --nocapture` | Three real-database close ownership tests passed; 14 filtered | [projection-close.log](projection-close.log) |
| `cargo test -p fathomdb-engine --lib timed_out_close_retains_provider_until_later_close_releases_it -- --nocapture` | Timed-out close retained the provider until later close; one passed, 96 filtered | [timed-out-close.log](timed-out-close.log) |

The first test binary was SHA-256
`6072fae53646b36b2abeb372ea07eb527fffa504273820f4fbc3ecd7b7a33a10`;
the second was
`f40e32f984346ebb0e3da1bc67ff39e5596f2916e7a7a3e7c28ea8094816d038`.
The initial unscoped `cargo test -p fathomdb-engine <filter>` attempted to
link unrelated integration targets and exhausted local disk; its
[invalid attempt](broad-target-invalid.log) is retained. Only the two
explicitly scoped reruns count as passing observations. Deleting this
worktree's generated Rust incremental cache restored space; no source or
test file was changed.

This check covers last-reference release outside the close lock, a retained
caller-owned embedder, and timeout/retry ownership. Pending embedding and
projection cancellation/failure under concurrent close, installed binding
behavior, and final-candidate lifecycle are still owed by Slice 135/150 as
specified in the [off-ladder handoff](../../off-ladder-qualification-handoff.md).
The other off-ladder Tegra Pages landing `c23e2d23f` is still not resolvable
in this local Git database; its ancestry and exact affected files remain
unverified here.
