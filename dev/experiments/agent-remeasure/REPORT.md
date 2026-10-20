# Agent re-measure: items 4 and 5 (2026-10-19)

Agent-access work items 4 and 5 of
[`dev/design/agent-access/README.md`](../../design/agent-access/README.md).

## Setup

- **Server.** `fathomdb-mcp` CUDA build from `5bc39f864` (Block 1 plus the
  queue rulings: no start check, `run_limit_ms` 30 s). Embedding on the
  RTX 3090 (`cuda:0`). Hybrid search, default limits.
- **Agents.** One headless Claude Code agent per question (`claude -p`),
  `claude-sonnet-5-5` or `claude-haiku-5-5`. The only tools offered are the
  seven `fathomdb` MCP tools. Four agents run at a time, each with its own
  server and store copy.
- **Prompt.** The conv-0 tuned prompt of the earlier subagent run, word for
  word, one question per agent. It still says snippets show the record
  start, which is no longer true (match-centred since Block 1). The
  server's own instructions describe centred snippets, so the agent sees
  both.
- **Judges.** Lenient: the harness grader (gpt-4o-mini, mem0 LoCoMo
  prompts), as in all earlier runs. Strict: the same model with a rubric
  that fails partial, hedged or extra-wrong answers. Intervals are 95 %
  bootstrap; paired differences are per question.
- **Validity.** 282 agent runs; 0 invalid (no tool outside FathomDB), 0
  unparsed answers, 0 tool errors, 0 busy results, 0 non-zero exits.
- Code and method: [README.md](README.md). Run data stays outside the
  repository.

## Item 5: conversation 1, all ten conversations in one store

81 questions (categories 1, 2 and 4; conv 1 has no category 3). The store
holds all 5,882 turns of the ten conversations.

| model | lenient | strict | cost | searches / q | fetches / q | cut refs fetched |
| --- | --- | --- | --- | --- | --- | --- |
| Sonnet 5.5 | 0.951 [0.901, 0.988] | 0.840 [0.753, 0.914] | $1.87 | 1.42 | 0.20 | 11 / 110 (10 %) |
| Haiku 5.5 | 0.926 [0.864, 0.975] | 0.741 [0.642, 0.840] | $0.12 | 1.40 | 0.28 | 20 / 104 (19 %) |

- **Haiku − Sonnet:** lenient −0.025 [−0.086, +0.037]; strict −0.099
  [−0.173, −0.025] (Sonnet wins 9, Haiku 1).
- **Strict by category** (correct / total). Sonnet: single-hop 6/11,
  multi-hop 25/26, open-domain 37/44. Haiku: 4/11, 24/26, 32/44.
  Single-hop (category 1, list-style questions) is where both lose most.
- **Other conversations.** Searches return hits from other
  conversations, as expected in one shared store. Two answers per model
  cite a record from another conversation; all four are strict misses.
- **Against the tuned run.** On the conv-0 sample the same prompt scored
  1.000 (Sonnet) and 0.967 (Haiku) lenient. On the untuned conversation in
  a ten-times larger store it scores 0.951 and 0.926. The intervals
  overlap; the drop is small and within noise at this size.

## Item 4: JSON against a compact text block

The 30-question conv-0 sample, conv-0 store, each model with both arms.

- **What the model sees.** Claude Code shows the model the server's
  `structuredContent` JSON, not its text block, so arm `json` is that
  JSON. Arm `compact` goes through `compact_proxy.py`, which drops
  `structuredContent` and `outputSchema` for `search` and `expand` and
  renders one line per hit, plus one line per hint. Every ref, cut flag,
  continuation and hint survives; epoch, `validity_instant`, `state` and
  `verdict` are dropped and scores are rounded.

| model, arm | lenient | strict | search tokens / call | result tokens / q | cut refs fetched | cost |
| --- | --- | --- | --- | --- | --- | --- |
| Haiku, json | 0.967 | 0.533 [0.367, 0.700] | 1,022 | 1,482 | 10 / 58 (17 %) | $0.046 |
| Haiku, compact | 1.000 | 0.567 [0.400, 0.733] | 683 | 928 | 12 / 48 (25 %) | $0.033 |
| Sonnet, json | 1.000 | 0.700 [0.533, 0.867] | 1,042 | 1,370 | 4 / 61 (7 %) | $0.679 |
| Sonnet, compact | 1.000 | 0.767 [0.600, 0.900] | 705 | 986 | 3 / 68 (4 %) | $0.474 |

Tokens are bytes ÷ 4 of what the model received; "search tokens / call" is
the mean of per-question means.

- **Tokens.** Compact cuts a search result by 33 % (Haiku 1,022 → 683,
  Sonnet 1,042 → 705) and the tool output per question by 28–37 %.
- **Cost.** Compact is 28 % (Haiku) and 30 % (Sonnet) cheaper, mostly from
  less cache creation (4.4–4.8k → 2.7k tokens per question).
- **Accuracy: no measurable difference.** Compact − JSON, strict, paired:
  Haiku +0.033 [−0.067, +0.133]; Sonnet +0.067 [−0.067, +0.200]. The
  lenient judge is at the ceiling in both arms.
- **Fetch on cut snippets: no consistent change.** Haiku 17 % → 25 %,
  Sonnet 7 % → 4 %. Both are above the ~6 % of the pre-Block-1 run, but
  that run differs in more than rendering (centred snippets, hints, one
  agent per question).

## Caveats

- 30 and 81 questions. Intervals are wide; a 5-point accuracy effect is
  not detectable at this size.
- The strict judge is an LLM with a stricter rubric, not hand grading. It
  agrees with the direction of the earlier hand grade (Sonnet above Haiku)
  but has not been calibrated against it.
- The prompt describes pre-Block-1 snippets; the server instructions do not.
- Item 4 runs only the conv-0 store, where the prompt was tuned.

## For the owner

- **Item 4 gate.** The compact block saves a third of the search tokens
  and about 30 % of the cost per question, with no measurable accuracy
  change and no change in fetch behaviour. Decide whether to design it
  (a `text_block` rendering the adapter controls, which also needs the
  `structuredContent` question settled: Claude Code shows the JSON, not
  the text block).
- **Item 5.** No sign that the conv-0 prompt tuning carried the earlier
  result: on conv 1 in a ten-conversation store both models stay above
  0.92 lenient. Single-hop list questions are the weak category.
- **Next runs.** Update the prompt's snippet sentence to match the
  server before any further measurement.
