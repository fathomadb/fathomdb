# Slice 45 RED evidence

## Public Engine pagination contract

Command:

```text
cargo test -p fathomdb-engine --test slice45_pagination --no-run
```

Exit: `101`

First diagnostic (verbatim):

```text
error[E0432]: unresolved imports `fathomdb_engine::PageErrorReason`, `fathomdb_engine::PageRequestV1`
 --> src/rust/crates/fathomdb-engine/tests/slice45_pagination.rs:5:63
  |
5 | ...te, PageErrorReason, PageRequestV1,
  |        ^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^ no `PageRequestV1` in the root
  |        |
  |        no `PageErrorReason` in the root
```

The same compile reported the intentionally absent `PageCursor`,
`EngineError::Page`, `read_canonical_page`, `read_operational_state`, and
`read_operational_state_page` contract surfaces.

## Schema step 33

Command:

```text
cargo test -p fathomdb-schema --test slice45_step33 -- --nocapture
```

Exit: `101`

Diagnostics (verbatim):

```text
thread 'step33_installs_unique_page_indexes_and_state_visibility_triggers' (94) panicked at src/rust/crates/fathomdb-schema/tests/slice45_step33.rs:6:5:
assertion `left == right` failed
  left: 32
 right: 33

thread 'step33_refuses_duplicate_legacy_page_keys_atomically' (95) panicked at src/rust/crates/fathomdb-schema/tests/slice45_step33.rs:44:5:
assertion failed: migrate(&connection).is_err()
```

These are the expected RED failures. Product code and schema were unchanged
when they were observed.
