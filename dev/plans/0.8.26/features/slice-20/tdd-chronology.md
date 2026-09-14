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
