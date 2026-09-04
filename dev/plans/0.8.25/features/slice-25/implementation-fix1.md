---
title: 0.8.25 Slice 25 implementation FIX-1 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 1
reviewed_commit: f660108bf9d12f32ca2341f04b7fdf2105cf6166
---

# Slice 25 implementation FIX-1 response

Preserved core RED commit:
`7366d9db7acd11c5534b37078e8e751e3b12b426`.

Preserved SDK-path RED commit:
`ccdc8a23169f58386574d2b1affffa9a2725d049`.

## Corrections

- Revalidate mutable nested lifecycle fields before canonical digesting, so an
  invalid public Rust value returns its typed operation path without panic.
- Simulate the ordered request in a rolled-back savepoint before resource
  exhaustion checks. The simulation uses transaction-local puts,
  dependencies, lifecycle changes, and closure state, preserving the frozen
  boundary → operations → closure → cursor → generation precedence.
- Map missing dependency endpoints to `reference_unavailable` and preserve
  nested domain paths in Rust, Python, and TypeScript.
- Make Python's request `TypedDict` declarations compatible with Python 3.10
  while retaining required and optional key distinctions.
- Distinguish a new terminal attempt from an inner race replay. Exact replays
  do not emit mutation telemetry or increment counters; keyed errors record
  their typed stable code.
- Redact matching receipt operation IDs in fixed 64-row keyset pages rather
  than materializing the full target set.

## TDD chronology

The core RED commit exposed all six original behavioral failures. The first
GREEN pass then corrected two expectation spellings to the already-governed
canonical contracts: provenance hash failures are owned by the
`/canonicalSourceHash` object path, and Engine telemetry keys use the stable
`ActuationError` code. Commit
`3fc3c121813c027d8b44e79a978fd9f8ceab42b1` preserves the first correction;
the final expectation changes remain visible in FIX-1 GREEN. These are oracle
name corrections, not relaxed behavior.

The separate SDK RED commit proves that both bindings previously collapsed a
decoder's nested field path to the operation's record/dependency root. The
GREEN change preserves the decoder-owned canonical path and prefixes it with
the operation root.

Post-correction verification cases additionally cover write-cursor and
dependency-generation precedence, a same-operation-ID race, and erasure over
more than one receipt page. They are acceptance-completeness tests, not
represented as pre-fix RED witnesses.
