# GLOBAL-01 v2 witness recovery contract

**Status:** paid execution authorized by coreyt on 2026-08-29 with a $12 hard
cap.

## Decision

Determine whether the stopped GLOBAL-01 witness can execute under the original
measurement design after correcting its map-output adapter. This recovery does
not change the retrieval hypothesis or reinterpret the invalid v1 witness as a
quality result.

## Diagnosed failure

The v1 map prompt contained contradictory attribution instructions. It told the
generator to cite canonical `source_id` and `content_sha256` pairs, but its
example and validator accepted only short `source_refs`. Both encodings retain
the same canonical attribution, yet a schema-correct canonical response failed
the compact-only validator.

The invalid-cell record retained usage, latency, and response hashes but not a
content-free validation category or response shape. Consequently, the stopped
receipt can establish semantic-contract exhaustion but cannot distinguish the
two valid attribution encodings after the fact.

## Correction

Configuration `apnews-global-lazy-coverage.v2.json` registers adapter
`compact_refs_or_canonical_v2` and a fresh configuration hash.

- The prompt asks unambiguously for supplied `SOURCE_REF` values.
- The validator accepts either compact references or exact canonical source
  pairs from the supplied batch.
- Both forms are normalized to canonical source ID and content hash before
  persistence.
- Unknown, duplicate, empty, or malformed sources still fail closed.
- The registered claim-count and 30-word limits remain unchanged.
- Invalid cells add only the error category and structural key/count metadata;
  they retain no question, claim, answer, or source text.

The v1 configuration, checkpoint, receipt, and $0.21765224 spend remain
immutable. V2 uses a fresh artifact root and checkpoint.

## Unchanged measurement contract

V2 preserves the 1,397-document corpus, 49 qualified questions, deterministic
development/held-out split, three-question witness, both retrieval arms,
DeepSeek V4 Pro generator, Claude Haiku judge/scorer, answer and map token
budgets, lifecycle rules, uncertainty method, acceptance boundary, projected
$9.50 spend, and proposed $12 hard cap.

## Gate and sequence

1. Pass the full zero-spend input, FathomDB 0.8.23, retrieval, lifecycle,
   Airlock-alias, and resilience preflight under the v2 configuration hash.
2. Obtain explicit HITL authorization and a hard USD cap for v2.
3. Run A/A and the three-question witness from a fresh checkpoint.
4. Stop and issue an invalid receipt on any semantic, attribution, lifecycle,
   completeness, or cost-cap failure.
5. Run the 39-question held-out comparison only if the witness is valid.

Coreyt authorized v2 model-completion calls on 2026-08-29 under the registered
$12 hard cap and witness-before-held-out sequence.

The zero-spend gate passed under configuration SHA-256
`15096cae2823d621954a640e185c8238e01ff8b46150c743c9c644a2f2af4cee`.
The registered receipt is `awaiting_hitl`.

## Basis

- [Stopped v1 witness](../../experiments/runs/global-01-lazy-witness-20260829T1924Z-aa159044/record.json)
- [V2 preflight result](2026-08-29-global-01-v2-preflight-result.md)
- [V2 preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T2026Z-15096cae/record.json)
- [V1 execution note](2026-08-29-global-01-witness-execution-note.md)
- [Original measurement contract](2026-08-29-global-01-lazy-coverage-contract.md)
