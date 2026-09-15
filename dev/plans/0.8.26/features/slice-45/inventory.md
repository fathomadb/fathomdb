---
title: FathomDB 0.8.26 Slice 45 — architecture inventory and claim matrix
status: APPROVED
---

# Slice 45 architecture inventory and claim matrix

## Document disposition

| Document | Observed state at `7a34a668` | Disposition in Slice 45 |
| --- | --- | --- |
| `dev/architecture.md` | Locked 0.6.0 snapshot still calls itself authoritative; crate, schema, read/write, and open-path details are not current. | Retain path and technical body as history; mark superseded, reframe its opening authority statements as historical, and add a mechanically checked successor banner. Do not rewrite its unique 0.6.0 rationale. |
| `dev/design/fathomdb-data-plane-architecture-v1.md` | Correctly superseded by v2. | Keep unchanged. |
| `dev/design/fathomdb-data-plane-architecture-v2.md` | Sole active architecture, but v2.1 stops at the 0.8.25 executable profile. | Update in place to v2.2 with an as-built 0.8.26 profile and explicit authority links. No new architectural decision. |
| `dev/README.md` | Lists `architecture.md` as canonical without identifying the active successor. | Update the engineering-reference and canonical-doc navigation to distinguish current versus historical. |
| `dev/design/README.md` | `UNREVIEWED`; declares 0.8.25 v2.1 current and carries a stale manual topic list. | Retain file-level `UNREVIEWED`; update only the architecture-navigation paragraph. Full file/topic classification belongs to Slice 46. |
| `dev/interfaces/README.md` | Describes 0.8.25 deltas as planned successors. | Point to the maintained 0.8.26 records while stating that section-local status controls; do not characterize every contained section as implemented or edit detailed contracts. |
| `dev/adr/README.md` and `ADR-0.6.0-decision-index.md` | Decision role and 0.8.26 rows are accurate. | Keep index content; add current architecture/index navigation only if needed. |
| `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md` | Correct accepted ruling, but its chronological “graph shape remains open” statement predates `seq-290`. | Preserve decision text; add a non-substantive subsequent-resolution pointer to the graph-evidence ADR. |
| `ADR-0.8.26-exact-graph-artifact-evidence.md` | Correct later accepted decision. | Add relationship metadata/pointer only; do not change substance. |
| `dev/notes/0.8.23-architecture-tradeoffs.md` | Bounded earlier-release follow-up input, not current authority. | Keep as reference. Slice 45 does not claim all questions are resolved. |
| `dev/DOC-INDEX.md` | Rows still describe `dev/architecture.md` as current and v2 as 0.8.25-only. | Correct touched architecture/navigation rows. Broad per-design indexing belongs to Slice 46. |
| Slice 10–40 plan/design/status records | Historical execution evidence with exact requirements and checks. | Keep unchanged; cite as evidence, never promote to current architecture authority. |
| Slice 46 draft | Owns maintained topic-design lifecycle and detailed reconciliation. | Keep boundary intact; consume this matrix next. |

No file is deleted, moved, or archived. No uncertain document is reclassified.

## As-built claim witnesses

| Current architecture claim | Decision/contract witness | Implementation witness | Verification witness |
| --- | --- | --- | --- |
| Ten Rust workspace members with a thin facade, engine, CLI, schema/query/embedder crates, dynamic bindings, and benchmark harness | crate-topology ADRs in the decision index; `dev/interfaces/rust.md` | root `Cargo.toml`; the ten named `src/rust/crates/*` member paths | direct `Cargo.toml` member enumeration during design review and independent verification |
| Missing/zero-length paths bootstrap at schema 34; nonempty noncurrent databases refuse before product mutation | breaking-boundary ADR; Rust/Python/TypeScript/wire “Fresh-database open boundary” sections | `fathomdb-schema::SCHEMA_VERSION`; open admission in `fathomdb-engine/src/lib.rs` | `slice40_fresh_database_cutover.rs` and Slice 40 status |
| The primary mutex-serialized writer connection owns caller mutation transactions; projection-worker connections publish async vector outcomes through a total-order `commit_gate`; reader-pool transactions own frozen search, graph selection, and evidence materialization | 0.6.0 single-writer ADR as refined by ratified 0.8.14 EXP-S D2/D5; frozen-read/evidence ADRs; current interfaces | `Engine::write_inner`; `Engine::actuate`; `projection_worker_loop` and `commit_projection_outcomes`; `frozen_read.rs`, `graph_expand.rs`, `evidence.rs` | projection ordering tests plus Slice 20 and Slice 35/40 suites/status records |
| Both explained frozen-search paths return finalized non-empty correlation identity without changing explanation-off semantics | current Rust/Python/TypeScript/wire explanation contracts | search finalization in `fathomdb-engine/src/lib.rs` | `slice10_frozen_explanation.rs`; Slice 10 wheel witness/status |
| Graph evidence is an opt-in positional V1 sidecar with frozen-only opaque exact target and terminal-edge resolution | exact-graph-evidence ADR; current Rust/Python/TypeScript/wire contracts | `graph_expand.rs`, `evidence.rs`, binding conversion layers | Rust/Python/TypeScript Slice 20 tests and installed package probes |
| Integrity inspection is bounded, immutable, version-matched, out of process, and absent from governed Python/TypeScript APIs | CLI interface; governed-surface contract | CLI `data-plane-integrity`; engine `data_plane_integrity.rs` | `slice30_operator_integrity_cli.rs`; Slice 30 status |
| Actuation has one changed-in-place five-operation V1 grammar; derived edges reuse canonical edge storage and require endpoints in the complete prospective batch | breaking-boundary and bounded-actuation ADRs; current binding/wire contracts | `ActuationOperationV1` and `Engine::actuate` in `actuation.rs` | shared Slice 35 fixture; Slice 25/35/40 actuation tests |
| FathomDB owns durable/provenance/visibility mechanisms while Memex owns extraction, truth, answer, model, spend, and HITL policy | bounded-actuation, exact-evidence, and frozen-read ADR boundaries; D26-08 scope ruling | Not applicable: this is a normative ownership boundary, not an absence-of-code claim. | independent design review against the governed interface inventory and accepted scope |

## Rejected expansion

- No rewrite of the locked 0.6.0 body.
- No bulk audit or lifecycle rewrite of `dev/design/`; Slice 46 owns it.
- No product/API/schema/package change and no new ADR decision.
- No claim that deferred multi-source provenance, snapshot leases, graph
  continuation/full paths, or semantic policy shipped in 0.8.26.
- No integrated package/platform or publication evidence; Slice 50 owns it.
