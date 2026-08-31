---
title: 0.8.25 Slice 4 — architecture and code-alignment review
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 4 — architecture and code-alignment review

## Verdict

The v2 architecture has the correct product boundary and dependency order. It
should become the successor architecture after five clarifications are made:
snapshot reproducibility, byte-span identity, current-state naming,
dependency-liveness semantics, and explicit wire/version evolution. The code
contains strong identity, validity, retrieval, lifecycle, projection, and
explanation substrates, but none of those substrates makes Slices 15–60 a
facade-only exercise.

No architecture authority, source, schema, interface, or test changed here.

## Proposed architecture corrections

| ID | Proposed correction | Reason / owning slice |
| --- | --- | --- |
| A25-01 | Define a frozen snapshot by observable semantics, not by presumed implementation: every operation either reproduces the same canonical boundary, validity instant, projection generation, and eligibility envelope or returns typed snapshot-unavailable/drift/expired. An opaque token need not hold a reader transaction indefinitely. | Current `ReadView` is a value policy, while each call opens its own deferred transaction; `search_expand` explicitly uses two snapshots. Slice 35. |
| A25-02 | Define exact source locators as UTF-8 byte ranges over a named immutable record revision, with an explicit hash algorithm and canonical byte representation. Reject invalid boundaries and hash mismatch. | “Exact span” is otherwise ambiguous across Rust strings, Python Unicode, and TypeScript. Slices 15/50. |
| A25-03 | Name the Slice 45 surface governed **current operational state** backed by `operational_state`; treat `latest_state` as a consumer concept, not a new table or semantic truth policy. | Prevents a facade name from being mistaken for a schema/semantic-consolidation contract. Slice 45. |
| A25-04 | Require each multi-source dependency to declare a small Engine-known liveness rule and define removal consequences without deciding semantic truth. At minimum, “all required” and “any surviving” need distinct closure tests; unsupported rules reject typed. | Architecture already assigns policy to the caller but does not bound the executable rule grammar. Slices 20/30. |
| A25-05 | Require every new persisted/public type to carry schema/wire version and unknown-field/unknown-variant behavior; additive SDK adoption must preserve compact default search-hit cost. | The plan requires parity but not explicit evolution semantics. Slices 15–60, audited at 75. |
| A25-06 | State that constraint and eligibility evaluation precede both seed/candidate truncation and graph expansion; explanation must distinguish ineligible, not-selected, and unavailable. | Prevents a nominal filter from being applied after top-K or BFS caps. Slices 35/55/60. |
| A25-07 | Keep `SearchHit` compact, but allow an opt-in evidence handle only when its creation does not bypass the bound visibility envelope. The resolver returns typed non-disclosure, never stale bytes. | Preserves the accepted v1 correction and current fast path. Slice 50. |

## Code-alignment matrix

| Slice | Existing seam | Verified mismatch and required direction |
| ---: | --- | --- |
| 10 | Typed experiment configs/receipts and completed performance results | Receipt schemas do not uniformly prove execution layer or `Engine.search` use. Extend experiment validation, not Engine behavior. |
| 15 | `IdSpace`, mandatory new-write `source_id`, `logical_id`, `write_cursor`, and stable hit identity in `fathomdb-engine/src/lib.rs`; schema steps preserve active logical identity | `write_cursor` is explicitly positional; no immutable record revision, source version, canonical hash, or stored locator exists. Rust facade/export parity also needs proof. Add persisted identity rather than rebranding the cursor. |
| 20 | Row-owned projection linkage and source-based erasure | No public queryable dependency graph or multi-source liveness contract exists. Add a governed persisted dependency substrate with cycle/reference validation. |
| 25 | Atomic `Engine.write`; separate `transition`, `purge`, `erase_source`, and provider-driven consolidation; compact `WriteReceipt` | No one caller-decided transaction spans those operations; receipt contains only cursor/row cursors/dangling count. Compose inside one Engine transaction and keep semantic providers outside. |
| 30 | Active/superseded/inactive validity axes; registry-driven row-owned erasure; WAL/telemetry completion checks | Lifecycle closure stops at row-owned artifacts because registered derived dependencies do not yet exist. Extend the existing transition/erasure machinery after Slice 20/25 rather than replacing it. |
| 35 | Strict `ReadView`; `SearchFilter`; allowlisted `Predicate` equality/range; typed invalid filter | `ReadView` is not a frozen snapshot, search and list grammars are distinct, and combined paths can use different transactions. Unify eligibility semantics and add typed membership/existence only over native projections. |
| 40 | Projection registry, status, drain, cursor, blocked/degraded signals | Readiness lacks a durable generation identity consistently correlated to mutation receipts and restart. Extend existing registry/status records. |
| 45 | `operational_state` table; mutation-log `after_id`; bounded list reads | No governed current-state read, canonical/graph opaque cursor, snapshot binding, mismatch/expiry outcome, or stable continuation contract. Keep ranked search top-K separate. |
| 50 | `SearchHit` contains typed ID, body, score/branch, `source_id`, optional CE score | No revision/source-version/span/hash/dependency/lifecycle handle and no eligibility-bound resolver. Add an opt-in sidecar; do not enlarge every default hit. |
| 55 | `search_explained`, query/per-hit signals, soft fallback, telemetry IDs, projection status, orphan-provenance doctor | No reciprocal source/dependency trace, deterministic exclusion reason, graph path, or cross-receipt correlation. Extend structural explanation; semantic entailment remains external. |
| 60 | `graph_neighbors` has direction/depth/`ReadView`; `search_expand` is bounded and shares graph storage | `search_expand` always expands both directions, lacks edge/target/predicate constraints and a view, returns no edge path, and runs search/expansion in separate transactions. Extend this path; do not add another graph engine. |
| 65 | FTS, vectors, RRF, CE, duplicate handling, experimental profile registry | Some candidate algorithms exist only in experiment code and prior treatments lost answer quality. Promotion must remain benchmark-gated and opt-in. |
| 70 | Valid-time filtering, temporal benchmark evidence, graph arm | No accepted temporal/associative profile; exact-anchor graph bridge had negligible effect. Qualification, not default change, is correct. |
| 75 | Cross-SDK allowlist/conformance, stress/perf gates, release artifact workflows | New contracts have no integrated concurrency, installed-artifact, overhead, lifecycle, or retrieval-only proof yet. Audit feature-local proof rather than deferring basic parity until closure. |

## Architecture-to-code decisions

- **Code should change:** immutable revision/source provenance, dependencies,
  atomic actuation, snapshot/eligibility, generations, pages/current state,
  evidence, traces, and constrained expansion are genuine product gaps.
- **Architecture should change:** A25-01 through A25-07 clarify contracts that
  are currently ambiguous or over-prescriptive.
- **Both should remain unchanged:** semantic interpretation, model/provider
  policy, query decomposition, answer synthesis/verification, and HITL remain
  outside FathomDB.
- **No new store:** canonical tables, operational state, graph tables, and
  projection registry are the intended substrates. New tables/indexes may be
  required for revisions/dependencies/generations, but not a parallel database.

## Slice 7 versus feature allocation

Only current-document authority fixes and release verification infrastructure
belong in Slice 7. Every A25 correction that affects the product or public
contract is allocated directly to Slices 15–60 and recorded in the v2 feature
plan. Slice 6 decides whether v2 is accepted and whether the repository-only
prework changes proceed.

## Principal code evidence

- `src/rust/crates/fathomdb-engine/src/lib.rs`: `WriteReceipt`, `IdSpace`,
  `SearchHit`, `ReadView`, `Predicate`, search variants, `search_expand`,
  lifecycle/erasure, and projection APIs.
- `src/rust/crates/fathomdb-schema/src/lib.rs`: canonical, operational,
  identity, validity, projection, and current-state schema.
- `src/python/fathomdb/{types,engine,read,graph}.py` and `src/ts/src/index.ts`:
  current binding shapes and graph/read limitations.
- Current ADRs/interfaces plus data-plane architecture v1/v2 and the complete
  Memex needs crosswalk.
