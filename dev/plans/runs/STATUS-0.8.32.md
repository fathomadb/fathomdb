# FathomDB 0.8.32 release status

**ACTIVE on `dev/0.8.32` (`/mnt/data/worktrees/release-0.8.32`).**
The [state](../release-state-0.8.32.json) owns structured status, decisions,
and generated regions. The [master plan](../0.8.32/plan.md),
[scope](../0.8.32/scope.md), and [Slice 9 triage](../0.8.32/prework/slice-9-triage.md)
own settled HITL Gate 1 decisions, requirements, and execution contracts.

## Current state

<!-- BEGIN GENERATED release-state:0.8.32:status-current-state -->**Next is Slice 40 (MCP-RW), NOT_STARTED.** Completed on local `dev/0.8.32` per release state: 10 (`e8853997a`) · 20 (`165c445c2`) · 30 (`e3a95688a`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.32:status-current-state -->

- **Phase A Prework Audits (`Slices 0–9`) — COMPLETE (HITL Gate 1 approved 2026-11-24):**
  - Cut `dev/0.8.32` worktree at `/mnt/data/worktrees/release-0.8.32` from `main` (`38a1594bf`).
  - Completed read-only prework audits (`prework/slice-0.md` through `prework/slice-8.md`) and consolidated 102-proposal triage (`prework/slice-9-triage.md`).
  - **HITL Gate 1 rulings (2026-11-24):**
    - **Decision 1:** Option 1 (`2026-11-24`) — `S39-COMPOSITE` (`SCHEMA_VERSION = 39`, composite `(scope, ...)` PKs/indexes, 8-col `vec0`, verified `fathomdb migrate 38 → 39`; `ADR-0.8.32-caller-scope-isolation.md` `#102` accepted). Single-scope (one agent per database, `scope = ""`) is the default posture and MUST be documented in `docs/`; `0.8.33` notes revisiting/relaxing that direction.
    - **Decision 2:** Option 1 — keep `SyncMode::Normal` default + opt-in `SyncMode::Full`; qualify both in Slice 50 QEMU/KVM `dm-flakey`.
    - **Decision 3:** Option 1 — canonical ladder `Slice 10` (`PREWORK-PREP`) and feature slices `20–70` (`70 = PUBLICATION`, separately HITL-gated).
    - **Decision 4:** Option 1 — pre-authorized `governed-surface-pin.json` updates (`Pin A` / `Pin B` / `Pin C`; `core = 5` and `recovery_denylist = 5` fixed).
    - **Decision 5:** Option 1 — `--features default-embedder` for 5-target `fathomdb-mcp` GH Release binaries & PyPI wheels; `default = []` on crates.io.
    - **Decision 6:** Option 1 — all recommended `include` / `postpone` / `reject` dispositions in `prework/slice-9-triage.md` approved.
    - **Decisions 7 & 8 (`2026-11-24`):** Rust `1.95.0 → 1.100.0` deferred to `0.8.33`; Slice 50 official Ubuntu `24.04` cloud-image download (`cloud-images.ubuntu.com`) approved.
    - **Decision 9 (`2026-11-25`):** Option B2-strict — fix `0.8.31` empty-DB reopen cursor bug (`owner_kind` on `_fathomdb_projection_terminal`) and per-class projection namespace (`PRIMARY KEY(entity_class, name)` + strictly typed `ProjectionTarget` across Rust/SDK/Python/TS) in Slice 40 (`WP-40A1`).
    - **Decision 10 (`2026-11-25`):** Option 2 (`R32-08`, `WP-40D`) — cache pooled-reader projection-generation status across `search(..., explain=true)`, `search_frozen(..., explain=true)`, `search_with_evidence`, and query-seeded `graph.expand`, and expose additive `PerHitExplain.vector_cosine: Option<f64>` across Rust/SDK/Python/TS.
- **Slice 10 (`PREWORK-PREP`) — LANDED (`e8853997a`):** Completed `WP-10A`–`WP-10E` + `FIX-1` (`f961e4537`), full unconfined `./scripts/agent-verify.sh` (`246/249` suites passed, `0` failed, `0` security violations), and independent `pro` adversarial review (`VERDICT: APPROVE`) per [features/slice-10/status.md](../0.8.32/features/slice-10/status.md). All Gate 1 decisions (`1–8`) are now ruled and Phase C Slices `20–70` scope is locked (Slice `70` publication separately HITL-gated).
- **Slice 20 (`SCOPE-AND-REFACTOR`) — LANDED (`165c445c2`):** Completed `WP-20A`–`WP-20E` + `FIX-1`–`FIX-3` (`7bd48de48`) — engine test relocation & `search`/`evidence` splits, declarative `TABLE_MANIFEST` (`35` tables), Schema 39 (`S39-COMPOSITE` + 8-col `vec0` + `fathomdb migrate 38 → 39` + golden `0.8.31` fixture), route-by-route `ScopePolicy`/`ScopePredicate` enforcement (single-scope default documented across `docs/`), and `Pin A` Rust/Python/TS parity — verified green by full unconfined `./scripts/agent-verify.sh` (`246/249` suites passed, `0` failed, `0` security violations), `mkdocs build --strict`, and independent `pro` adversarial review (`VERDICT: APPROVE`) per [features/slice-20/status.md](../0.8.32/features/slice-20/status.md).
- **Slice 30 (`BULK-AND-INSPECTOR`) — LANDED (`e3a95688a`):** Completed `WP-30A`–`WP-30C` + `FIX-1`–`FIX-3` (`56b897085`) — opt-in `WriteOptions` / `EndpointValidationMode::BulkNonStrict` / `Engine.write_with_options` (strict default preserved, cross-scope `ScopeViolation`, `fathomdb doctor` dangling/cross-scope verification), `Pin B` Rust/Python/TS parity (`57` ops / `95` allowlist, `core = 5`, `recovery_denylist = 5`), `TABLE_MANIFEST`-backed Inspector active-`scope` UI (`D-01` nondisclosure), and `inspector-browser-smoke` Playwright CI wiring — verified green by full unconfined `./scripts/agent-verify.sh` (`246/249` suites passed, `0` failed, `0` security violations), live Playwright smoke (`13/13`), `mkdocs build --strict`, and independent `pro` adversarial review (`VERDICT: APPROVE`) per [features/slice-30/status.md](../0.8.32/features/slice-30/status.md).

## Slice ladder

| Slice | Codename | Title | Depends on | Status |
| --- | --- | --- | --- | --- |
| 10 | `PREWORK-PREP` | Prework prep, baseline CI fixes, `R32-07` orientation/gate aids, immediate doc fixes, four `ADR-0.8.32-*` drafts, Slice 50 venue/fuzzing prep, and Decision 1 empirical experiment | Prework `0–9` | `LANDED` (`e8853997a`) |
| 20 | `SCOPE-AND-REFACTOR` | Engine test relocation & `search`/`evidence` hotspot splits, declarative `TABLE_MANIFEST`, Schema 39 (`S39-COMPOSITE` + 8-col `vec0` + `fathomdb migrate 38 → 39`), and engine-enforced caller scope & write authority (single-scope default, `Pin A`) | 10 | `LANDED` (`165c445c2`) |
| 30 | `BULK-AND-INSPECTOR` | Opt-in bulk/non-strict write mode (`Pin B`, strict default preserved) and Inspector active-scope UI + Playwright CI wiring | 20 | `LANDED` (`e3a95688a`) |
| 40 | `MCP-RW` | Atomic governed write + `latest_state` checkpoint seam + `EP-07`/`EP-01` (`Pin C`), `Gate1-D9` reopen cursor & per-class projection namespace, production read+write `fathomdb-mcp` + 3-channel packaging, and `Gate1-D10` cached pooled-reader projection status & `PerHitExplain.vector_cosine` | 20 | `NOT_STARTED` |
| 50 | `DURABILITY-AND-FUZZ` | Internal `fathomdb.logical-export/v1` backup/restore/rebuild proof, `SyncMode::{Normal, Full}` + QEMU/KVM `dm-flakey` power-loss campaign, and isolated `fuzz/` workspace | 30, 40 | `NOT_STARTED` |
| 60 | `QUALIFICATION` | Full-workspace qualification, paired `0.8.31` vs. `0.8.32` 10k/50k/100k latency/RSS/DB-size gate, candidate smokes, release docs, and golden `0.8.32` archive | 50 | `NOT_STARTED` |
| 70 | `PUBLICATION` | Authorized `0.8.32` publication across crates.io, PyPI, npm, GitHub Releases, and Tegra Pages (held for separate HITL publish approval) | 60 | `NOT_STARTED` |

## Next action

<!-- BEGIN GENERATED release-state:0.8.32:status-next-action -->**Commission Slice 40 (MCP-RW)** — Atomic governed write + latest_state checkpoint seam + EP-07/EP-01 (Pin C), Gate1-D9 reopen cursor & per-class projection namespace, production read+write fathomdb-mcp + 3-channel packaging, and Gate1-D10 cached pooled-reader projection status & PerHitExplain.vector_cosine. **Remaining ladder:** 40 → 50 → 60 → 70.<!-- END GENERATED release-state:0.8.32:status-next-action -->
