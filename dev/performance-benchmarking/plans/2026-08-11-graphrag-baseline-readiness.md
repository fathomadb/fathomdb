# GraphRAG baseline readiness plan

## Objective

First reproduce GraphRAG's native 15-document witness once. Then make one
separate, descriptive controlled AP-News global-answer collection; neither
one-pass result is a registered parity decision.

## Frozen inputs and contract

- AP-News archive SHA-256 is
  `2f70dda22a9f261f285c94f3ac13a8f0df60b69fe3df1b5853b47b372065a66f`.
- The preserved 15 inputs byte-match `load_articles()[:15]`; joined-body
  digest is `0bd4e0d9dafb9ef047c81fa84c5d95a77dba64f98ff2aa55adb044f4fad7011e`.
- Pin `graphrag==3.1.0` and every package in
  `data/corpus-data/0.8.4-graphrag-artifacts/graphrag-venv-pip-freeze.txt`.
- Record settings/prompt digests and use gpt-5.4 via Airlock, deterministic
  384d embedding shim, chunks 1200/100, max gleanings 1, cluster size 10,
  and 2,000-token reports for the native witness.

## One-run sequence

1. Create a fresh venv and workspace; never modify the preserved historical
   workspace. Materialize and rehash exactly the first 15 documents.
2. Start the embedding shim, run GraphRAG index once, then one native global
   query. Record commands, settings, output fingerprints, cache state, timing,
   and cost.
3. Run the FathomDB C/map-reduce global-answer arm over the same 15 docs and
   query with the same model and 1,500-token output budget. Plain top-k
   `Engine.search` is prohibited.
4. After the witness succeeds, use a separate fresh 200-document AP-News
   workspace. Run GraphRAG global level 1 with dynamic selection disabled and
   a FathomDB map-reduce arm over the same texts/question IDs. Persist paired
   answers and provenance; do not emit an AutoE confidence interval or parity
   verdict from n=1.

## Stop conditions

Registry installation, a live Airlock service, and a declared cost ceiling are
required. The previous virtual environment and package caches are absent. The
existing gated runner correctly rejects fewer than five judge repetitions, so
a new descriptive one-pass driver must refuse to write a registered decision.
