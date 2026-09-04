---
title: 0.8.25 Slice 20 design review — cycle 4
status: FAIL_CORRECTION_APPLIED
reviewed_design_version: 6
reviewed_on: 2026-09-03
---

# Slice 20 design review — cycle 4

## Verdict

FAIL. Design v6 repaired dependency-owned projection gaps but would have
required Slice 20 to redesign the complete historical global cursor/rebuild
state machine. Design v7 removes that coupling and requires cycle 5.

## Findings and disposition

| ID | Priority | Finding | Design v7 disposition |
| --- | --- | --- | --- |
| D20-15 | P1 | Rebuild reconstruction covered dependency cursors but not unrelated operational or erased-canonical gaps in the shared sequence. | Dependency mutation now uses a separate monotonic generation and never consumes a global cursor or projection terminal. |
| D20-16 | P2 | Boundary schema/open validation did not enforce exclusive global cursor ownership. | The global-boundary/marker design was removed; dependency generation is a singleton state version checked against extant row generations. |
| D20-17 | P2 | Slice 25 ownership of marker, terminal, advancement, and in-memory publication was ambiguous. | Defined exact validator/apply-helper responsibilities, one generation advance per transaction, Slice 25 receipt behavior, and no canonical cursor publication. |
