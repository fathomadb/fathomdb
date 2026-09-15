# Slice 30 TDD chronology

## Planned RED

Commit `3834f22a` added the process-level immutable inspection contract before
the production boundary existed. The first test draft used a nonexistent exit
constant and was corrected before it could be a behavioral oracle. The valid
RED command was:

```text
cargo test -p fathomdb-cli --test slice30_operator_integrity_cli -- --test-threads=1 --nocapture
```

One existing semantic-request case passed at the parser boundary; the other
eight cases failed because the CLI still used ordinary `Engine::open`. The RED
covered clean and findings runs, held lock, non-empty WAL and journal,
persistent `-shm`, lower/higher schema versions, missing input, corruption, V1
errors/exits/privacy, and byte/path identity for the complete product file set.

## GREEN and immutable correction

Commit `0fa51f05` added the operator-gated free inspection function, reused the
existing V1 executor, and routed only `doctor data-plane-integrity` through the
new boundary. Its first GREEN run passed five cases but failed four
no-mutation cases: SQLite read-only mode still created or altered shared-memory
state.

The implementation switched to a percent-encoded SQLite `file:` URI with
`immutable=1`, `READ_ONLY|URI`, and `query_only`. The nine-case suite then
passed. The same commit keeps request validation ahead of filesystem access,
opens and locks the existing `.lock` read-only without rewriting it, refuses
non-empty WAL/rollback journals, configures SQLite before extension
registration, requires compiled schema 33 exactly, and maps post-parse failures
to the privacy-safe V1 envelope.

The refactor added direct compatible-runtime coverage and an existing-database
missing-lock case, reaching 10/10. Design re-review then required explicit URI
contract propagation and a reserved-path oracle. Commit `4cbb8218` added the
cross-platform `reserved space#%3F.sqlite` no-mutation case and aligned the
plan, design, interface, public API contract, operator guide, and future
post-publication smoke. The suite passed 11/11. Commit `0794bcf0` separately
closed an operator-off unused-import warning exposed by a warnings-denied
facade check.

## Code-review RED/GREEN

Independent review found that decoding `PRAGMA user_version` directly as
`u32` classified SQLite's valid negative values as `integrity_corrupt` rather
than an exact-schema mismatch. Commit `e267f652` changed the schema oracle to
test exact signed values `32`, `34`, and `-1`. The focused RED reproduced:

```text
left: String("integrity_corrupt")
right: "database_schema_mismatch"
```

Commit `067d74b4` reads the pragma as `i64` and compares it with
`i64::from(SCHEMA_VERSION)`. Slice 30 process tests passed 11/11, legacy CLI
tests passed 3/3, and the independent reviewer returned PASS with no remaining
P0-P2 findings.

## Distribution witness

A clean temporary root installed the source candidate with:

```text
cargo install --path src/rust/crates/fathomdb-cli \
  --root /tmp/fathomdb-slice30-candidate.UCJPmr --locked
```

The first sandboxed attempt could not resolve the crates.io index; the same
command was rerun with approved network access and completed. The installed
binary reported `fathomdb 0.8.25`, which correctly identifies the current
pre-integration workspace package version. It created a clean fixture through
the legacy check and returned a clean DPI V1 report. Slice 50 owns the
integrated 0.8.26 candidate; only the post-publication smoke can prove the
crates.io 0.8.26 artifact.
