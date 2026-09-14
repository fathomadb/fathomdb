---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence
status: IMPLEMENTATION PAUSED — GPT-6 ASTRA MEDIUM DESIGN CHANGES REQUIRED
decision: D26-01 option A at seq-290
---

# Slice 20 plan — exact graph artifact evidence

## Purpose and outcome

Let a consumer take a target or winning terminal edge returned by constrained
graph expansion and resolve that exact immutable artifact revision as canonical
evidence under the same frozen read authority. This removes probabilistic body
re-search while preventing raw revision identity from becoming authorization or
an enumeration route.

The selected public shape is D26-01 option A: an opt-in first-generation V1
evidence sidecar on the existing V1 graph operation. Ordinary graph requests
retain their literal response path. This slice adds no parallel V2 operation,
logical-ID lookup, full-path proof, schema migration, or compatibility shim.

## Reconciliation since the draft

1. Slice 10 repaired frozen explanation finalization without changing graph
   evidence semantics.
2. Slice 15 completed at `422b2679`, measured the two shapes, proved erasure
   ordering, retained the ordinary-response byte oracle, and restored transient
   product changes before close.
3. HITL `seq-290` selected option A and unblocked this slice. References to a
   still-open public-shape decision are rejected.
4. The reviewed Slice 15 FIX-2 prototype at `06a54027` supplies the production
   starting point: two class-specific joined hydration statements, authenticated
   graph-disclosure references, intrinsic resolution, class-aware eligibility,
   complete-provenance refusal, and primary-connection erasure ordering.
5. Graph expansion already owns a reader-pool transaction through final
   selection and frozen revalidation. Hydration and reference minting stay in
   that transaction; only point resolution uses the primary connection.
6. Every returned target has a winning terminal edge. Each sidecar entry therefore
   carries required target and terminal-edge identities and references.
7. The prototype admitted only derived-semantic artifacts. Production must also
   resolve canonical-source graph targets with self-source provenance and no
   direct dependency.
8. Existing schema-33 revision/provenance storage is sufficient. Any migration
   or durable cache is outside the approved release boundary and is a stop gate.
9. Slice 15 settled comparative performance. Slice 20 retains bounded-query and
   ordinary-path regression checks but does not repeat the measurement campaign.

## Need, requirements, and acceptance criteria

**N26-20:** Memex needs exact, canonical evidence for a graph target or winning
terminal edge under the graph disclosure's frozen authority, without body
re-search or an ID-addressable disclosure surface.

- **R26-20A — opt-in carrier:** Add `include_evidence`/`includeEvidence`, default
  false, to `GraphExpandRequestV1`. When true, require a frozen context and
  return a positional `GraphEvidenceSidecarV1`; omit the result member entirely
  otherwise so the ordinary wire bytes remain unchanged.
- **R26-20B — exact identities and references:** Each entry binds one target
  position to required target and terminal-edge `ArtifactRevisionId` strings
  plus nominal `GraphEvidenceRefV1` opaque references. Stable IDs support
  joining but confer no resolution authority; internal write cursors are never
  public.
- **R26-20C — intrinsic resolver:** Add `resolve_graph_evidence`/
  `resolveGraphEvidence` accepting only an opaque graph evidence reference and
  equivalent `FrozenReadContextV1`. Return `ResolvedGraphEvidenceV1` intrinsic
  artifact/source facts, canonical bytes and selected text/locator, lifecycle,
  and an optional direct dependency. Canonical-source nodes always have none;
  derived nodes/edges may have none, but a registered dependency must match the
  artifact/source and exact generation/closure state. Do not synthesize score,
  rank, contribution, projection
  origin/generation, or path evidence.
- **R26-20D — atomic disclosure:** Hydrate and mint only after final graph
  selection, inside its reader transaction. Refuse the whole evidence-bearing
  expansion if any selected target or edge lacks complete valid provenance.
  Independently authorize canonical source bytes with status, created-after,
  attributes, lifecycle, supersession, validity, and closure, without applying
  artifact kind/source-type predicates to a differently typed source.
- **R26-20E — nondisclosure and linearization:** Bind references to database,
  frozen eligibility, graph request/disclosure, target position, artifact role,
  and selected target/edge revisions. A confidentiality-protected authenticated
  selector carries bounded internal cursors and revalidation claims; public
  identities remain outside the token. Authentication and authority precede
  materialization. Point resolution uses one primary-connection transaction and
  the existing before-return rendezvous so erase/excise cannot complete first.
- **R26-20F — complete public parity:** Rust, Python, TypeScript, canonical JSON,
  interface docs, public docs, accepted ADR, and governed-surface metadata agree.
  Dynamic decoders retain closed fields, exact RFC 6901 paths, strict booleans,
  validated artifact-revision strings, canonical encodings for actual numeric
  fields, and sidecar coherence.

Acceptance criteria:

- **AC26-20A:** Absence or false evidence selection produces byte-identical
  ordinary graph JSON. A true selection without frozen authority fails at
  `/context`; a true zero-target result contains a present empty sidecar.
- **AC26-20B:** Every nonempty sidecar is positional and length-equal to targets;
  each entry exposes both exact stable revision identities and both opaque
  graph-disclosure references, including the winning parallel-edge revision.
- **AC26-20C:** Target and terminal-edge resolution return exact canonical-source
  and artifact facts after unchanged restart. Canonical-source nodes have no
  direct dependency; derived nodes/edges work both without a dependency and
  with an exact registered dependency generation.
- **AC26-20D:** Tampered, foreign, mismatched-context, drifted, out-of-window,
  ineligible, revoked, superseded, erased, closure-fenced, and nonexistent
  references collapse to the nondisclosing evidence-unavailable contract.
- **AC26-20E:** Corrupt hash/locator or incomplete provenance on either selected
  class prevents any sidecar. Target filters are rechecked for nodes but are not
  misapplied to terminal edges or canonical sources. Derived node/edge artifacts
  that pass while their source attribute/status/time eligibility fails remain
  nondisclosing; canonical-source self-rows are covered. Resolution and
  erase/excise ordering is proven.
- **AC26-20E1:** After authenticated selection, a missing or ineligible canonical
  source refuses the whole expansion as `evidence_unavailable` at `/evidence`
  without identifying the target or source; all bindings preserve that envelope.
- **AC26-20F:** Empty results issue no hydration data statement. Nonempty results
  use exactly two bounded class-specific indexed data statements over at most
  50 target cursors plus 50 edge cursors, independent of traversal work. SQL
  inputs deduplicate by cursor while reconstruction preserves positional entries
  and shared-source associations; source validation/hashing is deduplicated.
- **AC26-20G:** Rust/Python/TypeScript round trips and installed-package probes
  agree; interfaces and public examples describe the same V1 contract; no V2,
  raw-ID resolver, migration, new table, or ordinary-path response drift exists.

## Design and authorized surfaces

The detailed design is in `design.md` and requires independent review before
RED. Authorized surfaces are limited to:

- graph/evidence engine carriers, codecs, hydration, resolver, facade exports,
  and focused Rust tests;
- Python and N-API bridges, Python/TypeScript V1 carriers, validators, methods,
  declarations, and focused real-engine tests;
- one accepted ADR plus the Rust/Python/TypeScript/wire interface documents,
  frozen-evidence guide, API references, changelog, and exact governed-surface
  allowlist/pin update for the new resolver;
- Slice 20 plan, design, TDD chronology, review/verification evidence, and status.

No unrelated refactor, dependency update, full-path evidence, batch resolver,
reader-pool point resolver, durable evidence cache, schema change, or release
publication is authorized.

## TDD implementation sequence

1. Commit the reviewed plan/design and record design-review findings.
2. **RED:** add Rust contract/engine tests first for request/result wire behavior,
   positional coherence, winning-edge identity, canonical and derived resolution,
   optional dependency, source-byte authorization, provenance refusal,
   nondisclosure, non-vacuous fixed-nonce request-commitment binding, restart, bounded SQL, and
   erase/excise ordering. Preserve the failing test commit and exact diagnostics.
3. **GREEN core:** promote the reviewed FIX-2 mechanisms with production token
   domains and minimal public carriers. Retain final edge cursors internally,
   hydrate after selection in the same reader transaction, and add the primary-
   connection intrinsic resolver.
4. **RED/GREEN bindings:** add failing Python and TypeScript codec/API/real-engine
   tests, then implement both bridges and SDKs. Add property tests for round trip,
   coherence, valid nonnumeric revision identities, invalid identity grammar,
   closed variants, canonical numeric fields, and per-field/one-byte token tampering.
5. **Docs/governance:** accept the ADR, update all four interface contracts,
   public guide/references/changelog, and register the exact new governed resolver
   surface and pin. Ordinary `graph.expand` remains the same governed operation.
6. Refactor only after GREEN without weakening or regenerating human-authored
   oracles. An independent code-review subagent reviews the actual committed diff;
   perform no more than two focused FIX-n cycles.

## Verification and closeout

- Run focused graph, evidence, frozen-authority, erasure, codec, Python, and
  TypeScript suites after the relevant change, not the full matrix after each fix.
- Run `./scripts/agent-verify.sh`, workspace Clippy with warnings denied, workspace
  check, strict docs build, and fresh installed Python/TypeScript package probes.
- A read-only verifier subagent independently checks functional, nondisclosure,
  query-bound, cross-binding, public-surface, TDD, and Git evidence.
- Write the Slice 20 status with exact RED/GREEN/review/gate SHAs and unresolved
  facts. Mark complete only after both reviews pass.
- Fast-forward the reviewed branch to `release/0.8.26`, update the release-state
  single writer and generated views, run post-landing preflight, then remove the
  clean temporary worktree and branch. Slice 30 is next.

## Stop gates

Stop for HITL on any schema migration/table/cache, raw-ID or logical-ID resolver,
split graph authorization/materialization transaction, existence disclosure,
rank synthesis, parallel V2 surface, dynamic-binding incompatibility requiring a
broader design, ordinary-response byte drift, or a performance regression that
invalidates Slice 15's accepted decision evidence.
