---
title: Slice 135 source-bound external Rust SDK S02 functional feasibility
status: CANDIDATE_ONLY_EXTERNAL_CARGO_FUNCTIONAL_NO_LATENCY_VERDICT
target_release: 0.8.27
---

# External Rust SDK S02 functional feasibility — 2026-10-07

A separate Cargo consumer outside the workspace compiled `fathomdb-sdk` and
its local sibling crates from candidate source
`febc62b5e792937d5312c0d7b8b97db963917784`. The package version string
is still 0.8.26; the candidate workspace `Cargo.lock` SHA-256 is
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
The retained [external manifest snapshot](Cargo.toml.snapshot), [external lockfile](Cargo.lock),
[dependency tree](cargo-tree.txt), [consumer source](main.rs),
[compiled executable](consumer.bin), [build output](build.stderr) and
[Rust toolchain version](rustc-version.txt) identify this source-bound run.
The external lockfile was seeded from the candidate workspace lockfile to
keep the build offline, then Cargo added the consumer's dependencies. The
first unpinned offline attempt selected an uncached newer crate and did not
run the product. The successful command used `cargo build --offline --locked
--release --manifest-path /tmp/slice135-rust-sdk-consumer/Cargo.toml` with
`CARGO_TARGET_DIR` pointing to the dedicated Slice 135 worktree target.

This is **not a published-crate installation**. The exact 0.8.26 baseline has
no `fathomdb-sdk`, so this Rust SDK result is candidate-only; comparable
engine cells are paired separately. The external manifest includes absolute
worktree paths and needs those paths substituted to rerun on another host.

The executable used a fresh [real SQLite database](consumer.sqlite) with the
default embedder. It wrote the shared 32-record S01 corpus, one canonical
source, two derived nodes and a provenanced `supports` edge. It configured a
searchable vector projection, drained, asserted `ready` and the retained
anchor, then checked text, vector-bearing and hybrid retrieval. It checked
two typed error paths: a zero search limit and an embedded-NUL text query.
It froze a read context, resolved ordinary and graph evidence back to the
canonical source, erased the graph source, checked idempotence and reopened.
The [raw stdout](stdout.log), [empty stderr](stderr.log) and
[resource report](resource.txt) come from a direct executable invocation
bounded by `timeout 120s`.

An independent SQLite query found **0 graph nodes, 0 graph edges and 32
retained corpus nodes** after close/reopen. The [state result](independent-state.json)
and [audit](audit.py) independently check the retained database and receipt
hashes; the [audit output](audit.json) also shows that a retained-edge
negative control is rejected. The [SHA-256 manifest](SHA256SUMS) seals all
retained files. This is one functional candidate run, with no paired Rust SDK
baseline, contention condition, latency sampling or published-artifact claim.
It does not close the rest of the Rust capability map or the Phase 1 checkpoint.
