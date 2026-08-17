# CORPUS-01 independent LLM-review amendment

**Date:** 2026-08-17  
**Authority:** direct HITL ruling `seq-253`.

## Separate evidence mode

`corpus-01-llm-review-protocol.v1` is a distinct, external-only evidence route.
It does not alter `human-gold-protocol.v2` or label LLM output as human gold.
It requires two independent model identities, prompt/transcript/run hashes,
blinded assignment, content-free record locators, and a total spend at or below
the authorized USD 20 cap.

## Qualified outcome

Two independent LLM reviewers completed a 16-record external LongMemEval
workset. The content-free manifest SHA-256 is
`79ae4aa04ae6d2de1fe22f969bf45d60b0208543b0d517a0ba92cf7380ad598c`.

| Result | Count |
| --- | --- |
| Knowledge-update records | 4 reviewed |
| Time-scoped-validity records | 4 reviewed |
| Supersession records | 4 reviewed |
| Source-erasure records | 4 reviewed |
| Adjudicated portfolio conclusion | `portfolio_qualified` |
| Direct paid API spend | USD 0.00 of USD 20.00 cap |

This qualifies external lifecycle-evidence adequacy only. It is not human
gold, a product claim, or evidence that FathomDB physically deleted every
projection; that latter execution claim remains within TRACE-01. Raw questions,
answers, sessions, reviewer transcripts, and external paths remain outside Git.
