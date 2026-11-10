# Slice 6 evidence register

Inspected November 10, 2026; repository baseline `6edc1155f`. These are source
observations and bounded chat reads, not executed product qualification.

| Evidence | Observed claim and limit |
| --- | --- |
| [README](../../../../../README.md), [docs home](../../../../../docs/index.md), [Node install](../../../../../docs/install/typescript.md) | Published .28 examples and format 37 remain appropriate. Planning .29 does not obsolete them. |
| [MCP architecture](../../../../design/mcp-adapter/architecture.md) | Explicit .27 evaluation scope and a historical SDK capability table; annotate current applicability rather than deleting evaluation evidence. |
| [MCP design](../../../../design/mcp-adapter/design.md), [interface](../../../../interfaces/mcp.md) | As-is SDK preamble, dual structured/text envelope and text-filter refusal remain documented; interface separately marks planned Slice 60 support. |
| [Adapter edge](../../../../../src/rust/crates/fathomdb-mcp/src/edge.rs) | Current descriptions distinguish lexical text and hybrid filters; structured content is constructed. Compact text-default is still a later implementation delta. |
| [Release design](../../../../design/release.md), [.28 status](../../../0.8.28/features/slice-95/status.md) | SDK trusted-publisher repair is recorded; assembled qualification preserves failed original campaigns. |
| Chat “Complete Slice 90”, `01a1a80a-e240-7e82-9ced-d701d105a3be` | Directly read two latest turns, `01a1c37f-9494-7791-979c-c55f32da6f05` and `01a1c373-be13-7a21-a016-2916b1077ed5`, dated November 9. Final reports publisher saved and OIDC-only run 42750034084 passed; 35 build/publication jobs skipped. Settings and remote run were not independently rechecked today. |
| Chat “Find unfinished 0.8.28 work”, `01a1a8aa-844e-7ed0-84bf-5f6669d85986` | Directly read two latest turns: disk cleanup `01a1a9b5-f88f-76b3-bfcc-8928aa8c175b` (November 4, America/Chicago; November 5 UTC) and release audit `01a1a8ad-a818-7b33-9845-db8c5898f131` (November 4). Old `fdb9fa932`/format36/unmerged-.28 statements are superseded by current .29 Git/format37. Cleanup's 120 GiB was historical, not current capacity or fresh cleanup authorization. |
| [Bootstrap](../../../../../scripts/bootstrap.sh), [build](../../../../../scripts/agent-build.sh), [verifier](../../../../../scripts/agent-verify.sh) | Editable install instructions conflict with worktree isolation; verifier requires checkout-owned venv. Resolve before future preparation, not by borrowing an environment. |

Original chat tool/command listings were inspected only as needed; no raw
transcripts, credentials or other project data are persisted. Uninspected older
turns, generated examples and remote runner access remain explicit gaps.
