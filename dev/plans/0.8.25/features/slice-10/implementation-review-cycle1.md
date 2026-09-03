---
title: 0.8.25 Slice 10 implementation review — cycle 1
status: FAIL_FIXED_IN_FIX_2
reviewed_commit: e6e4b049
reviewer: independent subagent
---

# Slice 10 implementation review — cycle 1

## Verdict

FAIL. FIX-2 was required before Slice 10 could close.

## Findings and disposition

1. **P1 — A nonexecuted or wrong-component witness could support an Engine
   metric.** FIX-2 requires every data-plane Engine metric to cite an executed
   witness whose component kind is `fathomdb_engine_search`. An
   `Engine.search` call path cannot be assigned to another component kind, and
   a graph arm's witness must agree with the arm's Engine component.
2. **P1 — The provisional v1 correction broke receipt immutability.** FIX-2
   restores the byte-original canonical record, pins its hash in the closed v2
   policy, fully validates its v1 sidecar, and makes every legacy classification
   ineligible as successful evidence.
3. **P2 — Persistence failures could append an index row without its sidecar.**
   FIX-2 adds a `before_index` finalization hook to the shared receipt writer.
   Runtime/result evidence and the validated sidecar must now exist before the
   append-only index registration occurs; hook failure leaves no index row.
4. **P2 — Negative coverage was incomplete.** FIX-2 adds RED/GREEN cases for
   nonexecuted and wrong-component Engine witnesses, legacy success rejection,
   and pre-index hook failure.

The remaining preflight errors occur before a run is registered. An execution
failure after valid preflight still produces a normal blocked record and
sidecar before index registration. An inability to write or validate the
receipt itself fails closed without registering an unverifiable run.
