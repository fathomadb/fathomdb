---
title: 0.8.25 Slice 20 design review — cycle 3
status: FAIL_CORRECTION_APPLIED
reviewed_design_version: 5
reviewed_on: 2026-09-03
---

# Slice 20 design review — cycle 3

## Verdict

FAIL. Design v5 resolved every cycle-2 finding, but its new global boundary had
no durable projection-terminal ownership. Design v6 applies the correction and
requires cycle 4.

## Finding and disposition

| ID | Priority | Finding | Design v6 disposition |
| --- | --- | --- | --- |
| D20-14 | P1 | A dependency registration consumed a global cursor without a readiness terminal; full/vector rebuild would also erase a naive terminal and permanently wedge later readiness. | Added an append-only no-projection-boundary marker, atomic terminal and readiness advancement, exclusive Slice 25 cursor ownership, fail-closed marker/terminal integrity, permanent marker retention after erasure, rebuild reconstruction, and focused RED/operator cases. |
