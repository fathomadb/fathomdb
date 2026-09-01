---
title: 0.8.25 Slice 6 — HITL decisions
status: AWAITING_FINAL_PLAN_APPROVAL
target_release: 0.8.25
observed_on: 2026-09-01
---

# Slice 6 — HITL decisions

## Owner rulings

Decision authority is append-only ledger entries `seq-272` and `seq-273`.

The owner approved the recommended dispositions for P25-01 through P25-06 and
P25-08 through P25-26 at `seq-272`. The owner further ruled that P25-17 keeps
all runs and data, and that P25-20 stays narrow while still completing the
maintained-index corrections that are needed.

After reviewing the focused dependency evidence, the owner ruled P25-07 at
`seq-273`: include the test-only `httpmock 0.7.0 -> 0.8.3` upgrade in Slice 7,
preserve the existing loader tests, prove `async-std` disappears, and stop if
product pins move or the current mock APIs prove incompatible.

## Recorded dispositions

| Register IDs | Owner disposition | Effect |
| --- | --- | --- |
| P25-01, P25-04–P25-07, P25-11–P25-16, P25-18–P25-20, P25-23, P25-24 | **Accepted for Slice 7 planning.** | These are the only candidate implementation packages; the detailed plan still requires review and final owner approval. |
| P25-02, P25-21 | **Retain/no change.** | Preserve the npm release pin and intentional historical/source/fixture/local-artifact policies. |
| P25-03 | **Accepted boundary.** | Windows CPU/native parity remains feature-local; Windows CUDA is postponed. |
| P25-08–P25-10 | **Postponed.** | Do not change the Candle stack, Pyright, or Ruff in Slice 7. |
| P25-17 | **Retain/no change, explicitly strengthened.** | Keep all `runs/` data and experiment/performance evidence. No deletion, pruning, or bulk path rewrite is authorized. |
| P25-20 | **Accepted narrowly.** | Correct only maintained current-authority index rows needed by the accepted documentation/architecture changes; no broad index rewrite. |
| P25-22, P25-25, P25-26 | **Accepted as feature scope.** | Product work remains in Slices 10–75 and is prohibited from Slice 7. |
| P25-07 | **Accepted with stop conditions.** | Upgrade only test-only `httpmock` to 0.8.3; preserve tests, prove `async-std` is absent, and stop on product-pin movement or API incompatibility. |

## Boundaries

- This ruling authorizes planning, not Slice 7 implementation.
- Slice 7 remains blocked until its approved-only plan is independently
  reviewed and the owner approves that final plan.
- No product implementation, publication, tag, registry mutation, hosted
  workflow, Windows CUDA work, broad cleanup, or historical-data deletion is
  authorized.

## P25-07 additional evidence

### Current path

- `fathomdb-embedder` declares `httpmock = "0.7"` only under
  `[dev-dependencies]`.
- Cargo resolves `httpmock 0.7.0`, which pulls `async-std 1.13.2` through both
  `httpmock` and `async-object-pool 0.1.5`.
- The dependency is used only by
  `src/rust/crates/fathomdb-embedder/tests/loader.rs`. It starts local mock
  servers for the loader's checksum, resume, concurrency, authentication, and
  cache behavior; it is not present in the FathomDB runtime contract.

### Available correction

The upstream `httpmock` 0.8.0 release explicitly replaced `async-std` with
Tokio, upgraded `async-object-pool` to remove `async-std`, and stated that no
breaking changes were expected. Current `httpmock` 0.8.3 requires Rust 1.88;
the 0.8.25 environment uses Rust 1.95. Its current manifest has no
`async-std` dependency.

Primary sources:

- <https://github.com/httpmock/httpmock/blob/master/CHANGELOG.md>
- <https://github.com/httpmock/httpmock/releases/tag/v0.8.3>
- <https://github.com/httpmock/httpmock/blob/v0.8.3/Cargo.toml>
- <https://github.com/async-rs/async-std/releases/tag/v1.13.1>

### Options

1. **Include `httpmock 0.7.0 → 0.8.3` in Slice 7 (recommended).** This is a
   test-only dependency upgrade. Preserve the loader tests as the oracle,
   require a RED dependency-policy fixture showing the old `async-std` path,
   update the manifest/lock, prove `cargo tree` contains no `async-std`, and
   run the loader tests plus workspace verification. Stop if the existing mock
   API is not source-compatible or lock resolution crosses product pins.
2. **Postpone.** Keep the discontinued dependency in the test graph and record
   it as known repository debt. Product artifacts are not directly exposed,
   but security/advisory verification remains noisy and the migration will
   still be owed.
3. **Replace `httpmock` with another server or custom fixture.** This removes
   the dependency choice entirely but rewrites a large, concurrency-sensitive
   test oracle. The extra risk and effort are not justified by current
   evidence.

The correction is reversible and has no publication or runtime behavior
effect. The owner selected option 1 at `seq-273`; no proposal-register decision
remains open. The reviewed Slice 7 plan still requires the final interactive
approval required by the Slice 6 contract.
