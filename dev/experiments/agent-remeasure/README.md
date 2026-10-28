# Agent re-measure: Claude agents through `fathomdb-mcp` (agent-access items 4 and 5)

- **Item 5:** Claude Sonnet 5.5 and Haiku 5.5 answer LoCoMo conversation 1
  (categories 1–4) through the Block 1 server. All ten conversations sit in
  one store. Each question gets a fresh agent.
- **Item 4:** the 30-question conversation-0 sample is replayed in two arms.
  Arm `json` shows search results as the server returns them. Arm `compact`
  shows one line per hit. The store holds conversation 0 only.

Raw transcripts, answers and verdicts stay in an external run directory, never
in this repository.

## Pieces

- `run.py` runs one headless `claude -p` agent per question. The agent gets
  only the `mcp__fathomdb__*` tools (`--tools ""`, `--strict-mcp-config`) and
  starts in an empty working directory, so no project instructions load. Each
  worker has its own copy of the store, because stores hold an exclusive lock,
  and its own MCP config. Per question the run keeps the stream-json
  transcript, the server stderr and one checkpoint row. `--resume` skips
  questions already done.
- `compact_proxy.py` is a stdio proxy in front of `fathomdb-mcp` for arm
  `compact`.
- `score.py` audits the transcripts and judges the answers twice with
  gpt-4o-mini: the harness's evidence-aware LOCOMO judge (lenient) and a
  strict rubric. It reports accuracy with 95% bootstrap intervals and paired
  differences between runs of the same item. When the final reply has no
  parsable JSON answer line, the whole reply minus the `INSTRUCTIONS_SEEN`
  line is judged, and `answer_unparsed` counts these. Verdicts are cached
  by question and answer hash. `mean_search_tokens_est_per_call` is the mean
  of per-question means (search bytes ÷ 4 per call), not a pooled per-call
  mean.
- `tests/` tests measurement correctness only: rendering keeps the
  information, and the audit counts correctly.

## Known deviations

- **Prompt and server disagree.** The prompt is reused verbatim from the first
  Claude run so the runs stay comparable. It still says "Snippets show only the
  start of each record", but since Block 1 snippets are centred on the match.
  The server's own instructions describe centred snippets.
- **What the agent sees.** Claude Code passes the model `structuredContent`,
  serialized as JSON, whenever a result has it. It does not pass the `text`
  block. Arm `json` is therefore the sorted, compact JSON of
  `structuredContent`.
- **The compact proxy drops `structuredContent`** for `search` and `expand`,
  and drops their `outputSchema` from `tools/list`. Keeping `structuredContent`
  would have left the compact arm identical to the JSON arm.
- **Fields the compact form drops:** `epoch`, `validity_instant`, `state`,
  `verdict` and the `budget.cut` key name. Scores are rounded to three
  decimals. Each hit line reads
  `<ref> <arm> <score> [@anchor] [cut] src=<source> | <snippet>`. The anchor is
  shown only when it is not `whole`.

## Commands

`$S` is the scratch directory and `$B` is the CUDA `fathomdb-mcp` binary. The
build recipe is in `../mcp-hybrid-vs-keyword/README.md`. Use the RTX 3090 only.

```bash
locomo_load <locomo10.json> --single $S/stores/all/locomo_all.sqlite
locomo_load <locomo10.json> --single $S/stores/conv0/conv0.sqlite --convs 0
python run.py --run-dir $S/runs/item5-sonnet --store $S/stores/all/locomo_all.sqlite \
  --conv 1 --model sonnet --binary $B
python run.py --run-dir $S/runs/item4-json-haiku --store $S/stores/conv0/conv0.sqlite \
  --conv 0 --sample conv0-30 --arm json --item 4 --model haiku --binary $B
python score.py --out $S/score $S/runs/*
```
