---
title: FathomDB 0.8.26 Slice 50 — verification architecture
status: APPROVED
---

# Slice 50 design — verification architecture

## Candidate and evidence topology

One reviewed implementation commit is the candidate input. Axis W is first
bumped mechanically to `0.8.26`; Axis E remains independent. Fresh wheel,
npm/native, and CLI artifacts are built from that exact commit. Evidence docs
may land afterward, but they identify the implementation candidate separately
from the final record commit so the artifact hash/commit binding is not
circular.

Generated evidence is data, not a manually asserted verdict. A versioned JSON
schema requires candidate SHA, Axis W/E, platform and toolchain identity,
artifact path/name/size/SHA-256, executed command identity, outcome, and bounded
diagnostic. The validator rejects unknown outcome classes, duplicate matrix
keys, a pass without a command/artifact binding, and a platform claim without
execution. Large binaries and raw logs remain outside Git; the durable record
keeps hashes and concise diagnostics.

## Installed integrated profile

Every package witness starts from a fresh install root with checkout imports
disabled. The integrated profile uses public APIs and real persisted state:

1. bootstrap a schema-34 database and atomically write the supported
   canonical/derived/dependency/edge unit;
2. commit another derived edge through current V1 actuation and retain its
   receipt/digest;
3. restart and prove deterministic read/replay state;
4. freeze, search with explanation/evidence, and resolve exact ranked evidence;
5. traverse explicit and query-derived seeds in outgoing, incoming, and both
   directions at depth one and multihop;
6. for ordinary and actuated routes, bind each selected result position to its
   target and winning terminal-edge reference and resolve both under the same
   frozen authority;
7. record route-selection provenance separately from intrinsic source/artifact
   evidence and assert graph evidence contains no ranked-only projection or
   contribution fields;
8. page dependency/projection state and run the version-matched external
   integrity CLI under its quiescence rule; and
9. reopen persisted state across bindings where the public contract permits.

One representative schema-33 fixture is hashed before and after a refused
open, including sidecar existence and bytes. This proves the fresh-only boundary
without creating migration or general backward-compatibility machinery.

## Windows test-hook inventory

A small versioned JSON document owns the complete private Python `test-hooks`
module/symbol surface. `_test_hooks_gate.py` derives the actual callable
surface from the PyO3 source and compares it with that contract. Its child-
process installed-wheel probe loads the same contract rather than a Python
tuple. The retained Windows package witness receives the trusted contract path
and proves every declared module/class/member exists in the disposable wheel.

The structural and typing guards derive their expected symbols from the same
contract, including the focused Slice 65 static checks that currently keep
handwritten subsets. The workflow proves the contract reaches the clean-import
probe in the disposable `test-hooks` wheel. Mutation tests add a Rust hook,
remove a declared symbol, restore a handwritten workflow subset, and omit the
contract argument; each must fail. Managed-connection creation counts, native-
state facts, retained-reader BUSY attribution, completion, and zero-test
controls are separate runtime oracles and remain unchanged.

## Registry visibility seam

One stdlib-only helper classifies PyPI and npm exact-version lookup responses.
The package set is derived, not handwritten: `fathomdb` at Axis W on PyPI;
`fathomdb` plus every package named by `src/ts/npm/*/package.json` at Axis W on
npm. It records registry, package, version, attempt count, bound, and terminal
reason. Only a well-formed response saying that exact requested version is
absent (including the registry's exact-version 404 shape) is retryable. Exact
presence succeeds. Authentication/authorization, rate-limit, 5xx,
DNS/TLS/transport, malformed JSON, wrong package identity, and semantic
metadata mismatch fail immediately. Exhaustion fails with an exact-version-
unavailable diagnostic.

A non-publishing workflow job runs after the publish jobs and before any
post-publish smoke fan-out. The existing `0.8.20` recovery route checks PyPI
only and omits npm exactly as its publish path does. The T1–T7 crates.io
dependency waiter remains unchanged and separate. Slice 50 proves the new
behavior with a local HTTP fixture and workflow-structure tests only; it
neither calls real registries for `0.8.26` nor treats fixture success as
registry evidence.

## Platform and package evidence

The existing CI `workflow_dispatch` exact-SHA route owns the five supported
native CPU targets: Linux x86-64/ARM64, macOS x86-64/ARM64, and Windows x86-64.
It builds the shipped wheel and matching N-API artifact on each actual runner,
installs local artifacts, and executes the runtime harness. The Windows matrix
arm runs `smoke-local-native-artifacts.ps1` and its retained N-API modules; it
does not own the WAL attribution suite. After a successful harness, a common
receipt writer hashes the one wheel and matched N-API binary, records their
names and sizes with the candidate SHA, target, runner/toolchain labels,
command identity, and outcome, then each matrix leg uploads only that compact
JSON receipt. A collector downloads all five receipts, rejects missing or
duplicate targets and candidate drift, and supplies their validated rows to
the durable manifest.

The separate `windows-wal-attribution` job builds the disposable Python
`test-hooks` wheel, runs the clean-import inventory and retained WAL controls,
and emits a distinct candidate/wheel/test receipt. It is required in the
candidate evidence but is not one of the five native artifact rows. Local
Linux package proof and external platform proof remain separate rows.

CUDA and Metal are excluded because Slice 50 changes no accelerator behavior
or package. Post-publication registry installs remain a separate release action
and cannot be inferred from local packages or dry-run workflows.

## Gitleaks admission and decision boundary

Run the exact candidate evidence generator, then scan its output directory
directly with the pinned Gitleaks configuration; `gitleaks-current.sh` alone is
insufficient because it archives only tracked paths. Run the tracked-tree scan
separately. If no stable benign value is rejected, P26-11 closes with no
configuration change.
If one is rejected, admit only its exact digest through the existing versioned
input and retain a nearby credential-shaped mutation that still fails. Paths,
regular expressions, generalized allowlists, and full-history debt triage are
outside this slice.

Slice 50 may mark the non-publishing candidate READY only when every local and
five-target requirement passes and all capable-executor claims have receipts.
It may not tag, publish, promote, or mutate release registries. Those actions
remain separate HITL decisions.
