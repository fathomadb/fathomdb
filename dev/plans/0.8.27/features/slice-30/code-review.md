---
title: FathomDB 0.8.27 Slice 30 - code review
status: PASS
target_release: 0.8.27
reviewed_candidate: 2967593cc77af82acc6a3e1e50d03970c1a07705
---

# Slice 30 code review

## First review

Independent adversarial review rejected candidate `5ddfb4a8` with three P1
and three P2 findings:

1. semantic comparison incorrectly required identical capture-source SHAs;
2. Python capture omitted named `m.add` registrations and stub class bases;
3. the baseline recorded unsupported Node 26 instead of pinned Node 25.9.0;
4. the comparator test was absent from the canonical fast harness;
5. missing-executable and exact-version failures were not typed/exact; and
6. baseline immutability allowed a hardlink alias and writes were not atomic.

Commit `f621c9cd` added genuine RED coverage for every finding. Commit
`add4f3f4` implemented the fixes, `d80a6623` recaptured the reviewed baseline
under Node 25.9.0 with complete Python rows, and `b102bceb` bound the corrected
design, chronology, and evidence.

## Final verdict

The same reviewer returned PASS at clean `b102bceb`. Distinct valid provenance
SHAs now compare equal when their surfaces match; malformed provenance still
fails. The baseline contains 121 PyO3 registrations including all 42 named
aliases/exceptions and preserves complete stub class bases. Node, exact tool
versions, typed execution failures, canonical test registration, hardlink
rejection, and same-directory atomic replacement all passed focused review.

No runtime product, schema, API, package-root, feature-gate, transaction, or
locking behavior changed.

## Adversarial review addendum

This record verifies the 12-row tool at `b102bceb`. The comparator and baseline
were subsequently changed by adversarial-review FIX-1/FIX-2 (see `status.md`);
that review is the reviewer of record for those changes and the 13-row
baseline `b54a01cc486c…` captured at `58bc8eb4`.

## Whole-work review and final verdict

The external follow-up was expanded from the reported leftovers to the whole
Slice 20/30 change set: plans, acceptance criteria, design, assigned
functions, implementation, tests, baseline, records, and generated artifacts.
It initially rejected the candidate for seven substantive gaps: stale Python
native shadowing, debug NAPI declaration leakage, incorrect multiline cfg
ownership, single-filesystem capacity checks, weak scratch ownership, a
repository-relative Slice 55 fixture, and stale closeout evidence. Subsequent
review rounds also rejected worktree-forbidden `maturin develop`, dirty-tree
attestation, assertion-based receipt validation, and missing environment
guards.

RED/GREEN commits `87a4585d`/`def7d894`, `0e1185ba`/`d63b585a`, and
`523b9427`/`2967593c` closed those findings. Independent code review returned
PASS at clean `2967593c`: the Python gate builds a locked wheel in disposable
scratch and validates a clean-HEAD, path-, digest-, and nonce-bound receipt;
the NAPI production build sanitizes debug output and is exercised after a
debug build; the comparator checks distinct cache and scratch devices; and
the Slice 55 fixture cleans its actual temporary directory.

Commit `6ba3be95` only corrected an older packaging assertion to follow the
canonical native-build wrapper. The full Python suite then passed 1,533 tests
with 27 documented skips. It does not alter the execution paths reviewed at
`2967593c`. Final verdict: **PASS**, with no open Slice 20/30 code findings.
