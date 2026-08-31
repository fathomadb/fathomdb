# LOCOMO-01 fixed-subset v3 amendment

**Tracks:** `LOCOMO-01`, `PARENT-01`  
**Date:** 2026-08-17  
**Authority:** direct HITL instruction to generate the missing question set.

## Replacement control

The withdrawn external `locomo-01-fixed-subset.v2` is superseded for the
authorized dry run by external `locomo-01-fixed-subset.v3`. It retains the
existing `locomo-fixed-subset.v1` data schema and changes only the external
control identifier and SHA-256 pin in `phase-b-execution.v1.json`.

The v3 control has exactly 32 unique canonical LOCOMO question identifiers and
SHA-256 `88434dd0bfbbcae73117ebbfd493220885e38441e4377212eb125c8ebf897933`.

## Deterministic construction

The source is the historical content-free A0 dry retrieval projection with
SHA-256 `4bab8eb140bfe7e601166ad2946e0a337acf480551754e3f3c96ac296810348e`.
It contributes only stable identifiers and LOCOMO categories. Category 3
(open-domain) is excluded under `eval.locomo_loader`'s existing class map.

The v3 selection uses seed `20260817 + category` over lexically ordered
historical IDs, converts `conv<N>_q<N>` to the canonical
`locomo-<N>-q-<N>` form, then writes lexical canonical order. Its fixed
allocation is 11 multi-session, 11 temporal, and 10 factoid questions. No
corpus text, questions, answers, evidence, predictions, or historical raw
output enters this repository.

## Scope and remaining gate

This amendment neither changes the normalized corpus pin, treatment grid,
metrics, CPU/GPU boundary, nor the `parent_child_turn_session_v1` treatment.
PARENT canonical identity is governed separately by the parent-proof v2
amendment. Independent review and a coordinator release remain required before
the five-cell dry run.
