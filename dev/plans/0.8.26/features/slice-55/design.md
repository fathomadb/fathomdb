---
title: FathomDB 0.8.26 Slice 55 — SDK parity oracle design
status: APPROVED
target_release: 0.8.26
---

# Slice 55 design — canonical-operation parity

## Authority split

`src/conformance/governed-surface-allowlist.json` remains the raw-byte signed
membership authority and is not rewritten or reformatted. A companion
canonical-operation map owns only pairing and discovery metadata. Its validator
must prove that the de-duplicated union of every entry's `signed_members`
equals the signed allowlist exactly. Signed tokens are separate from observed
runtime spellings: for example, signed `Engine.open` is observed as locator
`engine_static` plus spelling `open`, while signed `read.get_many` is observed
as Python `read/get_many` and TypeScript `read/getMany`. A mismatch fails; the
companion cannot widen, narrow, or repin the approved surface.

## Canonical operation model

Each companion entry represents one canonical operation and carries its state,
one or more signed tokens, and one locator/runtime-spelling pair for each
binding. The closed locator vocabulary is `package`, `engine_static`,
`engine_instance`, `admin`, `read`, and `graph`. Only `live` entries
participate in the executable parity set; observing a `reserved` entry is an
explicit failure and cannot satisfy a missing live operation.

Each binding independently introspects every applicable discovery root,
subtracts a small documented non-command exclusion set, and resolves every
remaining locator/spelling through the companion map. Python module namespaces
select exported functions rather than every callable so exported types such as
`TraversalDirection` do not masquerade as commands. The oracle then requires:

1. every introspected name maps exactly once;
2. every live manifest spelling is introspected on its named binding;
3. neither binding exposes an ungoverned command;
4. the canonical live-operation sets are equal; and
5. `(binding, locator, spelling)` is unique and cannot resolve to two canonical
   IDs; an identical spelling in both bindings is valid only when it names the
   same canonical operation; and
6. the flattened `signed_members` union across all entries is exactly the
   signed allowlist, independent of lifecycle state.

This separates four facts that a flat allowlist conflates: canonical operation
identity, discovery location, binding-specific spelling, and whether an entry
is live.

The production validator is pure over a parsed companion, parsed signed
contract, and observed per-binding locator/spelling sets. Integration tests
feed it actual runtime introspection. Mutation tests copy and alter those
inputs; they do not rewrite a golden file or test a second predicate.

## Existing `rerank` defect

The accepted parity ADR, signed allowlist, and locked binding interfaces already
classify standalone `rerank` as live. Python exposes it at package scope while
TypeScript does not. The canonical map records `package:rerank` for both, so the
first real-surface RED fails on TypeScript.

GREEN adds a package-level TypeScript async function and N-API function over
the existing engine `rerank_passages` helper. The TypeScript input is a query,
`{ id, body, score }[]`, a non-negative integer `rerankDepth`, and optional
`alpha`/`poolN`; output is `{ id, score, ceScore }[]`. JavaScript validates
strings, safe non-negative integer IDs, finite scores, depth/pool bounds, and
finite alpha before native work. Native work runs off the event loop and maps
panics and validation failures through the existing typed envelope. Depth zero
or empty input is the model-free identity path. No canonical operation,
retrieval pipeline, schema, or default feature set changes.

## Failure model

The tests must fail independently for a one-sided removal, one-sided addition,
misspelling, duplicate mapping, and reserved entry used as a live substitute.
Fixture mutation is isolated from production files and does not regenerate a
golden oracle. Error output names the binding, spelling, and canonical
operation responsible for the mismatch.

## Compatibility

The slice preserves the signed allowlist bytes and the approved live canonical
set. The only runtime implementation change fills the missing TypeScript peer
for an operation already present in signed and locked authorities. The
five-name recovery denylist remains a separate negative guard; `doctor` remains
absent because it is not a live SDK operation.
