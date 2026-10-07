---
title: Slice 135 external Rust SDK capability exercise
status: SOURCE_BOUND_CANDIDATE_ONLY
target_release: 0.8.27
---

# External Rust SDK capability exercise — 2026-10-07

This separate Cargo package depends on the candidate `fathomdb-sdk` source and
local sibling crates by relative path. It is outside the Rust workspace and
uses a real temporary SQLite database with the default embedder. The candidate
base is in [source-commit.txt](source-commit.txt); the SDK source hashes,
external lockfile, dependency tree, Rust toolchain, consumer source and
compiled binary hash are retained here. The package version still reads
`0.8.26`. **This is a path dependency, not a published-crate installation.**
There is no 0.8.26 Rust SDK baseline to pair against this candidate.

The [canonical operation map](../../../../../../../src/conformance/governed-operation-parity.json)
has 44 live IDs. [operations.json](operations.json) accounts for each exactly
once: **42 selected cases executed, zero unattempted gaps, two unavailable
provider/model cases** (`engine.ingest_with_extractor` and
`engine.consolidate_with_provider`). The [raw stdout](stdout.log) records one
marker after each selected assertion; [stderr](stderr.log) is empty. The
standalone `rerank` identity path was exercised, while cross-encoder model
qualification remains unavailable. These are operation-level selected cases,
not exhaustive condition coverage or performance evidence.

The consumer checks actuation commit, identical replay, stale-boundary refusal
and persisted absence; reciprocal dependency lookups, a derived-to-source
trace, a committed closure's complete zero proof and reopened status; canonical
and operational pages against point reads plus malformed-cursor refusal;
projection, generation, embedding and mutation statuses; append-only
`read.collection`/`read.mutations` and latest-state reads; source-backed
evidence, graph evidence, erasure and reopen. The [retained database](consumer.sqlite)
is checked separately by [audit.py](audit.py), including a deliberately retained
edge negative control. [audit.json](audit.json) records hashes and independent
counts; [SHA256SUMS](SHA256SUMS) seals the retained receipt files.
After integration into the Slice 135 checkout, a replay test first found that
the auditor compared Cargo's recorded absolute build path to the current
checkout path. The auditor now verifies the relative manifest dependency,
the retained tree's SDK path suffix, and equality with the retained audit.
The replay test is included in the focused accounting suite below.

The successful run used the manifest now retained as
[Cargo.toml.snapshot](Cargo.toml.snapshot). Its historical invocation was:

```sh
CARGO_TARGET_DIR=/tmp/slice135-rust-sdk-capability-target cargo build --offline --locked --manifest-path dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability/Cargo.toml
timeout 180s /tmp/slice135-rust-sdk-capability-target/debug/slice135-rust-sdk-capability /tmp/slice135-rust-sdk-capability-formatted.sqlite
python3 dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability/audit.py
python3 -m unittest discover -s dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability -p test_audit.py
```

The archived snapshot is not a live Cargo manifest. To rebuild, materialize
those exact bytes as `Cargo.toml` in an isolated copy with the same relative
source layout, then use the recorded invocation. The receipt auditor reads
the snapshot directly.

The run's stdout, stderr and SQLite file were copied into this directory after
exit 0. The compiled executable was 260 MB, so its SHA-256 is retained in
[binary-sha256.txt](binary-sha256.txt) rather than copying it. The focused
build, rustfmt, Python lint, audit and accounting tests passed. The full
repository Markdown validator stopped on existing links to an untracked E12
archive absent from this isolated worktree; the links do resolve in the main
Slice 135 checkout. No full workspace gate was run.
