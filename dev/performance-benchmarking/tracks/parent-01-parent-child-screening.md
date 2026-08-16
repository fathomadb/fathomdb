# PARENT-01 — Parent-child retrieval screening

**Status:** planned; joins LOCOMO-01 only through a frozen plan amendment.

## Decision

Does child retrieval followed by parent/session and bounded-neighbor context
improve evidence recovery and answer context over parent-only retrieval?

## Preparation and contract

1. Close TRACE-01 for the selected child and parent projections.
2. Freeze one treatment: child unit, parent relation, deduplication rule,
   neighbor bound, candidate limit, fusion rule, and returned-context shape.
3. Extend LOCOMO provenance and metrics to report child evidence recall, parent
   recall, duplicate rate, context expansion, and per-class latency.
4. Write tests for parent mapping, duplicate removal, missing-parent handling,
   neighbor bounds, and receipt isolation from public API changes.

## Exit evidence

The treatment is eligible only if it clears LOCOMO-01’s retrieval and latency
rules without a material class regression. It becomes an ANSWER-01 candidate,
not a product default, only after that evidence exists.
