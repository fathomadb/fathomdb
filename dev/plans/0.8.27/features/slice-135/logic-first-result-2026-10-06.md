---
title: Slice 135 first logic and exception result — edge FTS row error
status: DIAGNOSTIC_RESULT_OPEN_DEFECT
target_release: 0.8.27
source_sha: 8cbd330c8f83f35ef46d538ea23811c08af8066b
---

# First logic and exception result — 2026-10-06

This is one inspectable **diagnostic** result on the exact post-Slice-132
candidate, not the completed Phase 1 logic/exception checkpoint. The package
version string remains 0.8.26 in this source; the full Git SHA above and
`Cargo.lock` SHA-256
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`
identify the source used. The focused run finished 2026-10-07 02:40 UTC.

## Contract, path, and result

The audited path is `Engine::search_text_only_view_with_limit` → reader-pool
request → `read_search_in_tx` → edge-body FTS row decoding. It is a real
database path: [the Rust interface](../../../../interfaces/rust.md) exposes
`search_text_only` as `Result<SearchResult, EngineError>`, and
[the error design](../../../../design/errors.md) assigns physical SQLite
read failures to `StorageError`. The entry point converts
`SearchReaderError::Sqlite` to `EngineError::Storage` after emitting an internal
error event (`search_api.rs`, around line 1078). That handling is bypassed when
the reader drops a row error before constructing `SearchReaderError`.

The [bounded RED probe](logic-error-probe.rs) wrote two nodes and one
body-bearing edge to a real SQLite database, drained projection, and verified
that `search_text_only("slice135uniqueedgefact")` returned the edge. It then
changed only the edge FTS `kind` to invalid UTF-8 BLOB `x'ff'`. A direct FTS5
`MATCH` still found the row with `typeof(kind) = 'blob'`; FathomDB's independent
integrity check returned `SearchProjectionIdentityMismatch` for cursor 3.
The repeated search returned **`Ok` with an empty result list** instead of
`Err(EngineError::Storage)`. The final assertion failed as intended (exit 101).
The exact probe-source SHA-256 was
`4f7bba20d60405aae92cfae3e721f85f9db6f9cf5035560851446206957a4296`.
The [raw stdout](evidence/logic-first/slice135-logic-probe3.stdout) and
[raw stderr](evidence/logic-first/slice135-logic-probe3.stderr) contain the
exact trace and assertion. The
[SHA-256 manifest](evidence/logic-first/SHA256SUMS) covers the probe source and
retained raw artifacts.

The immediate cause is `rows.flatten().collect()` in the edge FTS branch of
`search.rs` around line 2079: `Result::Err` from `row.get::<_, String>(1)` is
discarded. The [measured line counts](evidence/logic-first/slice135-logic-search.lcov)
show the edge mapper entered twice (control and fault), while the line after
the `kind` decode was reached once. This fault is on a current-schema database;
the adjacent comment about tolerating a missing edge table on old schemas
does not explain suppressing a row-decode error on an existing table.

**Disposition:** confirmed logic/exception defect, open. A follow-up fix
should propagate row conversion errors through `SearchReaderError::Sqlite` and
keep any intentional old-schema fallback limited to its documented condition.
Use the RED probe before the fix; run the existing FTS rank-stream and edge
validity tests afterward. This diagnostic does not change product code or
claim a release verdict.

## Coverage and static audit

The focused probe was built with Rust coverage instrumentation; timing from
this build is not a latency measurement. The stable instrumentation run's
[LCOV export](evidence/logic-first/slice135-logic-search.lcov)
independently records `LF:1500`, `LH:257`, `DA:2061,2`, `DA:2069,2`,
`DA:2070,1`, and `DA:2079,2`. It records `BRF:0`, `BRH:0`: this run did not
provide branch coverage. The matching LLVM JSON export, with no
`--skip-branches` flag, also reports zero branch entries for `search.rs` and
zero branches across the executable; the
[extracted branch totals](evidence/logic-first/slice135-logic-branch-counts.json)
and [text report](evidence/logic-first/slice135-logic-coverage-report.txt)
retain that tool evidence.

A second build used Rust 1.95.0's **unstable** branch option via
`RUSTC_BOOTSTRAP=1` and
`RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`. On the same
RED probe, `llvm-cov report` measured **30/236 branches (12.71%)**,
**257/1,500 lines (17.13%)**, and 366/2,421 regions (15.12%) in `search.rs`.
The [branch LCOV](evidence/logic-first/slice135-logic-branch-search.lcov)
records `BRF:236`, `BRH:30`; the
[raw text report](evidence/logic-first/slice135-logic-branch-report.txt)
agrees. This is measured branch coverage for one fault probe, not the test
suite or Pareto-path overlay. The unstable flag is diagnostic only; no product
build or release gate depends on it.

The [compressed raw profile](evidence/logic-first/slice135-logic-run3.profraw.gz)
is retained; its uncompressed SHA-256 is
`4af660e7b7c7755151a429aed1d8029eb8f37eb4d91b650280b5cef1e1d936c5`.
The [branch-run profile](evidence/logic-first/slice135-logic-branch-run.profraw.gz)
has uncompressed SHA-256
`f83eacd43b2fdc1cf01db4e53b89ebc4737dcacc597b27493efc294d06f6bc90`.

The source audit found seven `rows.flatten()` sites in `search.rs` at lines
1246, 1544, 1947, 1986, 2010, 2079, and 2113. Only the edge FTS case above
was dynamically checked. The other six remain investigation targets; some
sit in explicit legacy or rank-stream fallback code, so their intended error
semantics must be checked individually. The one `filter.expect` in the search
body (line 2117) is guarded by the attribute-filter condition; this review
found no reachable panic through the probed request. `read_search_in_tx`
commits its read transaction after result assembly; early `?` exits drop the
transaction. Handle-release behavior was not measured here.

At the Python boundary, `search_text_only` calls `call_engine`, which catches
Rust unwind and maps engine errors to typed Python exceptions. The TypeScript
binding calls its `spawn_blocking` `call_engine`, which also catches unwind and
maps join failures. These are source-review observations, not panic-injection
results. The Python error translator has `let _ = setattr(...)` payload
attachments, another possible swallowed-error class outside this probe's
search path; it needs a separate behavioral check before classification.

The existing `./scripts/agent-lint.sh` gate passed unconfined on this exact
source SHA in the primary Slice 135 checkout; that output was reported by the
main agent and is not reproduced by this focused run. Semgrep was not installed
on this host. The existing Rust Clippy gate and this targeted source audit gave
the first signal without adding a new static-analysis policy.

## Reproduction and evidence limits

From the checkout at the source SHA above, copy `logic-error-probe.rs` to
`src/rust/crates/fathomdb-engine/tests/slice135_logic_probe.rs`, then run:

```sh
CARGO_TARGET_DIR=/tmp/slice135-logic-cov \
  RUSTFLAGS='-C instrument-coverage' \
  LLVM_PROFILE_FILE='/tmp/slice135-logic-build-%p-%m.profraw' \
  cargo test --locked --offline -p fathomdb-engine --features operator \
  --test slice135_logic_probe --no-run --message-format=json

LLVM_PROFILE_FILE='/tmp/slice135-logic-run3-%p-%m.profraw' \
  /tmp/slice135-logic-cov/debug/deps/slice135_logic_probe-a7c5b093c411e288 \
  --nocapture
```

The built test executable SHA-256 was
`d46e2c30adfa9a6b72eca0688bcc07028256a96f9fcad87e1e9cfb0197d0a80d`.
For the separate branch-coverage diagnostic, the build and test commands were:

```sh
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=/tmp/slice135-logic-branchcov \
  RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' \
  LLVM_PROFILE_FILE='/tmp/slice135-logic-branch-build-%p-%m.profraw' \
  cargo test --locked --offline -p fathomdb-engine --features operator \
  --test slice135_logic_probe --no-run --message-format=json

LLVM_PROFILE_FILE='/tmp/slice135-logic-branch-run-%p-%m.profraw' \
  /tmp/slice135-logic-branchcov/debug/deps/slice135_logic_probe-aca8145a2e21cc44 \
  --nocapture
```

The branch-build executable SHA-256 was
`47ea0b7e20ad292fc84c68e37c79557f711c83850fc4c4131660807ed7adf006`.
The [build stderr](evidence/logic-first/slice135-logic-branch-build.stderr),
[RED stdout](evidence/logic-first/slice135-logic-branch-probe.stdout), and
[RED stderr](evidence/logic-first/slice135-logic-branch-probe.stderr) retain
the exact second run. It reproduced the same defect and exited 101.

Tool versions: `rustc`/Cargo 1.95.0, matching LLVM 22.1.2-rust-1.95.0-stable,
Python 3.12.3 and its SQLite 3.45.1 for the independent fixture feasibility
check. The engine's SQLite runtime version was not captured. The
[build stderr](evidence/logic-first/slice135-logic-build.stderr)
retains LLVM profile-write warnings from instrumented build helpers running
in a read-only checkout; the `cargo test --no-run` command exited 0 and the
test executable wrote the specified `/tmp` profile during the diagnostic run.

Two earlier attempts are retained as invalid attempts: the
[first](evidence/logic-first/slice135-logic-probe.stderr) omitted a caller
embedder and failed at the edge write with `EmbedderRequired`; the
[second](evidence/logic-first/slice135-logic-probe2.stderr) used a zero BLOB,
which was valid UTF-8 and did not trigger the target row decoder. Neither is
used as defect or coverage evidence.

The probe used a debug, `operator`-enabled engine with `NoopEmbedder` to
exercise FTS and the read-only integrity oracle. It tested one corruption
shape and one query, not arbitrary SQL faults, concurrent queries, release
artifacts, a full static-analysis sweep, or the eventual final candidate.
