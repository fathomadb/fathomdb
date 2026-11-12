---
title: Slice 50 requirements and acceptance
status: PROPOSED
target_release: 0.8.29
slice: 50
---

# Slice 50 requirements and acceptance

Release-local refinement of N29-S/K → R29-REL-S/K → R29-3P-09/O3-09 /
D29-SEARCH/P12. These IDs replace no canonical requirement or locked AC.
Canonical NEED-010/012/016/022/023; REQ-010/011/013/014/018/029/052/053;
AC-056/072/073/074/075/076/079/081a/b/c and maintained SDK contracts constrain
applicable retrieval/parity/latency behavior. AC-075 remains vector fidelity,
not relevance against a fused ranking. AC-083's authorized100k text/hybrid
80ms p50 /300ms p99 target remains tracked, not a new pass gate;30 owns its
measurement. No canonical AC entry is authored here.

| Need → requirement | Falsifiable local acceptance | Primary oracle |
| --- | --- | --- |
| S50-N1 distinguish records; S50-R1 | S50-A1: accepted identity selects count/order and exact revision/owner/source for equal-body distinct logical IDs, anonymous/governed twins, node/edge, same entity revisions and ownerless FTS. Same record gets one contribution per arm at its first original positional rank; later repeated occurrences do not renumber following ranks. IdSpace stays write-time stable; no surrogate minting. Include byte-identical stored role-tagged twins and equal decoded plain text with different stored role tags; never fuse by decoded/normalized plain text. | Hand-authored record tuples and manual RRF arithmetic; real SQLite counterpart. |
| S50-N1 truthful provenance; S50-R2 | S50-A2: explanation rank/score, graph exclusion, evidence origin, snippets and weighting belong to the selected record. Graph traversal remains logical-entity traversal and uses the existing first winning path; no body-twin source/span/coverage borrowing. Otherwise-recordable named visible refs require one ref occurrence in the returned page and one cursor occurrence over the complete captured result, including hidden cursor aliases. Repeated visible ref or captured cursor refuses atomically; cursor equality never proves same record. Hidden-only ref strings remain nonmembers and existing membership/not_recorded precedence stays. | Independent expected per-record arm/source/graph path; independent ref/cursor-alias and cross-page cases after ruling. |
| S50-N2 exact affordable ranking; S50-R3 | S50-A3: reference and optimized route return identical ordered hit fields, score bits, projection cursor, fallback/degradation/error metadata and snippet sidecars at a fixed state/view/run window. Include text rank>200 agreement, ties, empty arms and duplicate physical candidates. Preserve node-then-edge arm rank and fixed direct-text candidate policy. | Complete rank reference plus manual adversarial case; actual route witness. |
| S50-N2 safe eligibility; S50-R4 | S50-A4: filters, flags, temporal/dependency visibility precede candidate caps; historical/frozen authority and snapshot-local attribute validation survive. Unsupported optimized controls take reference without changing results or hiding corruption. Direct text stays provider-free, with no inference/vector/graph/CE activity. | Explicit eligible ID sets, real transactions/commit/rollback, source-dependency fallback and provider trap. |
| S50-N3 exact bounded prefix; S50-R5 | S50-A5: every field of specialized snippets equals the retained full selector for generated Unicode/options/spans. Eligible fast cases have bounded examined bytes and allocation independent of suffix length; unsupported context falls back. Ellipses count in max_bytes; windows/spans stay body-relative, highlights stay snippet-relative UTF-8 bytes. | Full-selector differential plus independently stated examples/invariants and allocation/work controls. |
| S50-N2 attributable cost; S50-R6 | S50-A6: exact lightweight rows/hydrated bodies/bytes and prefix work are measured separately from untraced latency. Repeat actual30 cells on the same qualified inputs/native candidate; report p50/p99, RSS, counts and limits without claiming old CS64/native eligibility gains. No unmeasured speedup or new threshold. | Bounded work witness and30 protocol; no execution in Phase A. |
| S50-N1/S50-N3 coherent consumer surface; S50-R7 | S50-A7: actual same-build Rust/Python/N-API and both MCP formats retain selected order/IDs/source/explanation and snippet coordinates; compact presentation uses accepted rounding and whole-hit budgeting. Stored hints/spans stay intact. O3-09 tagged fixtures assert exact stored bytes and body-relative snippet coordinates. Unchanged context_item_from_hit derives public IDs; pack_context retains duplicate-ID refusal, including newly exposed h: twins and existing historical l: duplicates. The identity ruling must explicitly acknowledge that composed success→typed-refusal consequence or separately select a changed contract. Caller-owned unique ContextItem labels remain distinct from record identity. Interface changes accompany the accepted ADR in the product change. | Fresh installed artifacts, ample-budget typed comparison and separate constrained-page tests. |

A1/A2 are conditional on HITL selecting the semantics, not a preapproved key.
[Design](design.md) distinguishes existing and new behavior;
[test map](test-map.md) separates inspected tests from future REDs. Preserve the
approved60 format/filter/flag contract while keeping repeated-reference policy
explicit; no new tool, result field, schema or degradation vocabulary follows
from allocation. 70 has implemented the bounded degradation reasons/hints and native parity;
80/90 still own coverage reporting. Whole60/F11 stays held.

Quality-roadmap fit: use real database boundaries, optimized/reference comparisons,
independent expected selections, Unicode properties, targeted wrong-key/wrong-rank
counterexamples and bounded allocation/work tests. Broad SQL/VFS fuzzing,
whole-workspace MC/DC, chunking and a new testing framework are deferred to their
existing programs. This applies the roadmap without inventing release gates.

Current evidence refinement: accepted30 P measures direct text on zero-vector
stores; its distributions justify no hybrid/body-hydration benefit claim.
Integrated60/70 require same-build regression expectations for exact rounded
formats/budget behavior and absence/started-timeout/overload reasons. A1/A2 must
also pair equal bodies with asymmetric vector participation: only record A has
its validated head span, record B is pending/absent or has unknown byte_end.
Assert A alone receives vector credit; neither body twin inherits another's
coverage, projection origin, source or CE score. Existing B2-8 reporting semantics
and no-new-vector boundary remain; public coverage composition belongs to80/90.

Feedback acceptance refinement from final-review P2: scope is the conservative
MCP guard only, not telemetry-schema repair. A visible unique ref sharing a
cursor with any other captured result is unsafe even if that other ref is
unmentioned or hidden. Page-unique ref plus captured-run-unique cursor is required for attribution;
repeated same-record occurrences are conservatively refused without a proven
record-key handoff. Existing cursor-only events, `[erased]` stable IDs and direct
SDK feedback remain disclosed limitations, not occurrence-accurate claims.

Non-refusal control: a ref appearing once on the returned page may recur off
page at a different cursor and remain recordable. An off-page same-cursor hit
is unsafe even with a different ref. Ref counts are page-local; cursor counts
cover the complete captured result. Do not apply an all-store or blanket
whole-run repeated-ref refusal.
