# Slice 20 TDD chronology

## Corrected-design RED

Commit `884d34bd` added focused regressions for the approved corrected design.
Before GREEN, the focused test binary reported four failures:

- graph evidence references were 406 characters rather than the required 704;
- an authenticated relaxed-window context returned a successful evidence
  sidecar rather than `graph_context_invalid` at
  `/context/context/view/includeOutOfWindow`;
- missing provenance did not produce the required positional incomplete error;
- a denied canonical source did not dominate malformed locator detail.

The exact focused command was:

```text
cargo test -p fathomdb-engine --features test-hooks --test slice20_graph_evidence
```

## GREEN increments

Commit `e237720a` implemented the fixed 300-byte selector and 704-character
reference, separately domained commitments and stream/MAC, semantic request
binding, cursor-coherent target/edge resolution, authenticated relaxed-window
refusal, nullable ordinal-preserving hydration, and SQL-compiled target/source
eligibility. It retained `GraphTargetV1.writeCursor` and added no cursor to the
sidecar or resolver result.

Commit `9c92a005` strengthened global target-plus-edge authorization ordering and
added private fixed-nonce tests for all ten stream blocks, bytes beyond 64,
domain separation, exact framing, kind-set normalization, frozen-token
exclusion, and explicit-seed order sensitivity. Focused unit runs passed 2/2
for selector tests and 2/2 for request-normalization tests.

The corrected implementation retains the frozen snapshot validation before
hydration. Consequently, the original RED fixtures that mutate SQLite after
minting the frozen context now correctly return `FrozenRead(StateDrifted,
/token)`. The authoritative reconciliation at `fa39e5e9` requires preserving
those scenarios as security regressions and adding separate provenance fixtures
whose malformed state exists before the context is frozen. The implementation
agent was prevented by the execution security reviewer from changing those test
oracles despite the additive reconciliation; the release branch must land that
test-only reconciliation for merge and verification here.

The independently reviewed additive correction was cherry-picked as
`d8ae9a50`: it retains the post-mint mutations as explicit frozen
`StateDrifted` regressions and adds separately authenticated pre-freeze fault
fixtures for the nondisclosure and positional-incomplete precedence contracts.
The full focused graph-evidence suite then passed 13/13.

Commit `588a1b67` preserved a separate temporal RED. Its exact setup failure was
`Provenance(SourceRevisionIneligible, /provenance/sourceRevisionId)`: the first
fixture had made its canonical source historical before the governed derived
write. Commit `f7f7853a` corrected only fixture timing, keeping the source valid
at write while pinning frozen `validAsOf` exactly to tested start/end instants.
The target, winning-edge, and canonical-source half-open boundary test passed.

## External gate observed

`cargo check -p fathomdb-engine --all-targets --features test-hooks` reached an
unrelated future-slice test and failed because
`slice75_schema26_upgrade.rs` calls the not-yet-present `rebuild_projections`
method. This is outside Slice 20; targeted library and focused Slice 20 checks
remain the appropriate gate until that release-ladder dependency lands.

## Independent review FIX-1

The independent code review requested four corrections. The response remained
additive and preserved every earlier security oracle.

- `b5b47d33` added the nonce RED. The focused failure was the exact assertion
  `left: 1`, `right: 0`: minting still executed SQLite
  `SELECT randomblob(16)`. `d14fff43` replaced that query with the existing
  platform RNG mechanism through `getrandom::fill`; token framing and length
  stayed unchanged.
- `5242b0ab` added exact phase-two stored-fault and precedence oracles.
  `fbce32ae` made phase two classify all material before returning, order by
  target index with target before terminal edge, and report the exact
  `/targets/{i}/provenance` or
  `/targets/{i}/terminalEdge/provenance` path.
- `7be5aea4` added the runtime statement-count RED. Compilation failed with six
  unchanged `E0599` diagnostics because
  `graph_expand_with_statement_count_for_test` did not exist. `bf4f3525`
  instruments every statement executed on the real reader connection. The
  differential oracle proves zero evidence-hydration statements for an empty
  result and exactly two for one or multiple targets backed by multiple
  canonical sources.
- `ca66f23a` added recursively closed Rust sidecar/entry decoder REDs. The exact
  failure was `unwrap_err()` receiving `Ok(GraphExpandResultV1 ...)` for the
  unknown `/evidence/z~1future~0field`. `6986d44f` rejects sidecar and entry
  unknowns at escaped RFC 6901 paths.
- `dfaf33a0` added Python/TypeScript malformed resolved-response REDs. The
  TypeScript diagnostic was `AssertionError: Missing expected rejection` for
  `/z~1future~0field`. `48b9a5c3` closed the outer and nested response shapes,
  validates revision identities, union coherence, nullability, lifecycle
  coherence, canonical numeric strings, and exact paths. Python now validates
  decoded native JSON before indexing any member.
- `0ad5df62`, `cf5c9895`, and `36e80852` add the remaining real-engine oracles:
  edge-at-index-zero versus target-at-index-one precedence, target-before-edge
  at one index, absent versus present-empty source attributes, source status and
  created-after behavior with missing vector metadata, multiple distinct
  sources under the two-statement bound, primary-mutex ordering for both
  governed erase and operator excise, and real TypeScript target/edge
  resolution. The two global erasure hooks are intentionally exercised in one
  serial test; the first parallel spelling exposed a test-only hook race and was
  stopped rather than normalized into a product claim.
- `ecf60335` extends the source-independent wheel profile with installed-package
  graph expansion and exact target/terminal-edge resolution.

The fresh wheel gate built
`fathomdb-0.8.25-cp310-abi3-manylinux_2_39_x86_64.whl` with SHA-256
`df36b7886157527ecd30d3cf6d49270dcb90431cc9ea2a9a34302874c59991c5`.
The installed profile passed, and the nine focused Python tests passed against
that installed wheel with repository source injection disabled. A fresh local
npm install is deferred to the release packaging ladder because the committed
main package intentionally excludes the native binary and requires the
platform-package injection step; source-level real-NAPI coverage passed here.

## Independent re-review FIX-2

The final planned correction cycle closed all four re-review findings without
changing the approved public contract.

- `6666b906` added canonical-source authority and globally ordered missing-link
  REDs. A wrong canonical-source registry role/class/completeness combined with
  malformed target locator/hash returned
  `EvidenceCorrupt` at
  `/targets/0/provenance/canonicalSourceHash`, violating nondisclosure
  precedence. A missing terminal-edge link returned `EvidenceIncomplete` at the
  incorrect `/targets/0/terminalEdgeProvenance` path. The same RED also covers
  two targets where edge index 0 must precede target index 1, and same-index
  target-before-edge ordering.
- `6f7b946d` moves the complete canonical-source authority predicate into phase
  1: registry schema, node class, `canonical_source` role, complete provenance,
  canonical-row/source identity, lifecycle, half-open window, compiled filter,
  and closure state. Only after global authority passes are missing links
  collected, sorted by target index and target-before-edge, and returned at the
  exact target or `terminalEdge/provenance` path. Both focused REDs pass.
- `a5389d3c` added the engine-scoped rendezvous RED. Compilation failed with the
  unchanged `E0599` diagnostic: no method named
  `arm_graph_evidence_before_resolve_return_hook_for_test` existed on
  `Arc<Engine>`. `e47bdce0` adds the test-only rendezvous to each Engine instance,
  proves another engine cannot consume it, and proves both transaction orders:
  resolver-first may return its authorized copied result; erase-first and
  excise-first make graph resolution nondisclosingly unavailable. No SDK
  delivery-order claim is made.
- `3fc9ed6e` then strengthened the same finding by moving the erasure-side
  rendezvous off its legacy process-global seam. Its RED failed with `E0599`:
  no method named `arm_erasure_before_primary_lock_hook_for_test` existed on
  `Arc<Engine>`. `6f3662b0` adds the engine-scoped erasure seam, retains the
  legacy hook only for older unrelated tests, and proves neither resolver nor
  erasure rendezvous can be consumed by a different Engine.
- `5e84f837` extends the repository-owned local-native release smoke. It uses the
  real publish-time optional-dependency injection, packs the matched platform
  package and thin main package, performs a clean offline install, checks the
  installed declarations and exports, loads through the installed platform
  package, and resolves both target and terminal-edge evidence through the real
  installed N-API package. The marker
  `slice20 installed N-API graph evidence: pass` was observed, followed by the
  canonical smoke completion marker.

Final focused verification passed: Rust graph evidence 25/25, selector property
and framing 3/3, request normalization 2/2, no-SQL nonce 1/1, engine check,
engine Clippy with `-D warnings`, TypeScript typecheck and real-NAPI 3/3, shell
syntax, and strict public documentation.
