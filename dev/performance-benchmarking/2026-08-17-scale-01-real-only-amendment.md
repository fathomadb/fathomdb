# SCALE-01 real-only envelope amendment

**Date:** 2026-08-17  
**Authority:** direct HITL ruling `seq-253`.

## Ruling applied

The historical 18,472-row candidate contains 1,200 synthetic records. The
authorized real-only route removes those records and freezes a 17,272-document
primary arm. The existing 7,667-document bridge remains the canonical prefix
of that real-only primary selection.

This amendment supersedes the 18,472 primary-count statements in the TC-5
execution contracts. It does not relax the zero-synthetic rule, substitute a
new corpus, reuse historical EU7 output, or authorize a SCALE-02/product/latency
claim.

## External source and question-set evidence

The external source-inventory receipt has these content-free bindings:

| Item | Value |
| --- | --- |
| Real document count | 17,272 |
| Excluded synthetic document count | 1,200 |
| Bridge document count | 7,667 |
| Source artifact SHA-256 | `8abec2a82e01d6bd466643550b6591d6d348a325a4962f539209c613e49f75c4` |
| Question-set count | 100 |
| Question-set SHA-256 | `f53a07338c4703522895a9370b59c54202af3b07775e37ee47615a143a36fae8` |
| Question source | corpus-pack ground-truth queries |
| Selection seed | `20260817` |

The question text, document identifiers, and external artifact locations remain
outside Git. The question set is a fixed, deterministic external input; it is
not answer-gold and does not affect the exact-f32 ground-truth contract.

## Remaining execution gate

The source and question inventory are now present. A released smoke still
requires the existing CPU/model asset, exact vector-stage runtime,
ground-truth, output-root, and independent-review bindings. Until then the
only valid conclusion is that the inputs are prepared, not that TC-5 fidelity
has been measured.
