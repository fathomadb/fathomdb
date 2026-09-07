# Slice 60 RED oracle lint correction

Date: 2026-09-07

An independent read-only audit confirmed three `agent-lint` diagnostics were
mechanical issues in the frozen RED tests:

- `slice60_wire.rs:82` and `:95` called `Option::map` with a unit-returning
  mutation closure (`clippy::option-map-unit-fn`);
- `slice60_graph_expand.rs:348` declared the duplicate-seed request binding as
  `mut` without mutating it (`unused_mut`).

Each `Option::map` expression was replaced with its exact `if let Some(slot)`
mutation equivalent, and only the unused `mut` token was removed. Mutation
targets, values, decoder calls, assertions, test names, fixtures, and public
intent are unchanged.

## Frozen-file hashes

| Path | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs` | `d2480f043b7ebdd55e8197d2bb60809be65b277ae6ac42c240eeeff0a1a108fb` | `b29aa071102fbcc80164599c34c0a286848e4a013a4bb0bdb1bc4b30587bdf48` |
| `src/rust/crates/fathomdb-engine/tests/slice60_wire.rs` | `0f09a9e3ecc625b32cf2e527cc5421405c9a8459268c31eb8faa264af78aa656` | `9f0626019fbdc5bc451d42cdcf97b98fc17218e2b49886f4bc4ce3ff0b60ee4f` |

This is the independently audited lint-only exception to RED-test
immutability. The shared fixture and the other four frozen Slice 60 test files
were not changed.
