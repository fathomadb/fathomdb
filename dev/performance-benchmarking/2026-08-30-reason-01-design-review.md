# REASON-01 design review

**Reviewer:** independent read-only subagent  
**Date:** 2026-08-30  
**Initial verdict:** revise  
**Disposition:** accepted after the corrections below

## Evidence judgment

`protected_multiquery_v1` is the supported single candidate, but is not yet a
proven treatment. On the preserved 282-question development set, deep compact
gained two correct answers over protected multi-query while losing thirteen
grounded answers. Its retrieval difference was negligible, and the prior
24-case LongMemEval comparison tied. Rejecting deep compact is therefore the
more conservative choice.

## Findings and dispositions

1. **Supporting evidence was not an eligibility gate.** Added fractional
   gold-session recall as the primary retrieval metric with a paired 10,000-draw
   percentile-bootstrap lower bound. Added any-gold and all-gold reporting,
   explicit failure/tie handling, and cold/steady repetitions.
2. **The held-out cohort permitted post-selection.** Replaced a non-empty
   selection with all 109 untouched LongMemEval-S multi-session cases and froze
   ordered IDs plus source, exclusion, selection-code, and config hashes.
3. **Runtime pinning was incomplete.** Required commit, package, native module,
   CLI, model/cache, CUDA/driver/host, corpus, adapter, and identity-map hashes.
   The existing shared Python environment currently fails its FathomDB import;
   the implementation must repair and attest it before live equivalence.
4. **Resolution and filter semantics were ambiguous.** Added the complete known
   intent/override truth table and required read view, projection cursor, and
   metadata filter propagation by identity.
5. **A missing-module RED was too weak.** Required an importable skeleton and a
   targeted behavioral RED result for every contract family.
6. **Identity and trace details were implicit.** Defined identity as
   `SearchHit.id.value`, canonical attribution as `(source_id, body_sha256)`,
   domain-separated trace hashes, and generic context items.

With these changes the design is approved for TDD implementation. It remains
caller-side and introduces no Engine, SDK, schema, storage, or default-routing
change.
