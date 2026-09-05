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
