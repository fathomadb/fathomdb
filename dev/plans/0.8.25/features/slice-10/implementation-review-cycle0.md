---
title: 0.8.25 Slice 10 implementation review — cycle 0
status: FAIL_FIXED_IN_FIX_1
reviewed_commit: c7fd9fba
reviewer: independent subagent
---

# Slice 10 implementation review — cycle 0

## Verdict

FAIL. FIX-1 was required before Slice 10 could close.

## Findings and disposition

1. **P1 — Engine execution was not enforced.** Engine-contributed metrics and
   comparison arms could omit witnesses, and `unknown_historical` could support
   a successful data-plane claim. FIX-1 requires a matching per-arm witness and
   rejects unknown Engine execution for data-plane metrics.
2. **P1 — Measurement roots were sidecar-selected.** A classifier could omit a
   numeric quality leaf by excluding it from its roots. FIX-1 moves roots and
   exclusions into the hashed measurement plan and requires every metrics
   payload scalar to be either classified or explicitly excluded.
3. **P2 — The lint leg selected only `.venv/bin/python`.** FIX-1 uses the first
   available Python 3.11+ interpreter, preserving clean-clone portability.
4. **P2 — The native witness did not bind its actual runtime.** FIX-1 records a
   hashed runtime attestation for the interpreter, package, native extension,
   and CLI, plus a separate hashed result-detail artifact.
5. **P2 — Negative coverage was incomplete.** FIX-1 adds tests for missing
   witnesses, unknown historical execution, hidden metrics, overlapping roots,
   source-hash mismatch, and typed native failure mapping.

The provisional v1 native receipt is retained in the append-only index. Its
machine-absolute ancillary setup locators make it invalid for the normal run
contract, so the byte-original record is quarantined under
`experiments/superseded-runs/` and remains attributable to `c7fd9fba`. Policy
v2 explicitly maps and fully validates those original bytes while marking the
run superseded, so it cannot support a successful claim; a clean v2 witness is
required for closure.
