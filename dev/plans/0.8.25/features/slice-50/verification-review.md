---
title: 0.8.25 Slice 50 independent verification
status: VERIFIED
verified_product_commit: e741542d
updated: 2026-09-05
---

# Slice 50 independent verification

Independent verification passed the reviewed product candidate `e741542d`
with no unresolved P0, P1, or P2 finding. Later commits through `3a3b1571`
only record reviews and correct the governed-surface allowlist and its
self-description for already-approved Slice 45 and Slice 50 methods; they do
not change product, schema, wire, or runtime behavior.

## Functional evidence

- Focused Rust evidence tests pass 23/23 with default features and 23/23 with
  `test-hooks`; codec/property tests pass 2/2 and the Rust facade test passes
  1/1.
- TypeScript evidence tests pass 4/4. Ruff passes and Pyright reports zero
  errors and zero warnings.
- A fresh Linux wheel passes the 14-test Python evidence suite and an installed
  runtime smoke. The independently built verifier wheel SHA-256 is
  `5de8a978b2c080dcb571b30e1edd0cdd77cf4d61e5e0e7b99f9146f5c2af4408`;
  its native module SHA-256 is
  `5910a6611094d9919b30e67a88e63017efe555f8885146fd355e38dc6efea33f`.
- A second canonical Linux package route passes an offline installed-wheel and
  packed N-API smoke. The wheel SHA-256 is
  `d3cbdf449ed35b6c3531a277b0b8744bf1268a6e54dd64f7693ee7eb19655bac`;
  the N-API native binary SHA-256 is
  `1487215dca59b6672e3eb578b0d4f89909e98e5ac052965a7ab54d592c6085b7`.
- The complete authenticated corruption, authority, lifecycle, race,
  statelessness, and reachable-origin matrix passes. Existing search results,
  hit order, SQL paths, and database/WAL bytes remain unchanged.

## Windows evidence

The exact `e741542d` source archive had SHA-256
`0713a520f50657da005762d98ff17e1af47a9f32cb5cb45c9fe6b4649686f816`
on both Linux and Windows. Native Windows verification passed:

- Rust evidence tests 23/23;
- TypeScript/N-API evidence tests 4/4;
- fresh installed-wheel Python evidence tests 14/14;
- canonical offline wheel and packed-N-API smokes; and
- Python 3.11.16, Node 25.9.0, and Rust 1.95.0 builds.

The Windows wheel SHA-256 is
`ca242411253226579ee22bd2327f81395acbb90841e2d9988d67b24b8ff2f703`;
the native N-API binary SHA-256 is
`fde3c1551caa7609d6eaf269d6577bc41b0bfd4605131383e6d22de040a32421`.

## Repository and feature gates

The heavy repository gate first exposed two environment/governance issues,
not a Slice 50 product failure:

1. a stale worktree extension caused Python collection to miss a new symbol;
   fresh installed wheels passed, and the final heavy route uses a disposable
   exact-source environment with its own test-hook build; and
2. the shared governed-surface allowlist had omitted six already-approved
   Slice 45 pagination/state spellings. The correction is pinned at
   `6fc3258e`; its follow-up independent review passes at `3a3b1571` and
   confirms no executable predicate or product behavior changed.

The pin checker, fresh-wheel Python surface suite (17/17), and TypeScript
surface suite pass after that correction. The selected
`operator,test-hooks,default-embedder,default-reranker` Rust route also passes
serially, including the Slice 50 evidence suite at 23/23. Its first parallel
attempt encountered one scheduling-sensitive Slice 35 frozen-read race and a
second parallel attempt stalled in a legacy WAL-attribution race; the unchanged
focused oracle passed and the complete serial route passed. Neither involved a
Slice 50 product correction.
The final heavy and fast repository gates pass from exact source outside the
sandbox, including the standing-authorized ptrace route.

The isolated heavy gate used a Git archive of `3a3b1571` with SHA-256
`ad122db767ebc7d78e57b0b613e5553b4bcef0109caeb5d96a06b3957cddbe06`,
a source-owned disposable virtual environment, and a locally built test-hook
binding. It passed all 3/3 heavy suites with no skipped or excluded suite and
did not rebind a shared or worktree virtual environment.

CUDA/GPU execution is not required for evidence resolution: Slice 50 changes a
CPU storage/visibility operation and does not alter embedder or reranker
dispatch. The commonly deployed optional CE-reranker workload remains an
explicit Slice 75 release-verification requirement.
