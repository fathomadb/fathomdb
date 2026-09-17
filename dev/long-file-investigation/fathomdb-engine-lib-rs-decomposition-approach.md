# FathomDB engine `lib.rs` decomposition approach

## Position

`fathomdb-engine/src/lib.rs` should be decomposed inside the existing
`fathomdb-engine` crate without initially redesigning `Engine` or changing
public paths.

The objective is not to minimize line count. It is to make capability
ownership, targeted retrieval, review, and verification possible without
loading unrelated implementation.

This document is the adjusted successor to the verbatim response retained in
[`fathomdb-engine-lib-rs-decomposition-approach.txt`](fathomdb-engine-lib-rs-decomposition-approach.txt).

## Snapshot evidence

The file has varied substantially across active branches and release
worktrees. Measurements must therefore name their source revision.

- Local `main` at `8b4bc1c632a6`: 25,352 lines, 1,193,310 bytes, 663 function
  definitions, 100 functions ending in `_for_test`, and 85 public
  structs/enums/traits/types.
- Experiment baseline `a563362d7f65`: 33,292 lines and 1,506,800 bytes.

The exact count is not the architectural argument. Across the measured
snapshots, the file combines:

- engine construction and SQLite opening;
- reader-pool concurrency;
- writes and validation;
- search, ranking, filtering, and graph traversal;
- projection scheduling, commits, registry management, and maintenance;
- vector partition management;
- erasure and redaction;
- extractor subprocess sessions;
- operator diagnostics;
- telemetry; and
- extensive test controls.

Those capabilities usually are not needed together for one task. That is the
actionable ownership and retrieval signal.

## Architectural constraint

The accepted
[`ADR-0.6.0-crate-topology`](../adr/ADR-0.6.0-crate-topology.md) keeps
`fathomdb-engine` as one crate and calls for internal module boundaries. This
rules out speculative subsystem crates as the default response to the large
file.

Keep `Engine` defined in an ancestor of every capability module that needs its
private fields. The lowest-risk initial shape keeps it at the crate root:

```rust
// lib.rs
pub struct Engine {
    // Existing private fields remain unchanged.
}

mod erasure;
mod open;
mod projection;
mod read;
mod search;
mod test_hooks;
mod write;
```

Child modules can contain inherent implementations:

```rust
// search/mod.rs
use crate::{Engine, EngineError, SearchResult};

impl Engine {
    pub fn search(&self, query: &str) -> Result<SearchResult, EngineError> {
        // Existing implementation moved verbatim.
    }
}
```

External consumers continue to call `Engine::open`, `engine.search`, and
`engine.write`. No wrapper object, new trait, new crate, shared context bag, or
broad field-visibility change is required.

Moving `Engine` into `engine/mod.rs` while leaving capability modules as its
siblings would lose descendant access to its private fields. Do not adopt that
shape merely to make the tree look conventional.

## Candidate target

The following is a concrete starting hypothesis, not the preordained output of
the boundary experiment:

```text
fathomdb-engine/src/
├── lib.rs                    Engine state, public contract, re-exports
├── error.rs                  EngineError and error formatting
├── id.rs                     IdSpace, SourceId, identity derivation
├── temporal.rs               ReadView, validity and time conversion
├── reader_pool.rs            reader workers and request dispatch
├── test_hooks.rs             Engine test controls
├── open/
│   ├── mod.rs                Engine::open family
│   ├── probe.rs              database/schema/embedder probes
│   └── errors.rs             open-specific errors
├── write/
│   ├── mod.rs                Engine::write and preparation
│   ├── validation.rs
│   └── commit.rs             transaction and cursor assignment
├── search/
│   ├── mod.rs                public search methods
│   ├── exec.rs               search transaction
│   ├── ranking.rs            RRF, CE reranking, reweighting
│   ├── filter.rs
│   └── graph.rs
├── projection/
│   ├── runtime.rs            dispatcher and workers
│   ├── commit.rs             projection outcome transactions
│   ├── registry.rs
│   └── maintenance.rs
├── erasure/
│   ├── mod.rs                purge/excise behavior
│   └── redaction.rs
├── extract/
│   ├── session.rs
│   ├── ingest.rs
│   └── consolidate.rs
└── tests.rs                  root unit tests, initially moved verbatim
```

The older detailed proposal in
[`engine-decomposition.md`](../plans/refactor-background-check/engine-decomposition.md)
contains useful ownership analysis, but its line numbers are stale. The
boundary experiment should regenerate symbol manifests from immutable Git
blobs rather than treating those ranges as current instructions.

## Role of the boundary experiment

The file's responsibility concentration already supports decomposition. The
experiment chooses among safe boundaries, estimates their operational value,
and identifies a useful stopping point. It is not a gate that can overturn the
need for ownership boundaries solely because historical transcript coverage is
sparse.

Candidate production boundaries require:

1. a semantic or invariant-based justification; and
2. an independent signal from dependency structure, test ownership,
   co-change, or co-retrieval.

Size triggers review but does not approve a boundary.

### Holdout discipline

Form complete task groups before chronological partitioning. A task group may
include separate RED-test, implementation, review-fix, and verification
commits. If any group member occurs after a cutoff, assign the entire group to
the later partition. No group may contribute evidence to both training and
holdout.

Use only training-partition changes and observations to generate or weight
candidate boundaries. Derive manual-domain candidates from the training-end
source snapshot.

The current baseline may still be used to:

- inventory every symbol that the eventual move must account for;
- enforce current API, feature, privacy, and invariant constraints; and
- verify that the candidate can place code added after training.

Assign baseline-only symbols through a pre-registered structural rule after
the candidate tree is frozen. Do not inspect their holdout behavior to choose
their owner.

### Retrieval counterfactuals

File decomposition affects retrieval differently depending on harness policy.
Report these policies separately:

1. **Recorded-range replay:** map exact historical ranges and symbols onto the
   candidate tree without expanding them. This measures boundary crossings,
   not source avoided.
2. **Same-policy path remap:** change the target path while preserving the
   recorded context span and expansion rule.
3. **Whole-module counterfactual:** retrieve the complete owning module. This
   estimates savings only for a harness that expands reads to module
   boundaries.
4. **Symbol-aware lower bound:** retrieve the target symbol plus a fixed context
   and dependency allowance. This is a comparison bound, not an expected agent
   trace.

Do not pool these policies. In particular, do not award whole-module savings
to a candidate when the observed harness already used bounded range or symbol
retrieval.

Static replay cannot predict how filenames alter future search choices,
searches before the first edit, elapsed time, or model quality. Measure those
only from real post-pilot tasks or a separately controlled replay.

### Transcript measurement layers

Keep the following quantities distinct:

- bytes produced by the subprocess;
- bytes retained by the harness or transcript;
- bytes actually exposed to the model;
- model-visible tokens; and
- unique versus repeated source ranges.

For every quantity, record `exact`, `lower_bound`, or `unknown`. Truncated
output is a lower bound at the affected layer. Missing data is `null`, never
zero. Do not infer model-visible input from process output or transcript size.

Historical transcripts may support only the harness-retained layer. Label that
measurement as a proxy rather than treating it as direct LLM-context evidence.

## Execution sequence

### 1. Freeze the public and build surfaces

Before moving code:

- record exported types, methods, re-exports, and feature gates;
- use the existing
  [`dev/interfaces/rust.md`](../interfaces/rust.md) contract and governed
  surface tests;
- qualify and pin a complete Rust public-API comparison;
- exercise default, `operator`, `test-hooks`, benchmark, and relevant combined
  feature configurations; and
- record the exact source revision and file hashes.

Moving a public definition behind a root re-export normally preserves source
use but can affect documentation and canonical-path details. Do not assume the
current binding allowlists are a complete engine API inventory.

### 2. Complete experiment phases 0–3

Freeze inputs and decision rules, inventory source, mine Git history, and
normalize sanitized transcripts. Stop for data-quality review before generating
a preferred tree.

The review must report at least:

- task-group and partition assignments;
- historical symbol-mapping coverage and ambiguity;
- transcript inclusion and exclusion reasons;
- parsed, partially parsed, censored, and unparsed event rates; and
- which output measurement layers are actually available.

### 3. Prove the module mechanism with test support

The default pilot is test support only when the evidence and manifest confirm
that it is cohesive and all callers and build configurations are accounted for.

- Move the bottom `#[cfg(test)] mod tests` body into `tests.rs`.
- Move `_for_test` methods into `test_hooks.rs`.
- Preserve every item-level `cfg` exactly.
- Do not gate the entire module blindly; some current test hooks are available
  outside `#[cfg(test)]`.
- Move code verbatim without renames or opportunistic cleanup.
- Stop after the pilot and compare predicted coupling with the actual compile,
  test, API, and navigation results.

After a successful pilot, add a baseline-and-delta guard preventing new root
`*_for_test` methods without making existing unrelated debt permanently red.

### 4. Extract private implementation clusters

Initially leave `Engine` and public contract definitions in `lib.rs`. Move
private helpers and associated `impl Engine` methods by demonstrated
capability.

The candidate ordering to evaluate is:

1. extractor session, ingest, and consolidation;
2. erasure and redaction;
3. vector and projection ownership;
4. search, filtering, graph traversal, and reader dispatch; and
5. engine opening after the subsystems it wires have stable homes.

This sequence is evidence-informed, not mandatory. Each extraction must be a
small, independently verifiable behavior-preserving change.

### 5. Move public contract types only if worthwhile

After implementation ownership is stable, decide whether public DTOs should
remain as a deliberate catalog in `lib.rs` or move behind root re-exports.

A root of several thousand lines can be acceptable when it contains cohesive
public contracts rather than unrelated implementation. The success criterion
is that a search task primarily opens `search/`, an erasure task opens
`erasure/`, and a projection task opens `projection/`.

### 6. Decompose oversized functions separately

Moving a large function improves ownership and discovery but not its internal
reasoning burden. Treat function decomposition as a later behavioral change
with its own failing tests and review.

Candidate functions include search execution, batch commit, projection job
execution, erasure transaction orchestration, and engine-open sequencing.
Transaction boundaries, frozen views, WAL ownership, and failure ordering must
not change during mechanical file movement.

## Prohibited shortcuts

- Do not create new engine crates without a successor to the accepted topology
  ADR.
- Do not split by arbitrary LOC bands.
- Do not move `Engine` below capability modules that need its private fields.
- Do not broaden all fields to `pub(crate)` to satisfy a preferred tree.
- Do not introduce a giant shared `EngineContext`.
- Do not combine file movement with algorithm or transaction redesign.
- Do not treat equal-sized files as evidence of coherent ownership.
- Do not perform the complete decomposition in one branch.

## Stopping points

Three outcomes are defensible:

1. **Minimal:** extract inline tests and test hooks.
2. **High-value:** additionally extract the strongest independently owned
   production capabilities.
3. **Full:** reduce `lib.rs` to Engine state, contracts, shared controls, and
   deliberate re-exports while moving implementation into capability modules.

Stop when another boundary adds more dependency and navigation cost than the
ownership or retrieval benefit it provides. A uniformly small module tree is
not an acceptance criterion.
