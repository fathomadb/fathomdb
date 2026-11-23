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
- **Slice 10 (`PREWORK-PREP`) — LANDED (`3c4b84439`):**
  - Completed `WP-10A`–`WP-10D` (ADR `#73`/`#75`/`#76` & `ADR-0.8.28` amendments + project-boundary scrub; `markdownlint-cli2 ^0.24.2` & `rustix 1.1.6` lockfile hygiene; pre-publish `0.8.31` public-doc truth gate flip + `P6-01`/`P6-07`/`P6-11` fixes; `P7-03` security timeouts, `P7-07` PEP 691 + CDN-skew retry, `P7-08` CHANGELOG GitHub Release body, `P7-09` Bash 3 sentinel + Windows Slice 55 smoke, `P7-10` dynamic tag route + audited 171-record Gitleaks manifest; all wired in `scripts/agent-test.sh`).
  - Independent `pro` adversarial code review returned `APPROVE` after `FIX-1`; recorded in `features/slice-10/status.md`.
- **Phase C Draft Plans (`Slices 20–70`) — AUTHORED:** Draft plans written for Slices `20`, `30`, `40`, `50`, `60`, and `70`.
- **Slice 20 (`PROPERTY-SCHEMA`) — LANDED (`975637f3c`):**
  - Landed Layers 1–5 (`022b62b49`, `3da57de2e`, `644333afb`, `947398398`, `36802ba4e`, `9ee29cd3e`) plus `FIX-1` (`1c61aea20`) and `FIX-2` (`975637f3c`): Schema 38 (`STEP_38_SQL`), inline typed properties + O(1) hydration, `canonical_property_values` with 4 composite indexes and indexed `INNER JOIN` vector pre-selection, `vec0` 8-slot node budget, `_fathomdb_commit_timestamps`, explicit opt-in `R1` vector kinds, `read.operational_collections`/`collection` (`55/55` governed surface pinned at `36802ba4e`), W12 erasure fixes 3a/3b/3c, `ExportClass` audit, Rust/Python/TypeScript SDK parity, and updated interface docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `975637f3c`; recorded in `features/slice-20/status.md`.
- **Slice 30 (`STRICT-GRAPH`) — LANDED (`8e44a47ad`):**
  - Landed Layers 1–4 (`0a43d66b7`, `3f8dbc470`, `87cd6a633`, `eda733775`) plus `FIX-1` (`8e44a47ad`) after moving the worktree to `/mnt/data/worktrees/release-0.8.31`: strict write endpoint integrity + 32 convenience-edge test & 3 Inspector forensic fixture updates, `GraphExpandRequestV1` `#[non_exhaustive]` + `::new(...)` + closed typed predicates + current-time SQL pushdown (`UNION ALL`, `t_valid`, `proptest` equivalence), `search.rs` `SEED_FTS_N` mixed-edge fix + Phase 2 `UNION ALL` + opt-in `graph_seed` (`GraphFts`/`FusedTopK`/`Both`) + single-pass `CeScoreCache` + `PerHitExplain`/`EvidenceGraphOriginV1` provenance + `#[non_exhaustive]` `EvidenceSearchRequestV1::new(...)`, Rust/Python/TypeScript parity (1,789 Python / 653 TS / all Rust crates), and updated interface/public docs.
  - Independent adversarial code review returned **`APPROVE`** at HEAD `8e44a47ad`; recorded in `features/slice-30/status.md`.
- **Slice 40 (`HISTORY-GRAPH`) — ACTIVE:** Applying Plan Review `FIX-1` to `features/slice-40/plan.md` for `APPROVE`, then executing RED -> GREEN TDD implementation.

## Next action

Complete Slice 40 (`HISTORY-GRAPH`) Plan Review `FIX-1` (`APPROVE`), then execute Slices `40 → 50 → 60 → 70` sequentially (stopping before Slice 80 Publication for separate HITL approval).
