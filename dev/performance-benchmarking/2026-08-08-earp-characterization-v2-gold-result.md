# EARP characterization over v2 gold

## Decision question and claim class

What retrieval-quality shape does FathomDB's FTS-only path exhibit on the
frozen IR-C reuse-tier gold after the Slice 19 performance fix? This is a
descriptive characterization, not a comparator, parity, or surpass claim.

## System and exact configuration

The primary run used FathomDB 0.8.22 development code at
`93722e22`, `Engine.search_text_only`, `retrieval_mode=fts_only`, no default
embedder, and `fanout_used=10`. The resolved configuration SHA-256 was
`629f95e9bd09e8df5cdbbcd05f31368299261768cf0e6503a30a1ad5dfa404eb`.
The source result was recorded by commit `ff221074`; the first-class negative
aggregate and verification record came from commits `bf057d2e` and
`42c4b2a9` on `feat/earp-eval-platform-20260806`.

## Corpus and gold

- Corpus: frozen `0.8.x-B`, 10,506 documents from ten sources, SHA-256
  `fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`.
- Gold: `ir-c-reused-v2`, 4,597 queries, SHA-256
  `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`.
- Scoreable retrieval queries: 4,472; negative queries: 125.
- License posture: project-authored gold over a mixed-license corpus whose
  sources include MIT, Apache-2.0, BSD-3-Clause, CC-BY-4.0, research-use, and
  undeclared/upstream-chain material. Corpus and gold payloads remain local.

## Protocol

The runner measured strict and graded evidence recall at K=5 and K=10. The
gold is binary, so the strict and graded values coincide. `ndcg` was typed
`not_applicable` because the gold has no graded relevance, and supporting
coverage was typed `not_applicable` because no query carries supporting-unit
gold. A deterministic verification run after adding the negative aggregate
reused the same configuration and produced the same per-query sidecar digest.
This was not a predeclared repeated-run uncertainty protocol.

## Result and uncertainty

| Metric | K=5 | K=10 | Queries |
| --- | ---: | ---: | ---: |
| Overall evidence recall | 0.6552 | 0.7010 | 4,472 |
| Exact-fact evidence recall | 0.8705 | 0.9024 | 2,888 |
| Exploratory evidence recall | 0.2626 | 0.3340 | 1,584 |

FTS-only returned an empty result for 1 of 125 negative queries, an abstention
rate of 0.008. No sampling interval is reported: the campaign evaluated the
available frozen gold once, and the verification replay checked determinism
rather than execution-to-execution variability.

## Artifact availability

The local primary checkout retained both run directories when this note was
prepared:

- `experiments/runs/earp-characterization-v2gold-full-20260808T0833Z-629f95e9/`
- `experiments/runs/earp-characterization-v2gold-full-20260808T1341Z-629f95e9/`

For the verification run, `record.json` has SHA-256
`cc983eaafac6da96c7640e3dc8edfdbf604c3e181180e5415b5668e611bc6161`,
`earp.result.v1.json` has SHA-256
`8ca0aa75c5b60efd98669f6f4772bb869dee75c7cc69fe65551ca005ce6f19bf`,
and `earp.per-query.v1.jsonl` has SHA-256
`e8988d0ffef7ff9b067a278ec57b4884baef021fb585c6cc28fdecc27b5150b2`.
These paths are gitignored, machine-local artifacts and are not copied here.

## Nonclaims

The result does not establish answer accuracy, semantic-answer quality,
competitor parity, a dense/hybrid comparison, a product default, or a
repeated-run performance claim. In particular, the gold contains no free-text
answers and cannot support answer-accuracy conclusions.
