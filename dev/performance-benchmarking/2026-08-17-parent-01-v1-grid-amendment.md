# PARENT-01 v1 LOCOMO grid amendment

**Status:** HITL-approved treatment; implementation and execution are not yet
commissioned.

## Authority

HITL decision `seq-250` approves `parent_child_turn_session_v1` as the one
bounded PARENT-01 treatment in the authorized LOCOMO grid. It is covered by the
LOCOMO external-execution authorization in `seq-249`, subject to the normal
frozen configuration, worker, review, receipt, and artifact controls.

Future empirically motivated parent-child variants are permitted only through a
new dated frozen amendment, human-intended tests, independent review, and a
matched control. They may not expand the public surface, introduce a paid
service or extractor, or use a new corpus without separate authorization.

## Frozen treatment

| Field | `parent_child_turn_session_v1` |
| --- | --- |
| Child unit | One individual LOCOMO dialogue turn |
| Parent relation | The child's exact enclosing session; no cross-session or cross-conversation relation |
| Child retrieval | The already-frozen LOCOMO `hybrid` treatment, evaluated at its existing top-10 child cutoff |
| Parent selection | Deduplicate by parent session and retain at most five session bundles |
| Fusion | Reuse the hybrid child rank. A parent takes its best child's original rank; ties sort by stable parent session ID. No second score fusion is allowed. |
| Neighbor bound | At most one immediately preceding and one immediately following turn within the same parent session |
| Returned context | The seed child plus its bounded neighbors in chronological order: at most three turns per session bundle and 15 turns across five bundles |
| Runtime cells | The matching hybrid CPU/GPU and cold/steady cells; no cross-encoder is enabled by this treatment |
| Safe attribution | Each bundle records parent session ID, seed-child ID, ordered neighbor IDs, and TRACE-compatible source identity. Receipts retain only IDs, hashes, counts, and fixed diagnostics. |

## Evaluation and stop rules

The existing LOCOMO M1 R@10 rule remains a child-retrieval measure against A0.
The treatment additionally reports child evidence recall, parent/session recall,
duplicate rate, context-expansion count, and per-class M6/M7 latency. Missing
parents, ambiguous membership, a cross-session neighbor, or an unattributed
bundle fail closed. It may become an ANSWER-01 candidate only if it meets the
LOCOMO retrieval and latency rules without a material class regression.
