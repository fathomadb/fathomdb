# Prompt — long-record embedding: size the problem, then choose a fix

You are working in the FathomDB repository. Read `AGENTS.md` and the memory
index first and follow them: TDD, no mocked database, one writer per
checkout (use your own worktree off `origin/main`), the GPU mandate (RTX 3090
`cuda:0` only), never push a `v*` tag. This is **research and experiment
design first**. Write no product code until the owner (HITL) has reviewed
your findings and the design.

## The problem

The default embedder (bge-small) truncates every input to 512 wordpieces,
about 1.5–2 KB of English (`src/rust/crates/fathomdb-embedder/src/candle_bge.rs:60-66`,
`:558-568`). The engine embeds the whole body as one vector with no chunking.
For any record longer than that:

- Text after the cut is invisible to vector search. A long record whose
  relevant passage comes late can only be found by keyword (FTS5) search.
- Hybrid search fuses a vector score that describes only the record's start.
- Vector-only hits have no match position, so match-centred snippets (EP-17)
  fall back to the start of the record.

## Prior work you must read and build on

- `dev/design/agent-access/match-centred-snippets.md` § 3.4 and § 10 Q5, and
  `dev/design/mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md`.
  These define the snippet anchor. A chunk-level vector hit should give EP-17
  a real anchor (the matched chunk's span); design for that.
- `dev/design/agent-access/hints-sidecar.md`: if a hit was found only by its
  truncated vector, or a record is partly unembedded, decide whether agents
  need a hint for it.
- `dev/design/agent-access/queue-admission.md` and
  `dev/experiments/queue-sim/REPORT.md`: more vectors per record changes
  search cost. Hybrid p95 is already 0.5–1 s under contention (writer-mutex
  and projection contention). Report the latency impact against those numbers.
- `dev/design/chunking-strategy-and-test-guidance.md` and the IR-C record
  (`dev/plans/runs/IR-C-retrieval-findings.md`,
  `IR-C-ws1-fusion-experiment-full.json`, `IR-C-roadmap.md`). **Naive chunking
  was already tried and was flat to negative:** 128/96 windows with max-pool
  roughly doubled dense-only R@10 (0.073 → 0.163), but fused retrieval stayed
  at or below text-only. Do not repeat that experiment. Explain what is
  different now, or test the untested alternatives: late chunking,
  long-context embedders, chunking only past the 512-token limit, and
  record-level plus chunk-level vectors.
- `dev/ROADMAP.md`, "Requested": chunked embedding (requested 2026-10-18).
- `dev/design/1-2M-chunk-scaling-DECISION-BRIEF.md` and
  `dev/adr/ADR-0.6.0-vector-identity-embedder-owned.md`. Vector identity
  belongs to the embedder, and the vector count multiplies under chunking.

## What to do

1. **Size it first.** Measure the record-length distribution in wordpieces
   for the real consumer corpora: Memex-shaped data, LongMemEval, LoCoMo,
   AP-News, and the IR-C corpus. Report the share of records over 512 and
   the share of bytes past the cut. Also report the share of gold evidence
   past the cut, where gold spans exist. If almost no gold lies past the cut,
   say so plainly. The fix may then be a guard (a warning or a hint), not
   chunking.
2. **Show the failure.** Build a small preregistered probe. Use questions
   whose gold passage sits past token 512 in long records. Compare vector-only
   and hybrid recall for "evidence before the cut" against "evidence after
   the cut". Write the decision rule before the run.
3. **Survey the options**, citing sources:
   - chunk-only vectors with max- or top-k pooling;
   - a record vector plus chunk vectors;
   - chunking only the overflow;
   - late chunking;
   - a longer-context embedder;
   - ingest-side chunking by the consumer.

   For each option, cover the cost in vectors, embed time on the 3090, index
   size, query latency (p50/p95/p99), schema and projection impact (additive
   ALTERs only, no data migrations), and the snippet anchor it gives EP-17.
4. **Recommend one option**, with an experiment plan and a decision rule.
   Promotion requires a held-out gain on fused (hybrid) retrieval and on
   answers, not only on dense-only recall, with no regression beyond the
   latency budget.
5. **Write durably.** Put the findings in
   `dev/design/long-record-embedding.md` and catalogue them in
   `dev/design/document-lifecycle.json` (sorted by path). Put an experiment
   receipt under `experiments/` or `dev/experiments/`. Update the "Requested"
   entry in `dev/ROADMAP.md` to point at the findings. Pass
   `./scripts/agent-lint-md.sh`.
6. **Gate:** get a Fable-medium critical review, then the owner's review,
   before any code.

## Report back

The size of the problem in numbers, the probe result against its decision
rule, the recommended option and its cost, and open questions for the owner.
