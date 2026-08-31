# REASON-01 compact-ledger design review

**Reviewer:** independent read-only subagent  
**Date:** 2026-08-30  
**Initial verdict:** revise  
**Final verdict:** approved

## Required corrections

The initial review required one common blinded claim scorer for all arms, exact
quotes rather than model paraphrases as compact evidence, an enforceable compact
input budget, canonical scorer-input deduplication, an authoritative spend cap,
regenerated and interleaved raw arms, exact scorer pinning, a descriptive
decision rule, runner-owned provenance spans, and a new offshoot checkpoint.

The re-review found one remaining issue: compact grounding could have been
credited from a complete source body that the compact reader never saw. The
design now gives the common judge only each arm's actual reader evidence unit
and requires auditable per-claim entailment plus supporting citation IDs.

## Disposition

All blocking and high findings are resolved in the design. Implementation must
still pin exact OpenRouter model IDs, preflight every ledger prompt against the
registered model context, and include focused tests for arm interleaving,
scorer deduplication, strip-versus-body evidence, provider-cap refusal, and the
per-case compact input limit.

## Run-readiness amendments

The reviewer subsequently approved three bounded implementation amendments:

- a quote-mark-only canonical source fallback, frozen by table and hash, after
  exact matching fails and with all other normalization prohibited;
- a fixed five-attempt requested-seed schedule, recorded as retry
  diversification rather than reproducibility; and
- Claude Haiku 4.5 as the specialized exact-strip extractor while DeepSeek V4
  Pro remains the identical answer model for every arm.

The final approval required two frozen extraction probes, full-stage
cost/latency reporting, and inclusion of all abandoned diagnostic calls in the
authoritative provider spend. Both probes passed. The approved full run permits
transport resume only; it does not permit further prompt, route, limit, or
validation changes in response to outcomes.
