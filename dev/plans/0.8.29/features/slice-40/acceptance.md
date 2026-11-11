# Slice 40 proposed local acceptance

These are local A40 labels. Canonical `dev/acceptance.md` remains locked;
[release mapping](release-integration.md) preserves existing gates and tracked
AC-083. No row currently passes: measurements and selected implementation do not
exist.

| ID | Evidence required after prerequisites | Present disposition |
| --- | --- | --- |
| A40-1 | Current 30 component inventory plus paired end-to-end results for both schedules; separately report eligibility and batch treatments, miss/key cost, invalidations, hit opportunities, memory and uncertainty. Fix benefit/regression tolerances before treatment runs. | BLOCKED on full20/inputs/30; no inferred benefit. |
| A40-2 | Selected optimization and reference agree on hits, order/scores, eligibility before each cap, projection cursor, degradations and typed errors under the test-map authority/time/view matrix. Human-specified eligibility assertions prevent a shared-predicate oracle. | HELD pending exact reviewed treatment. |
| A40-3 | Reference and selected batch path agree on vector bytes/metadata, sidecars, spans, attributes, terminal/cursor state, mean pin/accumulator, events and failure/rollback outcomes. Runtime work counts prove selected repeated metadata reads actually decrease. | HELD pending measured residual and invariant proof. |
| A40-4 | Churn/reopen/old-reader controls demonstrate selected entry/byte/lifetime bounds; misses preserve correct reference/error behavior and include full miss cost. No retained snapshot or uncommitted authority leaks. | HELD pending ownership/eviction design. |
| A40-5 | Independent review dispositions cover all inventoried opportunities and holds; any selected change has per-behavior actual RED/GREEN, focused correctness/performance evidence, required final code-phase gate and fresh Astra final review. Not-selected/deferred names actual evidence and release impact. | Bounded proposal only; design review pending. |

A40-2/3/4 implementation evidence is conditional on selection. A measured
not-selected disposition does not require inventing code/tests to satisfy it;
review instead verifies why no change is warranted. Unavailable evidence remains
blocked/deferred, never a performance pass or conditional-slice completion.
