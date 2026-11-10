# Slice 7 delivery requirements

Proposed output from November 10 source inspection at `6edc1155f`.

| Need / requirement / criterion | Concrete review result |
| --- | --- |
| P7-N1 / R1 / A1 | Owned venv/npm missing, Node/npm mismatch and root capacity must resolve before compilation. |
| P7-N2 / R2 / A2 | Full/default verifier runs lint, typecheck, security, tests; heavy alone is not the full gate. Preserve failed/assembled historical evidence. |
| P7-N3 / R3 / A3 | Runner delivery, protected-route authorization and actual product execution are distinct; bound concurrency and post-build upload/cache headroom. |
| P7-N4 / R4 / A4 | W/E axes and package consumer checks remain; SDK OIDC repair is already recorded complete. |
| P7-N5 / R5 / A5 | Current-tree secret enforcement and isolated, redacted history classification are existing controls; no history rewrite or broad allowlist. |
| P7-N6 / R6 / A6 | First-tag bootstrap is hardcoded to v0.8.28; .29 needs an exact approved routing procedure before publication rehearsal. |

See [delay register](delay-register.md), [design](design.md), and
[test mapping](test-mapping.md). Remote access, actual timing/cost savings and
phase-specific peak disk budgets have not been measured.
