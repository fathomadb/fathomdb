---
title: Slice 71B — attribution harness TDD chronology
status: COMPLETE
date: 2026-09-08
---

# Slice 71B attribution harness TDD chronology

The attribution harness was developed as committed RED/GREEN pairs before the
sealed campaign. These commits changed experiment contracts and harness code,
not FathomDB product behavior.

| Commit | State | Evidence |
| --- | --- | --- |
| `90f3bfa7` | RED | Defines the initial measurement contract and failing focused tests. |
| `27bca1f8` | GREEN | Seals the write-attribution protocol and satisfies the initial contract. |
| `3c0eb4fa` | RED | Pins review corrections for ordering, identity, timing, signatures, and interpretation. |
| `ab1cc70e` | GREEN | Binds corrected attribution evidence and satisfies those tests. |
| `a9e0148c` | GREEN hardening | Normalizes the sealed probe lock without changing the campaign question. |
| `5678636e` | RED | Requires failed attribution attempts to bind their retained evidence. |
| `1082b3b6` | GREEN | Binds aborted-campaign evidence and canonical failure disposition. |
| `a7ca1d94` | RED | Requires active-cell harness failures to retain exact matrix context and artifacts. |
| `76e42b90` | GREEN | Retains active harness failures and passes the final focused review gate. |

At `76e42b90`, all 34 focused contract tests passed. The custom manifest
validator, both JSON Schemas, runner digest, and JSON parsing also passed.
Independent read-only code review found no remaining correctness issue and ran
no timed or broad verification.
