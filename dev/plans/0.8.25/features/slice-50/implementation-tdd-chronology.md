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
