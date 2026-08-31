# LOCOMO-01 and PARENT-01 Phase-B execution contract

**Tracks:** `LOCOMO-01`, `PARENT-01`

**Date:** 2026-08-16

**Status:** executable preparation only. This contract does not itself start a
benchmark, acquire LOCOMO, select a GPU, invoke a model, or create an external
artifact root.

## Purpose and authority

`experiments/configs/locomo-01/phase-b-execution.v1.json` and
`experiments.locomo_phase_b` turn the approved LOCOMO/PARENT preparation into
a content-free, release-gated execution plan. The configuration consumes the
integrated Phase-A catalog by its exact checked-in SHA-256 and keeps all corpus
and provenance inputs external and hash-pinned.

HITL decision `seq-249` authorizes the LOCOMO fixed-subset dry run, Phase-B
CPU/FTS grid, and GPU/cross-encoder cells. `seq-250` authorizes exactly one
PARENT-01 addition: `parent_child_turn_session_v1`. This contract does not
extend either authorization to a new corpus, paid service, extractor, public
API, answer scorer, product default, or parent-child variant.

## Frozen execution plan

The resolved plan has 52 distinct cells:

| Cells | Count | Program track | Meaning |
| --- | ---: | --- | --- |
| Frozen Phase-A grid | 48 | `LOCOMO-01` | Two ingest units, six retrieval treatments, and separate CPU/GPU cold/steady cells. |
| Parent-child treatment | 4 | `PARENT-01` | Turn-level hybrid child retrieval in the matching CPU/GPU cold/steady cells; no cross-encoder. |

The fixed 32-question dry-run subset is external-only and hash-pinned. It runs
only these five cells, in the listed order:

1. `turn--fts_only--cpu--cold`
2. `turn--hybrid--cpu--steady`
3. `session--fts_only--cpu--cold`
4. `session--hybrid--cpu--steady`
5. `turn--parent_child_turn_session_v1--cpu--cold`

Every planned cell carries its own `program_track`; no PARENT result may be
reported as an ordinary LOCOMO treatment.

## Parent-child treatment

`parent_child_turn_session_v1` is immutable in this configuration:

| Property | Frozen value |
| --- | --- |
| Child | One individual LOCOMO turn from the existing hybrid top-10 ranking. |
| Parent | The exact enclosing session only. |
| Session selection | Deduplicate by session, retain at most five. |
| Parent rank | Best child’s original hybrid rank, then stable parent-session ID. |
| Fusion | None; no second score or rank fusion. |
| Context | Seed child plus at most one predecessor and one successor from that same session; three turns per bundle and 15 total. |
| Attribution | Parent-session ID, seed-child ID, ordered-neighbor IDs, and TRACE-compatible source ID only. |

The adapter accepts a child only when provenance resolves exactly one parent
session, gives the child a non-negative session ordinal, and carries a
TRACE-compatible source ID. Each neighbor must separately carry its ID, parent
session ID, ordinal, and TRACE attribution. It is accepted only at ordinal
`child − 1` or `child + 1` in that same session; output is ordered by ordinal.
Compact neighbor strings are rejected. The adapter also rejects missing or
ambiguous hit fields, ranks outside the hybrid top-10, duplicate or seed-equal
neighbors, more than two neighbors, and duplicate neighbor ordinals. It never
infers a second supersession representation: TRACE-01 remains the lifecycle
mapping authority.

## Metrics and safe evidence

All cells retain the Phase-A M1/M2/M4-proxy/M6/M7 definitions and report the
factoid, temporal, and multi-session classes separately. PARENT cells add
child-evidence recall, parent/session recall, duplicate rate, context
expansion, and class-level latency. M4 remains retrieval-only temporal evidence
recall; no judge-scored answer-quality claim is produced here.

Every cell result is bound to its exact frozen cell ID and execution mode. A
receipt fails closed unless it has one unique result for every required cell:
the five listed dry-run cells in their frozen order, or all 52 full-grid cells.
It also requires M1, M2, M4-proxy, M6, M7, and all three class sections for
every result. PARENT cells additionally require child-evidence recall,
parent-session recall, duplicate rate, context-expansion count, and per-class
latency. A complete result cannot be closed with a missing, duplicated,
unknown, or wrong-mode cell.

Cell results expose only a safe logical external metrics reference, its
SHA-256, and aggregate scalar metric summaries. After a valid release, the
adapter can write the unchanged `experiments.record.v1` receipt and one
`experiments.index-row.v1` row through the shared helper. Both dry-run proofs
and full grids have verdict `complete` only after this validation; `n` is 32
for the fixed subset and 1,536 for the full evidence-backed LOCOMO grid. The
receipt records logical artifact names and digests, never external paths,
corpus text, questions, hits, model output, credentials, or historical receipt
paths.

## Release and execution boundary

`validate` and `preview` are always safe and make no external write. `execute`
requires a separate `locomo-phase-b.release.v1` token that contains:

- the resolved-config SHA-256;
- a safe release ID;
- `issued_by: track-runner-coordinator`;
- an independent-review commit SHA-256; and
- exactly `seq-249` and `seq-250` as its authorization references.

The runner also requires an already-existing external root outside the
repository. It rejects in-repository and historical-output destinations before
calling a cell executor. `dry_run` dispatches only the five fixed subset cells;
`full_grid` dispatches all 52 frozen cells. The dry-run dispatcher uses the
five frozen IDs in the stated list order, rather than incidental sort order.
The injected executor is the later reviewed integration seam; this preparation
module has no direct corpus, model, CUDA, network, or paid-service
implementation.

The release token is a coordinator-controlled integration record, not a claim
that a result is valid. An execution is complete only after every selected cell
has its external metrics evidence, a safe receipt and index row, and the
charter’s review and selection rules have been applied.
