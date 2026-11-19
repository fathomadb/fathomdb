---
title: Slice 30 correctness bug hunt and remediation
status: ACTIVE
slice: 30
target_release: 0.8.30
---

# Slice 30 — Correctness bug hunt and remediation

Make retrieval, attribution, coverage and open failures agree with their contracts on a small set of real-store cases.
This draft refines R30-COR/S50 and owns the correctness portion of R30-TEST under the [release plan](../../plan.md).
It takes its first cases from the [findings](../../testing-robustness-findings.md), [recommendations](../../quality-performance-robustness-recommendations.md) and [receiving allocations](../../release-followups.md).

Commissioned at seq-402 for implementation using the supplied slice prompt template.

## Baseline and allocation decisions

- Release baseline `5f4edd9831ddbb5ee12ebe594a3f6cd17fcc46b8` commissions
  the slice (seq-402) after 10 and 20. The worktree is based on that commit.
  Since reviewed draft `f7fb1da9a81ee226c70ae347fa2fab3e64ac3c25`, the
  affected Rust search, projection, MCP and admission source, interface
  contracts, and typed-error fixtures have no changes. Fresh `origin/main`
  at `b86b19847852e9beaeea32b98dae03b537d8fea4` has no new changes in
  those paths beyond the release. Resolution has advanced Tokio 1.52.3 to
  1.54.1 and webpki-roots 1.0.7 to 1.0.12. Preserve both.
- Adopt prework [3](../../prework/slice-3.md) and [4](../../prework/slice-4.md):
  exact same-state output, canonical attribution, atomic feedback, snapshot
  witness and committed coverage. Format 37, existing reader and writer
  boundaries, and [embedder-owned identity](../../../../adr/ADR-0.6.0-vector-identity-embedder-owned.md)
  apply under the [architecture](../../../../design/fathomdb-data-plane-architecture-v2.md)
  and [ADR index](../../../../adr/ADR-0.6.0-decision-index.md). Reject any
  new persisted snapshot stamp, write lane or cache.
- Adopt prework [5](../../prework/slice-5.md) and [9](../../prework/slice-9.md)
  P9-11: add independent rank and coverage oracles where existing assertions
  leave a gap. Map and retire the self-equality query scaffold. Keep the
  existing eligibility-before-cap, feedback validation and typed-admission
  mechanisms. Leave performance treatment, reset/model lifecycle, fork work,
  integrated qualification and publication with their assigned owners.
- Adopt the findings and recommendations for one bounded correctness hunt,
  author-derived expectations, selected committed/rolled-back state, and
  paired reference comparison. Defer broad mutation, coverage, fault and
  resource campaigns under the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
  Slice 10/20 added source-bound developer setup and diagnostic routing.
  Use checkout-owned Python 3.12 and npm dependencies, Rust 1.95 and dev
  Node 26.13.1. Preserve the selected SDK Node support range and inherited
  unqualified outcomes in their owning slices.

## Local requirements and acceptance

These are release-local refinements. The canonical acceptance suite remains authoritative.

### R30-S50 — Explain or delimit the natural-write score difference

- Need: distinguish rank movement between reader states from a same-state wrong result. First inspect the retained reports, both sample records, source pins and final databases indexed by `/mnt/data/fathomdb/0.8.29-S50-data/README.md`. Keep the archival evidence intact.
- The G-W b10/s1 SDK-hybrid diagnostic differs at sample 860, query 360, operation 9971. Eager scores are 1.240362% and 0.850694% higher for two hits, with matching ordered document IDs and text. Both calls used the reference route. The cursors were 108610 and 108600. Reconstructed ranks support a hypothesis, while the missing historical candidate lists and reader states leave asynchronous projection and tie order unresolved.
- Use one bounded CPU reproduction on an owned real store with one query and a controlled projection step. Capture the selected route and relevant candidate record IDs, raw vector distances, original arm positions, tie groups and resolved tie order, reader snapshot witness and projection cursor. Capture both sides at the ranking read, with the same query inputs and validity window. A cursor value alone cannot identify the complete reader state.
- Pair the moving-state case with a fixed-state control exercising the supported reference and optimized routes with route witnesses. Require [S50-A3](../../../0.8.29/features/slice-50/requirements.md) exactness for ordered fields, score bits, cursor, sidecars and applicable error/fallback metadata on matched state and inputs.
- Acceptance: independently recompute affected scores from captured original ranks and identify the input or ordering difference, or report precisely what the bounded probe leaves unresolved. Discuss a state-aware natural-write comparison design and the evidence it would require. Preserve the original failed comparison, qualification credit and charges. Add no blanket tolerance, retrospective pass or full 160-member replay.

### R30-COR — Preserve stored-record and response attribution

- Need: readers can distinguish records with identical stored bytes, and feedback identifies the occurrence actually returned in the named response.
- Trace candidate ranking and late hydration in [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs), and `FusionHit`, `RecordKey` and `fuse_records` in [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs). Use canonical node/edge plus cursor, or the actual ownerless FTS locator. Start with the [record identity suite](../../../../../src/rust/crates/fathomdb-engine/tests/slice50_record_identity.rs) and [lightweight controls](../../../../../src/rust/crates/fathomdb-engine/tests/slice50_lightweight_controls.rs).
- Acceptance: equal-body records retain distinct owners, cursors, source and sidecars, including matching public refs and tagged bodies. One record seen in both arms gets one contribution per arm at its first original position. Repeats within an arm leave later original ranks intact. Assert exact rank and representative provenance from fixture rows. Preserve the selected duplicate-ID refusal in context packing.
- Trace `fold_response_refs` in [session state](../../../../../src/rust/crates/fathomdb-mcp/src/session.rs) and scoring-page retention and `feedback` in [MCP tools](../../../../../src/rust/crates/fathomdb-mcp/src/tools.rs). The map covers kept hits in one response. Check repeated refs within a page and across continuation responses against the [MCP contract](../../../../interfaces/mcp.md).
- Acceptance: in the [feedback suite](../../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs), an otherwise recordable call mixing unique and ambiguous refs across `used` and `unused` returns the typed refusal with zero label writes. A repeated occurrence of one cursor remains unique. A unique label in its own response reaches its expected query and cursor. Preserve store, membership, telemetry, text, acquisition and final epoch precedence.
- Review exact occurrence labels only. Describe candidate schema and compact rendering, binding to response/store/epoch, paging and budget lifetime, validation and the distinction from latest-eligible logical fetch. Record the owner choice or defer any ABI change. For a selected change, require exact attribution and atomic handling of stale, foreign and mixed labels in both formats.

### R30-COR — Report committed coverage, scored extents and native failures truthfully

- Need: coverage reflects committed mapped heads, snippets report extent from the actual participating record, and callers can distinguish native admission failures.
- Trace `capture` and `scored_extent` in [vector spans](../../../../../src/rust/crates/fathomdb-engine/src/vector_spans.rs), token validation and publication in [stored coverage](../../../../../src/rust/crates/fathomdb-engine/src/stored_coverage.rs), and [snippet selection](../../../../../src/rust/crates/fathomdb-engine/src/snippet_query.rs). Use the [stored coverage contract](../../../../interfaces/rust.md#stored-mapped-head-coverage-0829), [snippet design](../../../../design/agent-access/match-centred-snippets.md) and [existing engine tests](../../../../../src/rust/crates/fathomdb-engine/src/stored_coverage_tests.rs).
- Acceptance: independently enumerate attributable mapped heads. Check `R=K+U`, truncated known count, body and unrepresented bytes, exact fractions and denominator-zero nulls. Distinguish NULL from known zero and count unattributed heads separately while preserving original corruption refusals. Include retained history and an unmapped record.
- With a deterministic reader rendezvous and existing transaction controls, distinguish committed results, rollback, conservative invalidation and frozen refusal. A pinned reader returns the old exact endpoint or the contract's frozen refusal. A new ordinary read sees committed facts. A failed or rolled-back capture cannot publish staged counts.
- Assert original-body UTF-8 boundaries, including role bytes, and exact scored extent for the selected canonical vector participant. Reject a mid-codepoint or malformed tuple. Unknown endpoints and nonparticipants omit the extent, and known zero survives. Keep bounded indexed extent work and snippets-off behavior.
- [Open admission](../../../../../src/rust/crates/fathomdb-engine/src/open.rs)
  and the coverage token both decode the same stored profile columns. A
  name/revision BLOB or negative dimension already causes token capture to
  fail as well as admission to refuse. Existing installed
  [Python](../../../../../src/python/tests/test_typed_errors.py) and
  [Node](../../../../../src/ts/tests/typed-errors.test.ts) cases discriminate
  which failure wins. Reuse these checks on exact checkout-native artifacts.
  The engine controls independently distinguish span tuple refusal, requested
  provider mismatch and token-only refusal. A second malformed field within
  the same profile adds no precedence information. Keep payload and repeat
  refusal assertions, and prove process isolation and healthy persisted reopen
  with the existing native case or a thin invocation of it. No admission
  rewrite or duplicate binding matrix.

## R30-TEST and stop rules

- After Slice 9 selection, Slice 30 implements the approved independent correctness oracles. Extend the existing suites with a short real-store property over known/unknown endpoints and commit/rollback/reopen, and hand-authored rank and ownership expectations for the selected same-state cases. Use fixture facts and original positions to derive expectations independently of production SQL and fusion helpers.
- The oracle must reject a plausible wrong result, such as treating NULL as zero, publishing rollback counts, or merging body twins and renumbering later ranks. Retain minimized seeds. Use the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) for scoped property and mutation choices.
- End after the named cases and at most one discriminating minimized follow-up unless new evidence justifies extending the scope. Classify each as confirmed product defect, fixture/operator defect, expected behavior, bounded non-reproduction or unresolved observation with consequence. There is no bug quota.
- Remediate confirmed defects in this slice. Critical wrong results or data loss require repair or explicit owner disposition before release. Give Slice 70 each selected test gap, its implemented oracle or authorized disposition, and the concrete wrong result the oracle rejects.

## Design and execution

1. **Controlled ranking read.** Use a single owned temporary format-37 store,
   one fixed valid-as-of, a deterministic 8-D test embedder and a handful of
   real writes, under 16 authored rows. Build vector/FTS state through the
   normal write and projection path. Drain the seed. Freeze the existing
   scheduler, write one nearer nonlexical candidate, then compare the same
   reference hybrid before and after releasing and draining that projection.
   This separates the pending vector from synchronous lexical and canonical
   work. Use explicit validity, the existing projection controls and bounded
   waits. Keep the selected visible twin records and text ranks stable, and
   expect the one added vector to shift their original positions.
2. **Ranking and authority witness.** In the in-crate diagnostic only, attach
   an opt-in operation setting to the existing observer token. Use the
   existing forced reference route without changing environment in a
   thread-sharing test. Extend observations at the actual vector SQL output
   (raw exact L2 and physical rowids) and before fusion (typed record keys,
   original position and raw score for each vector and text arm, including
   deferred lexical inputs). Record actual winner hydration versus reference
   execution. Use the existing in-transaction cursor, visibility generation,
   nonce, dependency generation and bound view as the paired witness. The
   single small fixture is below every cap and captures the complete ranked
   input. Equal distances expose observed order, without assuming a new
   cross-state tie policy. No new SQL for candidate tracing, ordinary
   observer payload change, production code path or public ABI.
3. **Independent and same-state oracle.** Fixture vector geometry and text
   frequency/length give distances, ties and lexical order. Check expected
   membership independent of producer output, then recompute both score
   bit patterns using fixed K30/weights1/3 and first original positions.
   Compare full ordered response, score bits, snapshot/cursor, metadata and
   snippet sidecars across reference and actual winner hydration at both
   fixed boundaries. Reuse A3 rank-above-200, malformed losing candidate,
   empty-arm/duplicate, fallback and mixed node/edge controls at their owning
   suite. Use the existing same-body, ownerless, repeat-credit and SDK packing
   tests for provenance; strengthen a concrete uncovered composition only.
4. **Coverage and extent oracle.** Use the existing engine fixture/control
   seams with a bounded generated group of attributed mapped records of
   independently authored body lengths and endpoints (NULL, zero, truncated
   and full), a retained version, an unmapped fact and a well-formed
   unattributed head. Cross-check every public count and exact optional
   numerator/denominator from the fixture, including explicit all-unknown
   and known-zero cases. Compose endpoint changes in a real rollback and
   commit, validate conservative invalidation and refresh, and reopen to
   confirm only committed counts. Keep failed capture publication, pinned
   reader/external commit and original corruption tests. Use the original
   role-tagged string and its byte indices for a UTF-8 case through search,
   covering a selected participating record and the existing nonparticipant,
   unknown and invalid-boundary behavior.
5. **Feedback and test maintenance.** Run the existing mixed unique/ambiguous,
   same-cursor, validation-order and compact refusal cases. Extend the MCP
   suite with a telemetry-backed continuation where one public ref is unique
   on each kept response and maps to that response's distinct query/cursor.
   A focused Astra-reviewed follow-up covers a replacement revision with the
   same public ref. Bind hybrid response IDs to ordered kept ref/cursor pairs
   so the earlier handle retains its original query and cursor. Repeat
   requests with the same membership retain deterministic IDs.
   Review precise labels with schema/rendering, validation and budget in the
   closure. The owner retains the choice to add them. The query compiler's
   in-crate `lib.rs` and `stop_words_tests.rs` already exercise generated
   literal quoting and stop-word behavior. Delete
   `fathomdb-query/tests/property_template.rs` (string equals its clone);
   the existing scaffold checker never registered it.
6. **Review and verification.** Obtain direct Astra-medium review of this
   plan and design, with one FIX-1 and re-review if needed, then commit.
   Each confirmed behavior defect needs a separate genuinely failing
   regression commit with the observed failure, followed by GREEN. Record
   passing characterization and oracle improvements as such. Obtain direct
   Sol6.2-high code review and Terra verification. Run one strict full gate
   on stable source; recover narrowly and verify failed, affected and unrun
   scopes where possible. Preserve ordinary hooks, exact runtime selectors,
   diagnostics and actual exits.
7. **Closure.** Write this slice's status and send the parent exact commit
   range, findings, test coverage and qualification gaps. The parent owns
   canonical state, integration and worktree cleanup. Do not modify any
   frozen campaign result, shared ledger or release-state artifact.
