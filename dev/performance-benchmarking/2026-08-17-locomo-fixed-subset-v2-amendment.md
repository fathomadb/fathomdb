# LOCOMO-01 fixed-subset v2 withdrawal

**Tracks:** `LOCOMO-01`, `PARENT-01`  
**Date:** 2026-08-17  
**Authority:** direct HITL instruction to generate the missing question set.

## Withdrawal

The absent `locomo-01-fixed-subset.v1` was provisionally superseded for the
authorized dry run by external `locomo-01-fixed-subset.v2`. It retained the
existing `locomo-fixed-subset.v1` data schema and changed only the external
control identifier and SHA-256 pin in `phase-b-execution.v1.json`.

The v2 control has exactly 32 unique identifiers and SHA-256
`41296bdc7202252b9d1caa27f38039e2b3b3aa4e9e5327c17f7276a3e1744228`. Its
identifier conversion was found not to have produced the canonical LOCOMO
form required by the factual qualifier. It is withdrawn and must not be used
for a benchmark run. The corrected control is recorded in the v3 amendment.

## Deterministic construction

The source is the historical content-free A0 dry retrieval projection with
SHA-256 `4bab8eb140bfe7e601166ad2946e0a337acf480551754e3f3c96ac296810348e`.
It contributes only its stable IDs and LOCOMO categories. Category 3
(open-domain) is excluded under `eval.locomo_loader`'s existing class map.

The v2 selection uses seed `20260817 + category` over lexically ordered
historical IDs, converts those IDs to the canonical current LOCOMO identifier
form, then writes lexical canonical order. Its fixed allocation is 11
multi-session, 11 temporal, and 10 factoid questions. No corpus text,
questions, answers, evidence, predictions, or historical raw output enters
this repository.

## Scope

This withdrawal neither changes the normalized corpus pin, treatment grid,
metrics, CPU/GPU boundary, nor the `parent_child_turn_session_v1` treatment.
