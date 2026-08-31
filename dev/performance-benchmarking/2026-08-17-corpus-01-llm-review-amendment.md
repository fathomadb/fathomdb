# CORPUS-01 independent LLM-review amendment

**Date:** 2026-08-17  
**Authority:** direct HITL ruling `seq-253`.

## Separate evidence mode

`corpus-01-llm-review-protocol.v1` is a distinct, external-only evidence route.
It does not alter `human-gold-protocol.v2` or label LLM output as human gold.
It requires two independent model identities, prompt/transcript/run hashes,
blinded assignment, content-free record locators, and a total spend at or below
the authorized USD 20 cap.

## Pilot outcome

Two independent LLM reviewers completed a 12-record external LongMemEval pilot.
The content-free manifest SHA-256 is
`830118a4d2398dc2e495c749e95816709d0174d10189af22dc39b8e92458ab0d`.

| Result | Count |
| --- | --- |
| Knowledge-update records | 4 supported |
| Time-scoped-validity records | 4 supported |
| Supersession records | 2 supported, 2 insufficient evidence |
| Adjudicated disagreements | 2 |
| Source-erasure records | 0 |
| Direct paid API spend | USD 0.00 of USD 20.00 cap |

The pilot is explicitly `evidence_limited`: source erasure is unrepresented,
and no broad mutable/provenance-preserving agent-memory claim is qualified.
Raw questions, answers, sessions, reviewer transcripts, and external paths
remain outside Git.
