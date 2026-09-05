---
title: 0.8.25 Slice 50 implementation TDD chronology
status: GREEN
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
Engine-owned generation history rather than requiring it to remain the current
serving generation. It rejects zero or duplicate matches and validates the
closed generation-ID grammar before disclosure. The focused suite passes 7/7,
including restart, retirement, and supersession. FIX-1 later replaces this
initial history scan with a direct protected selector.

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

## RED 5

The fifth test increment defines installed-surface parity with one facade
compile test and real-database Python/TypeScript search-and-resolve tests. Rust
failed on the missing facade re-exports. Python static checking failed on the
two missing methods and request types. TypeScript checking failed only on the
two missing Engine methods. The local Python binary is stale before this
increment, so native Python execution is reserved for the rebuilt-package
GREEN route rather than misreported as a product diagnostic.

## GREEN 5

The Rust facade now re-exports the evidence contract. The PyO3/Python and
napi-rs/TypeScript bindings expose request-object search and resolution with
the same nested result vocabulary and typed `FDB_EVIDENCE` error family. The
focused TypeScript real-database route passed. A wheel built from this exact
worktree and installed into a fresh temporary environment passed an installed
open/write/search-with-evidence/resolve/error smoke.

The first full TypeScript run passed 402/404 tests and correctly stopped only
on the governed-surface pin because the two new reads had not yet been added to
the approved Slice 50 surface record. The allowlist and its explicit shape
oracle now carry `search_with_evidence`/`searchWithEvidence` and
`resolve_evidence`/`resolveEvidence`; the pin is intentionally refreshed only
after the implementation commit supplies immutable provenance.

## RED 6

The sixth increment required a result minted with
`include_out_of_window=true` to resolve under the exact same relaxed frozen
view. The focused Rust test failed with `evidence_unavailable`: resolution was
unconditionally enforcing the validity window even though origin search had
lawfully relaxed that axis.

## GREEN 6

Artifact, canonical-source, and graph-origin validity checks now honor the
originating view's explicit validity relaxation while retaining active-state,
supersession, provenance, and dependency-closure barriers. Graph-origin
resolution additionally revalidates its edge source revision. The focused
Rust suite passes 9/9.

## RED 7

The seventh increment required unsupported evidence request schemas and
unknown TypeScript request fields to use the evidence-specific typed error
contract. The focused TypeScript test caught `InvalidArgumentError` for the
schema case instead of `EvidenceError`; the installed Python smoke also showed
that `EvidenceError` was not exported at package top level.

## GREEN 7

Both SDKs now expose `EvidenceError` and return
`unsupported_schema_version` at `/schemaVersion`; TypeScript additionally
returns `unknown_field` at the exact unknown request-field pointer. TypeScript
passes both focused real-database tests. The rebuilt, freshly installed Python
wheel passes exact source resolution and the typed schema-refusal smoke.

## RED 8

The eighth increment made the disclosure precedence executable. After a
reference was minted, the fixture made its artifact provenance incomplete and
its canonical source ineligible under the bound owner predicate. Resolution
incorrectly returned `evidence_incomplete`, revealing structural state before
current source authorization.

## GREEN 8

Incomplete-provenance detail is now evaluated only after artifact, source,
graph-origin, lifecycle, validity, eligibility, and closure authorization.
The hidden case returns only `evidence_unavailable`; restoring source
eligibility makes the same authenticated incomplete row return the documented
`evidence_incomplete` result. The focused real-database suite passes 10/10.

The codec verification also includes a property test over cursors, validity
instants, ranks, finite scores, and commitment bytes. Canonical payloads
round-trip byte-for-byte, remain under the size cap, and a one-character token
tamper always returns the non-disclosing error.

## RED 9

The first implementation review identified three executable contract gaps.
New failing tests require evidence search to collapse malformed frozen contexts
to `evidence_unavailable`, require unsupported resolver schemas to precede
context authentication, and require Python/TypeScript evidence search to reject
negative ranking controls before native unsigned conversion. The Rust tests
failed with `FrozenReadError`; TypeScript reproduced the same leak through the
native addon.

A second RED fixture corrupts the authoritative Slice 15 source-version row
after minting a valid reference. Resolution incorrectly succeeded because it
validated only the derived link fields and source revision body.

## GREEN 9 — implementation-review FIX-1

Evidence search and resolution now apply the documented schema and
nondisclosure precedence in the Engine itself. Python no longer performs a
leaking preflight call, and both dynamic SDKs validate the same ranking controls
as ordinary frozen search before crossing FFI.

Authorized resolution now reuses the complete Slice 15 canonical/derived
provenance-chain validation and maps persisted dependency corruption to typed
evidence corruption. Retained projection generations use an authenticated,
key-protected selector and one indexed primary-key probe instead of scanning
unbounded history. Closed response discriminants are checked by both dynamic
SDKs before public objects are constructed.

The adversarial suite now also covers source/dependency corruption, source
deletion and erasure, graph-edge retirement, both frozen-search race windows,
and expanded low-entropy privacy markers. The focused Rust suite passes 16/16;
strict Clippy is green; TypeScript and a freshly built/installed Python wheel
pass the corrected evidence routes.

## RED 10

The graph-origin fixture next corrupted the captured edge's authoritative
source-version row without changing the returned node or edge revision.
Resolution incorrectly succeeded, proving that the main body provenance was
fully checked while the independently authorizing graph subject was not.

## GREEN 10

Graph edge-seed and traversal resolution now retain the authorized edge/source
pair and validate its complete Slice 15 provenance chain only after all
nondisclosing eligibility and lifecycle checks. Authorized corruption returns
`evidence_corrupt` at `/projectionOrigin/graphOrigin`; subsequent edge
supersession remains indistinguishable `evidence_unavailable`. The full focused
suite remains 16/16 green.
