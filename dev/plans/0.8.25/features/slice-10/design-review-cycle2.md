---
title: 0.8.25 Slice 10 independent design review — cycle 2
status: FAIL_RESOLVED_BY_FIX_3
reviewed_design_version: 4
---

# Slice 10 independent design review — cycle 2

The independent reviewer returned **FAIL** on version 4. FIX-3 is the final
allowed design correction.

| Severity | Finding | FIX-3 disposition |
| --- | --- | --- |
| P1 | Evidence-only artifacts could not satisfy nonempty metric-root rules. | Version 5 permits roots only on metric payloads and requires evidence-only roots to be empty. |
| P1 | Mandatory lint depended on gitignored machine-local historical data. | Version 5 separates a deep raw-byte audit from portable lint and commits a compact, pinned derivation receipt. |
| P1 | Self-declared components could hide semantic contributors. | Version 5 requires a config-bound pre-execution measurement plan; historical manifest entries pin equivalent ownership. |
| P2 | Blocked reason was called closed without a schema or vocabulary. | Version 5 reuses the EARP blocker shape and defines the exact permitted code subset. |
| P3 | Architecture asserted a historical negative more strongly than its dirty receipt allowed. | Architecture now labels the bypass as retrospective analysis and the executable state as `unknown_historical`. |

No implementation begins unless cycle 3 returns READY with no unresolved
P1/P2. A further implementation-shaping failure exhausts the design FIX budget
and blocks the slice for explicit replanning.
