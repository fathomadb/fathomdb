---
title: 0.8.25 Slice 50 implementation TDD chronology
status: RED
---

# Slice 50 implementation TDD chronology

## RED 1

The first product test commit adds real-database tests for:

- exact UTF-8 source-span resolution and positional sidecar association;
- identical non-disclosure for reference tamper and context mismatch; and
- keyed commitments that cannot be matched as plaintext or raw SHA-256
  dictionary candidates.

The focused command was:

```text
cargo test -p fathomdb-engine --test slice50_evidence
```

It failed before any product implementation with Rust diagnostics for the
missing evidence request/result/lifecycle/error types, `EvidenceRefV1`,
`Engine::search_with_evidence`, `Engine::resolve_evidence`, and the corresponding
`EngineError` variant. This is the intended RED boundary.

The test file is frozen during GREEN corrections except when a later reviewed
requirement adds a new test; existing assertions will not be weakened to make
the implementation pass.

## GREEN 1

The first implementation increment added the public Rust evidence types, a
canonical authenticated reference codec with field-domain keyed commitments,
exact provenance loading, and the two Engine operations. The focused test now
passes 3/3 with no warning.

This increment is not the slice-completion claim. Later RED increments cover
same-reader-snapshot construction, graph-origin capture and reauthorization,
artifact/source eligibility, lifecycle and corruption matrices, codec
properties, default-path invariance, and cross-SDK parity.

## RED 2

The second product test increment adds two reviewed architecture boundaries:

- a graph-arm result must resolve the reached node's canonical source while
  separately naming the exact traversed edge revision; and
- canonical source bytes must independently satisfy access-bearing attribute
  terms from the frozen search filter.

The focused command failed 3 passed / 2 failed. Graph evidence returned
`evidence_incomplete` at `/results/1/graphOrigin`, while source resolution
incorrectly disclosed a source whose `owner=bob` did not satisfy the frozen
`owner=alice` predicate. These are the intended RED failures.

## GREEN 2

The second implementation increment introduced a distinct evidence request on
the owned reader pool and a monomorphized capture strategy shared by the search
algorithm. The no-evidence strategy is zero-sized and records nothing; the
evidence strategy captures the winning BFS edge cursor, constructs references,
and validates the frozen snapshot before the same reader transaction commits.

Resolution now independently applies access-bearing eligibility to canonical
source bytes, verifies that a graph origin is an active exact edge revision
connected to the returned node, and discloses the reached node's source rather
than substituting the edge's source. The focused suite passes 5/5.

A parallel engine-lib run encountered interference between existing
WAL-attribution concurrency tests and was stopped after a second test stalled.
The named failing test passed immediately in exact isolation; this is not used
as a broad-regression green claim. A serialized package run remains required.

## RED 3

The third test increment fixes the reference lifetime boundary. It requires an
equivalently reminted context to resolve after restart and after the originating
projection generation becomes retired, while separately proving that a
superseded artifact is non-disclosing.

The focused command failed 6 passed / 1 failed: the resolver compared the
reference only with the current serving generation and therefore rejected the
still-present retired origin. The supersession case already returned the
required `evidence_unavailable` outcome.

## GREEN 3

Resolution now authenticates the committed originating generation against the
bounded Engine-owned generation history rather than requiring it to remain the
current serving generation. It rejects zero or duplicate matches and validates
the closed generation-ID grammar before disclosure. The focused suite passes
7/7, including restart, retirement, and supersession.

## RED 4

The fourth test increment requires a derived result to return its existing
zero-or-one Slice 20 source dependency. The focused command failed 7 passed /
1 failed because successful resolution still returned `dependency=None` for a
registered dependency.

## GREEN 4

The resolver now loads and validates the Slice 20 dependency row and complete
source-link chain on the same reader snapshot, returning the typed dependency
only after the artifact and source are authorized. The focused suite passes
8/8.
