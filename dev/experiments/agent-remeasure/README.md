# Agent re-measure: Claude agents through `fathomdb-mcp` (agent-access items 4 and 5)

- **Item 5:** Claude Sonnet 5.5 and Haiku 5.5 answer LoCoMo conversation 1
  (categories 1–4) through the Block 1 server. All ten conversations sit in
  one store. Each question gets a fresh agent.
- **Item 4:** the 30-question conversation-0 sample is replayed in two arms.
  Arm `json` shows search results as the server returns them. Arm `compact`
  shows one line per hit. The store holds conversation 0 only.

Raw transcripts, answers and verdicts stay in an external run directory, never
in this repository. The setup below describes the October 19 experiment; the
[current candidate preparation](#current-candidate-preparation) is proposed and
has not run.

## Pieces

- `run.py` runs one headless `claude -p` agent per question. The agent gets
  only the `mcp__fathomdb__*` tools (`--tools ""`, `--strict-mcp-config`) and
  starts in an empty working directory, so no project instructions load. Each
  worker has its own copy of the store, because stores hold an exclusive lock,
  and its own MCP config. Per question the run keeps the stream-json
  transcript, the server stderr and durable reservation/completion rows. The
  current runner is serial; `--resume` skips completed questions and refuses
  unknown/invalid attempts.
- `compact_proxy.py` is retained for the historical `compact` experiment only;
  current candidate measurements use the built server directly.
- `score.py` audits the transcripts and judges the answers twice with
  gpt-4o-mini: the harness's evidence-aware LOCOMO judge (lenient) and a
  strict rubric. It reports accuracy with 95% bootstrap intervals and paired
  differences between complete runs of the same item. Current runs reject
  unparsed answers before grading. Verdicts are cached by question/answer,
  and a changed requested judge model refuses cache reuse. `mean_search_tokens_est_per_call` is the mean
  of per-question means (search bytes ÷ 4 per call), not a pooled per-call
  mean.
- `tests/` tests measurement correctness only: rendering keeps the
  information, and the audit counts correctly.

## Historical deviations

- **The historical prompt and server disagreed.** It said "Snippets show only
  the start of each record", but Block 1 snippets were already match-centred.
  Current runs apply the single approved design correction below.
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

## Store preparation

After authorization and the hard execution prerequisites, use the current
candidate loader, owned tools and allocated RTX3090. The historical build
recipe is in `../mcp-hybrid-vs-keyword/README.md`; it does not qualify a current
candidate binary. `$S` is this task's external scratch directory.

```bash
locomo_load <locomo10.json> --single "$S/stores/all/locomo_all.sqlite"
locomo_load <locomo10.json> --single "$S/stores/conv0/conv0.sqlite" --convs 0
```


## Current candidate preparation

This is the required pre-90 Block1 smoke/item4 remeasure, not a retrieval-only
substitute or the final 160 qualification. Actual selected20–70 exits and the
applicable integrated checks/performance/both-format conformance remain hard
predecessors. No model/spend approval or execution is implied by this proposal.

**Proposed selection for owner approval:** exact historical answerer
`claude-haiku-5-5`; judge `gpt-4o-mini` with the existing lenient and strict
rubrics. Smoke: conv1 `q0,q1,q2,q3,q4,q5`, two questions each from categories
1/2/4, all ten conversations in one store, separately under native `text` and
`structured`. Item4: existing `CONV0_SAMPLE` fixed 30, conv0-only store, both
native formats. This is 12 smoke + 60 item4 = 72 fresh agent episodes; it does not
claim the historical full 81 item5 or two-model 282 campaign was repeated.
Model IDs describe the historical selection; current pricing and availability
remain unverified until an explicitly approved canary. Unavailable models stop
without automatic fallback or retuning.

The dataset exists at
`/home/coreyt/projects/fathomdb/data/corpus-data/raw/locomo10.json`; the existing
loader's identity check and evidence filtering accept 150 conv0 / 81 conv1
questions, including all fixed 30. The judge harness exists at the `HARNESS`
path in `score.py`. Historical block1 scratch stores were not located in the
known locations checked. Build fresh candidate-format stores later with the
existing `locomo_load` commands above; this is an execution step, not a missing
owner-provided fixture. Use the integrated candidate MCP/loader, owned tools
and selected GPU only after the heavy window is available.

**Implemented after Astra-medium design acceptance; final review pending.**
The existing runners/tests now provide these controls, without a second harness
or proxy-based current arm:

- `run.py`: select explicit native `result_format=text|structured` in server
  config and launch the server directly for both. Replace only
  `Snippets show only the start of each record` with
  `Snippets are centred on the match` in the shared prompt; both arms and
  smoke use that identical correction. Keep every other prompt word.
- Bound scheduling to one worker, 8 model turns, 180 seconds per episode; pass
  Claude's `--max-turns 8 --max-budget-usd 0.02`. Installed Claude 2.1.322
  accepts these options in metadata-only help. Keep tool isolation and empty
  cwd; audit actual 4-tool/3-search limits. Terminate the full child process group
  on timeout, preserve partial transcripts, and stop the phase on invalidity.
- Proposed reported-cost allocation: each 6-question smoke phase $0.12, each
  30-question item4 phase $0.60, judges $0.56; total $2.00. Reserve $0.02 before
  each answerer episode, persist it with existing run data, and check remaining
  phase allowance before scheduling. Unknown/timeout charges retain their full
  reservation and halt; no automatic episode retry or resume re-spend. Persist
  complete answers incrementally and resume only truly missing, unstarted cells.
- Expose the judge cap/fixture paths as arguments instead of hardcoded $20/main
  paths. Keep the existing cached per-answer grades and backoff for known
  pre-response rejection; stop on unknown billable failure or malformed grade
  rather than automatically repeating a possibly charged call. Report actual
  judge spend and halt at its allocation. CLI budgets and the existing
  post-response cost ledger are **not a provider-enforced billed ceiling**:
  the final in-flight call can overshoot. If the owner requires an absolute
  billed ceiling, execution waits for a provider-enforced limit; no such control
  was verified here.
- `score.py`: parse native JSON-quoted refs and bare `cut` in text results;
  keep historical proxy parsing/tests. Report search hits/page, bytes/estimated
  tokens per call (bytes / 4 estimate, not billed tokens), separately reported API
  token usage, cost/question and cut-ref fetch rate.
  Off-policy calls, tool errors/busy, nonzero/timeout/no-result episodes,
  unparsed answers, missing grades or missing expected qIDs invalidate the phase;
  never issue a paired verdict from a silent intersection. Require all 30 paired
  item4 qIDs and report strict/lenient paired 95% intervals. Preserve the existing
  accuracy-stop contract; invent no extra numerical quality threshold.
- Keep same-page rendering byte comparison separate from end-to-end results.
  Reuse candidate-native deterministic renderer/format parity coverage; the old
  proxy byte ratio cannot qualify the current built renderer. Stage semantic
  failing tests for format selection/prompt correction, bounded scheduling and
  resume, native text measurement, and incomplete/invalid-pair refusal before
  implementation. Fake child/API boundaries are appropriate for runner tests;
  real candidate LLM/MCP/store execution is still required for the exit.

Only after final implementation acceptance, explicit owner model/spend approval,
actual hard predecessor exits and the available heavy window, execute these
commands from this checkout. `$P` is its owned Python, `$B` the integrated
candidate MCP binary, `$D` the dataset above, `$H` the existing judge harness,
`$S` this task's external run directory and `$MODEL`/`$JUDGE` the exact approved
model IDs. Provision current stores first with the existing `locomo_load`
commands above. Run these four phases in order, stopping on any failure:

```bash
"$P" dev/experiments/agent-remeasure/run.py --run-dir "$S/smoke-text" \
  --store "$S/stores/all/locomo_all.sqlite" --conv 1 --qids q0,q1,q2,q3,q4,q5 \
  --result-format text --model "$MODEL" --binary "$B" --dataset "$D" --cap-usd 0.12
"$P" dev/experiments/agent-remeasure/run.py --run-dir "$S/smoke-structured" \
  --store "$S/stores/all/locomo_all.sqlite" --conv 1 --qids q0,q1,q2,q3,q4,q5 \
  --result-format structured --model "$MODEL" --binary "$B" --dataset "$D" --cap-usd 0.12
"$P" dev/experiments/agent-remeasure/run.py --run-dir "$S/item4-text" \
  --store "$S/stores/conv0/conv0.sqlite" --conv 0 --sample conv0-30 --item 4 \
  --result-format text --model "$MODEL" --binary "$B" --dataset "$D" --cap-usd 0.60
"$P" dev/experiments/agent-remeasure/run.py --run-dir "$S/item4-structured" \
  --store "$S/stores/conv0/conv0.sqlite" --conv 0 --sample conv0-30 --item 4 \
  --result-format structured --model "$MODEL" --binary "$B" --dataset "$D" --cap-usd 0.60
"$P" dev/experiments/agent-remeasure/score.py --out "$S/score" --cap-usd 0.56 \
  --dataset "$D" --harness "$H" --env-file "$EVAL_ENV" --judge-model "$JUDGE" \
  "$S/smoke-text" "$S/smoke-structured" "$S/item4-text" "$S/item4-structured"
```

Defaults are one worker, 8 turns, 180 seconds and $0.02 episode cap; all are exposed
arguments. `--resume` skips completed episodes only and refuses unknown/invalid
reservations. Scoring refuses incomplete phases before constructing a client.
Unknown judge outcomes remain pending in the existing cache and cannot be
rejudged automatically. `--no-judge` is a zero-egress transcript audit; absent
grades stay unknown and it cannot pass accuracy. No actual candidate run has
been performed by this implementation work.

**Exact owner question:** approve the above Haiku 5.5 answerer/GPT-4o-mini judges,
identical single-sentence prompt correction, 6-question conv1 both-format smoke
and fixed 30 paired item4, with one worker / 8 turns / 180 seconds / $0.02 per episode and
$2 reported-cost stop including judges, accepting possible final-call billing
overshoot? The October 19 answerer costs extrapolate to roughly $0.10 for this
selection; this is historical sizing, not a current price quote. Source review,
fixture preparation and all hard predecessor checks remain required before any
priced run.


Pure runner/scorer tests are registered as the fast `test-agent-remeasure`
suite in `scripts/agent-test.sh` (explicit Linux-controller scope). The same
zero-inference command is:

```bash
python3 -m unittest discover -s dev/experiments/agent-remeasure/tests -p 'test_*.py'
```
