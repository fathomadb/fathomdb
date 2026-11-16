# Agent re-measure: Claude agents through `fathomdb-mcp` (agent-access items 4 and 5)

- **Item 5:** Claude Sonnet 5.5 and Haiku 5.5 answer LoCoMo conversation 1
  (categories 1–4) through the Block 1 server. All ten conversations sit in
  one store. Each question gets a fresh agent.
- **Item 4:** the 30-question conversation-0 sample is replayed in two arms.
  Arm `json` shows search results as the server returns them. Arm `compact`
  shows one line per hit. The store holds conversation 0 only.

Raw transcripts, answers and verdicts stay in an external run directory, never
in this repository. The setup below describes the October 19 experiment. The
[current candidate preparation](#current-candidate-preparation) has an implemented
runner and ready fixtures. The approved candidate sequence and judging are
complete; see the [current results](#current-candidate-results-2026-11-16).

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

Build the production server separately with
`cargo build --locked --release -p fathomdb-mcp --features embed-cuda --bin fathomdb-mcp`
and preserve that executable before building `--example locomo_load`. The
example's self dev-dependency enables `test-seams`; requesting the server and
example together also enables those seams in the server. Use the separately
built production server for the agent episodes.

```bash
locomo_load <locomo10.json> --single "$S/stores/all/locomo_all.sqlite"
locomo_load <locomo10.json> --single "$S/stores/conv0/conv0.sqlite" --convs 0
```


## Current candidate preparation

Under owner seq-392, the same approved 72 episodes run once on the final
integrated Slice90 candidate before local close. Slice90 source work may proceed
while measurements continue. Calls wait for applicable integrated correctness,
performance, both-format conformance and artifact qualification, plus the
approved models and budget. Model/scope/budget approval remains seq-388; no
separate pre-90 run, model canary or rehearsal is required.

**Owner-approved selection (seq-388):** exact historical answerer
`claude-haiku-5-5`; judge `gpt-4o-mini` with the existing lenient and strict
rubrics. Smoke: conv1 `q0,q1,q2,q3,q4,q5`, two questions each from categories
1/2/4, all ten conversations in one store, separately under native `text` and
`structured`. Item4: existing `CONV0_SAMPLE` fixed 30, conv0-only store, both
native formats. This is 12 smoke + 60 item4 = 72 fresh agent episodes; it does not
claim the historical full 81 item5 or two-model 282 campaign was repeated.
Use the exact approved model IDs and existing reported-cost stop. The first
selected episode establishes model availability; if unavailable, stop without
automatic fallback, retuning, retry or a separate canary.

The dataset exists at
`/home/coreyt/projects/fathomdb/data/corpus-data/raw/locomo10.json`; the existing
loader's identity check and evidence filtering accept 150 conv0 / 81 conv1
questions, including all fixed 30. The judge harness exists at the `HARNESS`
path in `score.py`. The approved all-conversation and conv0-only fixtures are
prepared and closed, ready with complete vector coverage and no pending work. Reuse them
with the final candidate MCP while format37, model and data remain unchanged;
rebuilding the server does not require reingestion.

**Implemented and independently accepted by Astra-medium; integrated at
`73eb4cf7d45a3ceed3bdce20256f84d7d1c76d91`.**
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
- Approved reported-cost allocation: each 6-question smoke phase $0.12, each
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

After final integrated Slice90 acceptance and the applicable correctness,
performance, conformance and artifact qualification, execute the approved
seq-392 sequence once from this checkout. The owner model/spend approval is
already recorded; no additional canary or rehearsal approval precedes it. `$P`
is its owned Python, `$B` the integrated candidate MCP binary, `$D` the dataset above, `$H` the existing judge harness,
`$S` this task's external run directory and `$MODEL`/`$JUDGE` the exact approved
model IDs. Reuse the prepared stores above. Run these four phases in order,
stopping on any failure:

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
grades stay unknown and it cannot pass accuracy. The approved sequence completed
once; its actual results are recorded below.

**Approved scope (seq-388):** the above Haiku 5.5 answerer/GPT-4o-mini judges,
identical single-sentence prompt correction, 6-question conv1 both-format smoke
and fixed 30 paired item4, with one worker / 8 turns / 180 seconds / $0.02 per episode and
$2 reported-cost stop including judges, accepting possible final-call billing
overshoot. The October 19 answerer costs extrapolate to roughly $0.10 for this
selection; this is historical sizing, not a current price quote. Source review,
fixture preparation and the applicable integrated qualification remain required
before calls. Seq-392 places the single 72-episode sequence on the final integrated
Slice90 candidate before local close, without blocking source work on unfinished
measurements.


Pure runner/scorer tests are registered as the fast `test-agent-remeasure`
suite in `scripts/agent-test.sh` (explicit Linux-controller scope). The same
zero-inference command is:

```bash
python3 -m unittest discover -s dev/experiments/agent-remeasure/tests -p 'test_*.py'
```


## Current candidate results (2026-11-16)

The approved 6+6+30+30 sequence completed once: 72 valid episodes, 144 completed
judge responses, no pending/unknown grades, tool errors, busy results, off-policy
calls or unparsed answers. Existing completeness and reported-budget checks
passed. No extra episode, availability canary or fallback was run.

Requested answerer `claude-haiku-5-5` matches every transcript's initialization,
assistant response and `modelUsage` identifier. Requested judge `gpt-4o-mini`
was served as `openai/gpt-4o-mini` for all 144 recorded responses. The owned
runtime used OpenAI 3.51.0 with SDK `max_retries=0` and wrapper
`retry_unknown=False`.

The pinned owned Claude CLI was 2.1.325, SHA-256
`0dc4f4a9a165fc60746d7ae8489712e5fc352f1e7ad4eb22e7141bd0812eb0cf`,
at `<raw root>/claude-2.1.325/claude`. Production MCP source
`222fa39542662f73be943c02d0c8ab14b59e84bb`, SHA-256
`b3c6f256fe88f37b8287dbef29b0610ef4cd7ad029e78661eebfb344914f3eed`,
was preserved at `refresh-open-222fa3954/fathomdb-mcp-cuda` under the raw root.
The unchanged format-37 fixtures were `stores/all/locomo_all.sqlite` (all ten
conversations, 5882 turns) and `stores/conv0/conv0.sqlite` (419 turns), prepared
with the preserved `locomo_load-cuda` SHA-256
`a632e4e102f0514d2c60eb8072d8bb94eb7933e3d98c7d1985cbf051ae9925de`.
Each phase used the existing runner's private copy; originals stayed untouched.
The dataset remains the `locomo10.json` path documented above.

| Phase | Questions | Lenient correct | Strict correct | Answerer reported USD | Mean search bytes/call |
| --- | ---: | ---: | ---: | ---: | ---: |
| Smoke text | 6 | 6/6 | 4/6 | 0.01031050 | 2823.92 |
| Smoke structured | 6 | 6/6 | 4/6 | 0.01028600 | 3840.22 |
| Item4 text | 30 | 29/30 | 17/30 | 0.04562938 | 3224.49 |
| Item4 structured | 30 | 28/30 | 18/30 | 0.05035855 | 4075.08 |

Paired differences below are structured minus text, in percentage points, with
95% paired bootstrap intervals from the existing scorer and complete qID sets.

| Pair | Lenient difference [95% interval] | Strict difference [95% interval] |
| --- | ---: | ---: |
| Smoke, n=6 | 0.00 [0.00, 0.00] | 0.00 [0.00, 0.00] |
| Item4, n=30 | -3.33 [-10.00, 0.00] | +3.33 [-6.67, +16.67] |

Answerers reported $0.11658443; judges charged $0.01786230 under the existing
higher-of-table/route ledger rule: $0.13444673 total against the $2 allocation.
All four answerer phase caps and the $0.56 judge cap were respected. API-reported
answerer usage across72 episodes (178 recorded API messages;179 reported turns) was 356 uncached input, 366299 cache-creation,
1563203 cache-read and 55314 output tokens. Judges reported 95266 input tokens
(including 2048 cached), 6210 output and zero reasoning tokens. Search payload
bytes divided by four remain estimates, separate from these API counters.

Separately, the retained current-native CUDA14 single-hit `Caroline` comparison
matched ref, snippet, arm and source_id: rendered text blocks were 339 UTF-8
bytes in text mode and 517 in structured mode. This renderer sample is separate
from the natural agent search populations in the table. Instruction-presence
self-reports were yes for 15/36 text and 14/36 structured episodes; the remaining
self-reports were no.

Raw root:
`/mnt/data/tmp2/fathomdb-0829-continuation/final-mcp-20261116`.
The four phase directories retain metadata, checkpoints, transcripts and grades;
`score/summary.json`, `score/questions.jsonl` and `score/judge-ledger/judge.jsonl`
retain the comparison and spend. The existing answerer entry log retains strict
zero-owned-actor cleanup and the same-inode physical lease acquire/release
postcheck with the selected GPU empty. The GPU was released before judging.
