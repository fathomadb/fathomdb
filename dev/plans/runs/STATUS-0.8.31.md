# FathomDB 0.8.31 release status

**ACTIVE — release implementation on `release/0.8.31`.**
The [state](../release-state-0.8.31.json) owns structured status and next
actions. The [master plan](../0.8.31/plan.md) and [scope](../0.8.31/scope.md)
own settled HITL decisions, requirements, and execution contracts.

## Current state

- **Slice 0 (`PREFLIGHT`) & Phase A Prework Audits (`Slices 1–9`) — LANDED (`bf8e905e3`, `42595212c`):**
  - Merged completed `origin/inspector/dev/0.2.1` (`d6501cc5a`) cleanly at `086d5af0a`.
  - Landed RED→GREEN worktree Python/TS bootstrap fix (`20269cee4` → `db271323f`) and bootstrapped owned `.venv`, `node_modules`, and `src/ts/node_modules`.
  - Authored ADRs `#73`–`#76`, W12 fence review, `v0.8.30` baseline latency measurements, and closed golden schema-37 DB.
  - Adopted `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` (`6ee16b9e8`), remediated all Slice 0 review findings (`bf8e905e3`), completed Phase A Prework Audits (`Slices 1–8`), and obtained **HITL Gate 1 approval** (`2026-11-22`) on `prework/slice-9-triage.md`:
    - `P4-09`: add `_fathomdb_commit_timestamps(write_cursor, committed_at_ms)` in Schema 38.
    - Renumbered ladder to `Slice 10 = Prework Prep`, feature slices `20–80` (`80 = Publication`, held for separate HITL approval).
    - Pre-authorized `governed-surface-pin.json` updates for `read.operational_collections`/`read.operational_collection` (Slice 20) and `resolve_evidence_many` (Slice 50).
    - Recorded strict project-boundary rule (no Memex-specific information written into tracked FathomDB repo/public files).
- **Slice 10 (`PREWORK-PREP`) — PLANNED (awaiting HITL Gate 2 approval of `features/slice-10/plan.md`).**

## Next action

Complete adversarial plan review of Slice 10 (`features/slice-10/plan.md`) and Phase B release plan (`plan.md`), present HITL Gate 2, then execute Slice 10 and fan out Phase C draft plans for feature slices `20–70`.
