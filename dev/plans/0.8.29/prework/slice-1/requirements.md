# Slice 1 dependency review requirements

Proposed source/selected-primary-source audit, November 10 at `6edc1155f`.

| Need / criterion | Evidence and limitation |
| --- | --- |
| S1-N1 / R1 / AC1 | [Candidates](candidate-register.md) account for all nine local refs and source pin groups. Survey discovery identifies 119 recognized manifests, including 79 untiered; exporter requirements are omitted. Comprehensive sweep is not complete. |
| S1-N2 / R2 / AC1 | [Design](design.md) retains deliberate fork/SQLite/MCP/tool/API baselines; unknown pin rationale stays investigate. |
| R3 / AC2 | Selected official Rustls/DOMPurify advisories mapped to locked versions, reachable features and unproven exploit conditions. No whole-tree security-clean claim. |
| R4 / AC2–3 | [Test mapping](test-mapping.md) assigns compatibility checks, current-lock adaptation and 9/10 versus fork140 ownership. No update executed. |

Accept or defer the discovery/classification gap explicitly at 9; selected
candidate research cannot satisfy the plan's all-manifest accounting criterion.
Live repository alerts, license/yank status across the graph and unassessed
external patched sources remain open research, not assumed clear.
