---
title: "Slice 90: source and authority census"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 source and authority census

Read-only frozen source witness: clean parent
`2f6913c1d59ac8ff3a30de835b57290a8635998c` on `llm/0.8.29-chat`, at
`/mnt/data/worktrees/fathomdb-0.8.29`. The app-managed path resolves here.
Only the exclusive phase-a temporary tree is writable in this assignment.
No source/Git/release JSON/generated region/ledger edits, builds, native loads,
database opens, GPU/model/network/paid runs or cleanup were performed.

## Authority and predecessor distinctions

- [Actual90 plan](plan.md), [master](../../../plan-0.8.29.md),
  [direct method](../../direct-execution.md), [checkpoint rulings](../../prework/slice-9/decision-log.md)
  and prework3 needs/CRUD/requirements/acceptance allocate only residual B2 work.
- Accepted [hints ADR](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md)(f)
  authorizes omission; [guard §7.1](../../../../design/long-record-embedding.md#71-now-a-guard-no-new-vectors)
  accepts B2-8 and snippet scored_bytes, preserving Start. It does not approve
  the proposed exact aggregate fields/population/availability.
- [Format35 ADR](../../../../adr/ADR-0.8.28-store-format-35.md) establishes stored
  head spans/unknown offsets; [format37 coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
  governs present visibility state. No .29 migration or backcompat shim.
- Slice10 completed104dcbc;20's integrated657edeb Linux repairs do not close20 policy/
  Windows cells or Block1.30 exact frozen TC5 inputs remain absent.40 has no
  selected optimization.50 identity/census is proposal, with actual baseline/
  snapshot/identity ruling still required.70 actual20-reconciled proposal is
  independently reviewed design, not source implementation.
- Slice60 candidate `2d8278c7290799afec4f9d15f06c5010017a9fd7` is read-only predecessor
  evidence for real formats/filter/budget RED/GREEN. git-show confirms state/
  verdict remain required there; it is not integrated/fully qualified by this
  packet. Latest parent source307c92e3 adds operator/provenance records and is
  still awaiting qualification. Its actual final gate/fresh review remain pending; retain F11's
  unresolved feedback ambiguity consequences and historical-view behavior.
- Slice80 PhaseA's nine files and its aggregate-v2 are proposal inputs. V2 SHA256 is
  `a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`, at
  `/tmp/fathomdb-0829-direct/slice-80/aggregate-design/proposal-v2.md`.
  Its recorded Astra PASS is bounded design review, not full80 owner approval.
  V2 supersedes earlier permanently-missing aggregate presentation, undefined
  carrier, fifth missing-capability entry and optimistic rollback-cache hit.
  No earlier contradictory text is applied by90.

Current50 design/feedback proposal lives under `/tmp/fathomdb-0829-direct/slice-50/phase-a/`;
70 current input under `/tmp/fathomdb-0829-direct/slice-70/actual20-reconciliation/`;
60 reconciled contracts under `/tmp/fathomdb-0829-direct/slice-60/actual20-reconciliation/`.
These are real inspected inputs, not accepted new contracts. Actual successor
reread is mandatory before full-ready/code; this snapshot does not waive it.

## Code-grounded delta

Paths below are repository-relative with frozen-source line witnesses. Only
relevant large source bodies were read; no exhaustive workspace audit is claimed.

| Area | Existing source witness | Remaining proposed work / ownership |
| --- | --- | --- |
| Omission | mcp/src/output.rs:48 Envelope.state, :455 SearchOutput.verdict; tools.rs:323 constructors; orient output:132. | Optional absent state/verdict plus exact disclosure;60 already owns text omission. |
| Arguments | mcp/src/tools.rs:242 generic serde parse; edge.rs typed schemas/discovery. | Shared schema-derived key precheck and typed bounded detail. No nested-map policy change. |
| Feedback | tools.rs:1384 actual membership before epoch/telemetry/text; session.rs ref→cursor BTreeMap. | Ordered bounded offender detail only, after actual50 ambiguity disposition. No new ref identity choice. |
| Default | tools.rs:375 samples session once and builds flags/hint; edge.rs:31 instructions. | Required top-level nullable default, explicit header and guide. Existing routing preserved. |
| Storage | engine/src/vector_spans.rs:3 original EXISTS invalid/orphan predicate; projection_commit.rs transactional tuple publication. | Preserve storage/validation. Aggregate stream adds counters within existing pass; per-hit reads new reporting only. |
| Cache/status | engine/src/open.rs:1254 full validation; projection_generation.rs:1318 full completion, :1462 cache, :1796 runtime status; projection_runtime.rs:96 carrier. | New writer-local summary/token and status field; no universal existing coverage authority. No orient full-status scan. |
| Scored hit | engine/src/snippet.rs:127 lacks scalar; search.rs:2944 builds sidecar on tx; :2684 explanation ranks body-keyed. | Exact accepted50 selected key and actual vector participation; explanations/branch/body cannot supply proof. |
| Rust exposure | engine/src/lib.rs and sdk/src/lib.rs:137 re-export SnippetV1; facade src/lib.rs runtime types. | New aggregate returned types and scalar exposed through existing carriers;70 separately owns six missing facade types. |
| Native snippets | py/src/snippet.rs:46/:56 and napi/src/snippet.rs:48/:59 explicit fields; both read_search.rs convert sidecar. | Add Option scalar converter; do not assume auto propagation. |
| Native status | py/src/projection.rs:453/:461 and napi/src/projection.rs:420/:429 explicit runtime fields. | Required tagged coverage conversion plus exact numeric limits. |
| Python | fathomdb/types.py:351/:440; _sdk_search.py:163; read.py:514. | Frozen aggregate types/decoder + dataclass scalar and explicit maps; exact ints/None/0. |
| TS | src/projection.ts:160; binding.ts:257/:367; read.ts:216; search.ts:73/:344. | Public/native aggregate union/decoder decimal strings for counts/bytes/fractions only, numeric profile.dimension and optional scoredBytes checked u32. |
| MCP formatting | tools.rs orient/search_page and output.rs Hit/ProjectionStatusOut.60 candidate supplies selected formats. | Lift coverage once, scalar from paired snippet; exact compact JSON coverage line and optional scored_bytes pre-pipe; extend both schema/accounting. |

Rust paths start `src/rust/crates/fathomdb-`; Python paths start `src/python/`;
TS paths start `src/ts/`. [Design](design.md) specifies proposed deltas without
changing that existing ownership. Scope preservation: built B2-2/3 hints,
partial6 default signals,7 queue and8 spans are controls, not missing features.
Existing physical completion's cross-owner/vector refusals are distinct from
aggregate/open span predicate; reporting never broadens the latter.

Fully read test-file claims are limited to schema.rs/stores.rs/feedback.rs,
and native PyO3/NAPI snippet modules. Other tests were located by names and
relevant sections; [test-map](test-map.md) explicitly schedules full reread
before any load-bearing assertion/change. Runtime qualification was not run.

Verified memory files in canonical main-repo store: agent-access-block1-design-state.md,
mcp-adapter-proposal-owner-decisions.md, tdd-red-must-fail-per-finding.md,
keep-measurement-tools-in-repo.md, verify-design-against-code-not-just-architecture.md
and slice-prompt-verify-test-claims.md. Their actual files were read. Apply current
release/checkpoint authority where historical memory predates settled scope.
