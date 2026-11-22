---
title: FathomDB Inspector — Astra-medium assessment of Fable actions
date: 2026-10-28
target_release: 0.8.30
desc: Necessary, recommended, optional, and rejected follow-up actions from the independent Fable review
status: ACTIVE
blast_radius: review prioritization only; no requirements amendment or owner policy ruling
---

# FathomDB Inspector — Astra-medium assessment of Fable actions

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Basis and disposition

At the owner's request, the existing independent Astra-medium reviewer assessed
all eleven findings in the [Fable review](fable-review.md), the
[requirements and acceptance criteria](../needs-requirements-acceptance.md), and
[high-level design](../design/high-level-design.md), against source and contracts at
clean commit `8ba9fc2f9`. This was a read-only review. No further Anthropic run
was made.

Astra's assessment: Fable identified useful follow-up work but overstated
several premises and treated too many engineering choices as product decisions.
The proposal can proceed to detailed design. Necessary actions below must be
addressed at their stated stage; implementation obligations are not all
requirements blockers today. The structure-appropriate document, lexical,
vector, and interactive graph views remain mandatory.

This document records the reviewer's assessment. It does not amend the
requirements, implement remediation, or record a human policy decision.

## NEED — required corrections or delivery obligations

| Fable finding | Action | When required and why |
| --- | --- | --- |
| 1 — disclosure | Define what operator inspection may disclose separately from normal retrieval eligibility. Distinguish retained inactive/superseded data, owed physical erasure, and telemetry-redaction bookkeeping. Add distinct fixture cases and deliberately specify refusal, withholding, or diagnostic access. | Requirements before affected reads are designed. Existing non-reconstruction rules do not settle retained-content visibility; this is a genuine product-policy question. Do not automatically adopt blanket nondisclosure. |
| 2 — incomplete provenance | Expose recorded `legacy` / `migrated_incomplete` provenance and add fixtures using current ordinary writes. Do not imply every canonical record is a fully proven source or derived record. | Requirements and fixtures. Current ordinary node/edge writes can produce these values; no migration scenario is needed to justify coverage. |
| 4 — eligibility by class | Specify/test node state and validity separately from edge validity and dependency closure. Include an inactive node and an expired edge; do not fabricate a node-style edge state field. | Detailed design and tests before implementation. Generic eligibility wording needs class-correct execution and presentation. |
| 5 — vector semantics | Specify metric, representation, candidate population, and selection procedure per neighborhood operation. State whether it follows binary shortlist plus f32 L2 reranking or an independently defined exact operation; disclose relevant centering and define F-07's distance oracle. | Vector detailed-design contract. Exact candidate distances do not establish globally exact nearest neighbors. Both stages need not be separate UI controls. |
| 6 — lexical attribution | Identify actual index/fields, tokenizer, and scoring route for each supported lexical request. Distinguish indexed content from canonical body and BM25 from BM25F. Verify routes rather than treating every historical table as an active projection. | Detailed design and conformance tests. Necessary for truthful scores, evidence, and origins. |
| 8 — schema support | Explicitly state the initial supported-schema policy; current compiled schema only fits the existing admission precedent. Test both older- and newer-schema refusals with accurate diagnostics. | Admission contract before implementation. Older-format support is additional scope, not implied current functionality. |
| 11 — cached runtime | Establish/test truly cache-only query and optional reranker initialization. Missing, partial, or corrupt assets must not cause downloads or cache repair. Assess reuse of existing checksum-verifying local-assets code. | Implementation obligation before claiming INS-X01; not a new requirements blocker. The no-download requirement already exists. |

### Evidence for necessary actions

- [Erasure bookkeeping](../../../src/rust/crates/fathomdb-engine/src/erasure.rs): the pending-redaction queue concerns telemetry identifiers and does not itself retain erasable payload. Application read eligibility and operator disclosure are separate questions.
- [Write commit](../../../src/rust/crates/fathomdb-engine/src/write_commit.rs): ordinary node/edge registration can record legacy/incomplete provenance.
- [Temporal rules](../../../src/rust/crates/fathomdb-engine/src/temporal.rs): node state/validity and edge temporal rules differ.
- [Vector candidate filtering](../../../src/rust/crates/fathomdb-engine/src/filter.rs) and [search](../../../src/rust/crates/fathomdb-engine/src/search.rs): bounded binary selection precedes f32 L2 reranking; lexical execution also owns distinct routes and scoring.
- [Open implementation](../../../src/rust/crates/fathomdb-engine/src/open.rs): public admission rejects noncurrent nonempty schemas rather than migrating them.
- [Embedder loader](../../../src/rust/crates/fathomdb-embedder/src/loader.rs): the default path can create cache directories; checksum-verifying local-assets code exists behind `tc5-benchmark` as a factoring candidate.

## SHOULD — substantive recommendations

| Fable finding | Action | Why recommended rather than a prerequisite now |
| --- | --- | --- |
| 1, 3 — engine-owned data | Label engine-owned operational bookkeeping; preserve structural role/provenance; test coverage/graph roles for supported inputs and make count filters explicit. | Improves inventory clarity. Fable's cited structural-row writer is test/test-hooks gated, so routine production prevalence and mandatory separate aggregate counters are not established. |
| 7 — related access layers | Add a short source-grounded reuse assessment for MCP/agent-access, shared query semantics, Inspector-owned connections, and cancellation. Reconcile scratch writes with query-only execution: current BM25F tokenization creates/mutates temporary FTS tables. | Important detailed-design risk reduction. The design already identifies factoring and connection ownership as new work; one abstraction across all services is not automatically necessary. Supported lexical execution must still pass conformance/preservation checks. |
| 9 — WAL refusal | Give actionable non-destructive guidance: preserve database/sidecars, stop owners, investigate outstanding state. Add the refusal message to F-05. | Implementation/UX improvement within current quiescent scope. WAL-present support is not required; destructive recovery is not the routine remedy. |
| 10 — visual acceptance | Add automated graph identity/direction/expansion, lexical-evidence, and vector-measurement checks, plus explicit human selection, expansion/collapse, and cross-view navigation checks. | Strengthens test design. Mandatory interaction forms coexist with the rule that arbitrary pixel layout is not the oracle. |

Evidence: [engine write paths](../../../src/rust/crates/fathomdb-engine/src/write.rs)
contain the test gate; [search implementation](../../../src/rust/crates/fathomdb-engine/src/search.rs)
contains temporary FTS tokenization work; [inspection guidance](../../../docs/operations/integrity-inspection.md)
already supplies non-destructive refusal guidance.

## NICE-TO-HAVE — optional enhancements

### Deeper inspection

- Finding 5: side-by-side binary-candidate and f32-neighborhood comparison beyond correct attribution of the selected operation.
- Finding 6: inactive/historical lexical-structure inspection beyond supported active routes.
- Finding 3: role-distribution dashboards and dedicated engine-derived aggregate counters beyond clear labels and scoped counts.

### Broader compatibility and availability

- Findings 2/8: older-schema adapters and a historical-file compatibility corpus.
- Finding 9: read-only attachment to a consistent WAL-present database, with separate admission design and validation.

### Cross-service consolidation

- Findings 6/7: reuse of agent-access snippets or a shared session abstraction where source-grounded analysis establishes a benefit. Assess first; another branch's code is not already present here by implication.

## Reject or reframe

- Finding 1: telemetry-redaction identifiers do not prove retained erased canonical payload. Trace each obligation and retained data independently.
- Finding 2: legacy prevalence is unproven, and a current public migration fixture is invalid. Current ordinary incomplete-provenance writes are the sounder reason for coverage.
- Finding 3: the cited writer is test-gated; existing role labeling does not automatically misclassify records as caller-authored.
- Finding 4: consistent terminology does not require identical lifecycle axes. Add class-specific tests without asserting an existing contradiction.
- Finding 6: historical schema declarations do not establish active routing or registered projection status; sibling snippet work is a reuse candidate.
- Finding 7: shared session architecture is an engineering option, not necessarily a human product ruling.
- Finding 8: no public migration-on-open fallback exists on the reviewed branch.
- Finding 9: reject lossy recovery as a routine inspection remedy.
- Finding 10: retain the owner-required graph canvas and structure-specific views.
- Finding 11: no-download is already required and some reusable code exists; integration and verification remain delivery work.

## Product decisions versus engineering choices

The genuine unresolved human decision is inspection disclosure policy where
existing contracts do not settle it. Historical-schema expansion, WAL/live
attachment, and distribution commitments are product-scope decisions if
pursued. A conservative current-schema, quiescent design does not require
asking for all those expansions now.

Loader factoring, shared internal abstractions, connection ownership, payload
contracts, metric attribution, and test mechanics are ordinarily engineering
choices subject to existing API/ADR rules. They do not each require a new
approval merely because Fable called them product decisions.
