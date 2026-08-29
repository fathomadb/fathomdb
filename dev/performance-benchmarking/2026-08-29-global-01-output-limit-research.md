# GLOBAL-01 DeepSeek output-limit correction

**Status:** v3 zero-spend preflight passed; paid witness next.

Airlock resolves `deepseek-v4-pro` to native
`deepseek/deepseek-v4-pro`. Its live model metadata reports a 1,000,000-token
input limit and 393,216-token output limit. DeepSeek's official documentation
reports the equivalent rounded limits: a 1M context window and 384K maximum
output.

The prior 1,500-token reduction setting was therefore an experiment ceiling,
not a model or provider limit. Three reductions reached exactly 1,500 tokens,
returned `finish_reason=length` behavior, and truncated their JSON. DeepSeek's
JSON guidance explicitly requires a reasonable `max_tokens` value to avoid
mid-object truncation.

V3 sets both matched reduction arms to 4,096 output tokens and leaves map,
answer, retrieval, scoring, and acceptance rules unchanged. The task ceiling
is deliberately far below the model maximum. Relative to 1,500, its 84 planned
reductions add at most 218,064 normal-path output tokens, or $0.864 at the
registered conservative $3.96/M rate. The projected run total is $10.40 under
the unchanged $12 hard cap.

The runner now preserves provider `finish_reason` and identifies `length`
directly. Resumed invalid witnesses also receive a new receipt timestamp rather
than rewriting an earlier receipt.

The [authorized v3 preflight](../../experiments/runs/global-01-lazy-preflight-20260829T2113Z-b0f3c328/record.json)
passed at zero spend under configuration SHA-256
`b0f3c3281392b4012ae4d7bcf44bb577bb1a9354e5ea4cf37c966f8a410fa83d`.

The first paid response exposed a local mixed-type usage check before its A/A
cell could checkpoint. The corrected client validates numeric prompt and
completion counts independently and retains `finish_reason` as metadata. The
stopped root contains no completed cell and must not resume.

The [post-fix preflight](../../experiments/runs/global-01-lazy-preflight-20260829T2117Z-b0f3c328/record.json)
passed at zero spend.

The [v3 witness](../../experiments/runs/global-01-lazy-witness-20260829T2118Z-b0f3c328/record.json)
confirmed `finish_reason=length` at exactly 4,096 tokens on all three reduction
attempts. The model limit is not the constraint. The current output repeats
canonical source UUID/hash pairs and long mapped-claim IDs. The next correction
keeps the 4,096 task ceiling and uses compact local references with deterministic
canonical restoration, avoiding an 8,192-token cost expansion.

V4 implements the compact reduction adapter and fail-closed canonical
restoration without changing the answer, retrieval, scoring, or acceptance
contract.

## Sources

- [DeepSeek models and pricing](https://api-docs.deepseek.com/quick_start/pricing)
- [DeepSeek Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/)
- [DeepSeek JSON output](https://api-docs.deepseek.com/guides/json_mode/)
- [OpenRouter DeepSeek V4 Pro](https://openrouter.ai/deepseek/deepseek-v4-pro)
