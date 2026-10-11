---
title: Slice 135 exact-source external Rust SDK capability exercise
status: AUDITED_SOURCE_BOUND_CANDIDATE_ONLY
target_release: 0.8.27
---

# External Rust SDK consumer at source 224e44c59

This separate Cargo consumer built offline against the exact candidate source
`224e44c593c13d86ece648adabe445723db04070` and its local `fathomdb-sdk`
path dependency. It is **not a published-crate installation**, and there is
no 0.8.26 Rust SDK peer. The build exited zero; the compiled binary SHA-256
is recorded in [binary-sha256.txt](binary-sha256.txt), and the manifest,
lockfile, dependency tree, toolchain and consumer source are retained.

The consumer's [operation register](operations.json) accounts for all 44
governed operations: **42 selected calls executed, no supported gaps, two
provider/model cases unavailable**. The standalone `rerank` identity path
ran; a cross-encoder model was not qualified. The [stdout](stdout.log)
records one marker after each assertion and the final completion marker.
Stderr is empty. The consumer exercised actuation, dependencies, closure,
search and evidence, pagination, projections, erasure, close and reopen on a
retained real SQLite [database](consumer.sqlite).

The [independent audit](audit.json), replayed by [audit.py](audit.py),
checked exact operation accounting, output markers, source and dependency
identity, and reopened canonical and operational SQLite state. The four
auditor tests passed, including missing-operation and retained-edge negative
controls. The [SHA256SUMS](SHA256SUMS) manifest has SHA-256
`8433aed6d93adca4a580875b4fdb73f0a1b58df1061f67c43ae96e05aee956be`.

These are selected functional calls, not exhaustive contract conditions,
installed-package timing or other-platform qualification. The Rust SDK
result is candidate-only; comparable engine behavior has its separate
paired 0.8.26 result.
