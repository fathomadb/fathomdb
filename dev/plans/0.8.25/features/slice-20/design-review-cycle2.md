---
title: 0.8.25 Slice 20 design review — cycle 2
status: FAIL_CORRECTIONS_APPLIED
reviewed_design_version: 4
reviewed_on: 2026-09-03
---

# Slice 20 design review — cycle 2

## Verdict

FAIL. Design v4 resolved all cycle-1 findings, but three remaining contracts
were not executable. Design v5 applies the corrections and requires cycle 3.

## Findings and disposition

| ID | Priority | Finding | Design v5 disposition |
| --- | --- | --- | --- |
| D20-11 | P1 | Dependency high-water recovery could silently reuse a boundary after corruption or erasure. | Named and initialized the key; specified canonical parsing, row/key reconciliation, typed open failure, overflow refusal, atomic update, and restart tests. |
| D20-12 | P1 | Replay/read integrity omitted the canonical owner, source-version, and self-link side. | Required validation of the complete registration-to-canonical chain for registration, replay, and both lookups, with raw-corruption cases for every link. |
| D20-13 | P2 | Lookup wire shapes, exact SDK names, and required-field precedence were incomplete. | Added both lookup objects, exact method names/returns, per-request field order, dependency-ID path, and removed irrelevant boolean/enum language. |
