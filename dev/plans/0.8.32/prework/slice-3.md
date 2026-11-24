---
title: 0.8.32 Prework Slice 3 — Product Documentation CRUD Drafts
status: COMPLETE
target_release: 0.8.32
slice: 3
date: 2026-11-24
---

# 0.8.32 Prework Slice 3 — Product Documentation CRUD Drafts

## 1. Review basis, controlling invariants, and Phase B ladder alignment

Strictly read-only audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`), aligned with `dev/plans/0.8.32/prework/slice-0.md` (including Owner Rulings 1–4 and addenda) and `dev/plans/0.8.32/prework/slice-2.md` (`P2-01`–`P2-10`).

### Controlling invariants

1. **`dev/acceptance.md` is LOCKED.** Never add canonical `AC-NNN` items or mutate locked acceptance thresholds there. Release `0.8.32` binds behavioral proof to release-local `R32-01`–`R32-07`, slice-local acceptance criteria (`AC32-*`), MCP adapter requirements (`MR-*`), Inspector requirements (`INS-*`), and named RED→GREEN TDD tests.
2. **Non-breaking Schema 38 (`scope` activation).** `SCHEMA_VERSION` remains **38**. Existing `0.8.31` stores (`scope = ''`) open directly with zero migration.
3. **Governed surface pin (`scripts/check-governed-surface-pin.sh`).** Currently pins **56** canonical operations, **93** allowlist members, **5** `core` ops, **5** `recovery_denylist` names (`recover`, `restore`, `repair`, `fix`, `rebuild`), and **2** `runtime_controls`. `core` (5) and `recovery_denylist` (5) stay invariant. Any addition to `governed-{operation-parity,surface-allowlist}.json` requires **explicit HITL Gate 1 pre-authorization**.
4. **Python script separation (`AC-041` / `REQ-037`).** Base `fathomdb` `[project.scripts]` remains empty; `fathomdb-mcp` ships as an independent lockstep binary wheel (`src/python-mcp/`) exposed optionally via `fathomdb[mcp]`.
5. **Automated public-doc truth (`scripts/check-public-doc-truth.py`).** While `published: null`, no `PUBLIC_DOCS` may claim `0.8.32` is already published; `docs/compatibility/index.md` keeps `"no public automatic migration"` with `SCHEMA_VERSION` **38**.
6. **Strict FathomDB product boundary.** Zero external consumer-project specifics in tracked files.

### Canonical Phase B slice ladder mapping

| Phase B Slice | Draft `plan.md` Ref | Commissioned Documentation & Contract Scope |
| --- | --- | --- |
| **Slice 10** (`PREWORK-PREP`) | Slice 0 | `R32-07` orientation & gate aids, four `ADR-0.8.32-*.md` ADRs, baseline CI/Dependabot fixes, early `fathomdb-mcp` publisher/release prep, immediate factual `docs/` fixes |
| **Slice 20** (`SCOPE-AND-REFACTOR`) | Slice 10 | `search.rs`/`evidence.rs` submodule splits & 10 `*_tests.rs` relocations + `module-boundary-policy.txt` update, declarative `TABLE_MANIFEST`, non-breaking Schema-38 `scope` activation across Engine + Rust/Python/TS SDKs |
| **Slice 30** (`BULK-AND-INSPECTOR`) | Slice 20 | Opt-in bulk / non-strict write mode keeping default `engine.write` strict, plus Inspector active-`scope` UI & Schema-38+ alignment |
| **Slice 40** (`MCP-RW`, parallelizable with Slice 30 after Slice 20) | Slice 30 | Production read+write `fathomdb-mcp` per Owner Ruling 1 + addenda (`allow_writes: false` default, per-store opt-in + bound caller scope, fail-fast startup conflict checks, `AC32-MCP-SAFETY-*`, governed writes/`latest_state` checkpoints, scoped frozen evidence, selected `EP-*` unblockers) + Owner Ruling 3 packaging (GH Releases 5 targets + `crates.io` + PyPI `fathomdb-mcp`) |
| **Slice 50** (`DURABILITY-AND-FUZZ`) | Slice 40 | Backup/restore/rebuild proof + internal `fathomdb.logical-export/v1` contract, disposable local Linux VM / loopback `dm-flakey` power-loss fault venue + declared durability/oracle model (Owner Ruling 2), and broad coverage-guided fuzzing (`quality-backlog.md`) |
| **Slice 60** (`QUALIFICATION`) | Slice 50 | Full `./scripts/agent-verify.sh`, paired `0.8.31` vs `0.8.32` latency/RSS benchmark gate, `dev/needs.md` & `dev/requirements.md` applicability sync, pre-publish `docs/release-notes/0.8.32.md` + `docs/generated/*`, golden `0.8.32` DB archive |
| **Slice 70** (`PUBLICATION`) | Slice 60 | Post-HITL authorization publication truth flip (`RELEASE_SCHEMAS["0.8.32"] = 38`, published notices, multi-registry verification receipts) |

---

## 2. Governed-Surface Pin Impact — Explicit HITL Gate 1 Pre-Authorization Flags

1. **Candidate Pin Surface A — Schema-38 `scope` activation (Slice 20, `R32-01`):** Additive `ScopePolicy` / `scope` on `OpenOptions` / `EngineConfig` (and/or optional read/write options + `scope` fields on `NodeRecord`/`EdgeRecord`/`SearchHit`). If any new governed top-level type/error (`ScopeViolation`) or scoped helper enters `governed-{operation-parity,surface-allowlist}.json`, **HITL Gate 1 must pre-authorize updating `scripts/governed-surface-pin.json`**.
2. **Candidate Pin Surface B — Opt-in bulk / non-strict write mode (Slice 30, `R32-03`):** Keep `Engine.write(batch)` strict by default and `Engine.actuate` strict-only; exposing `WriteOptions` / `Engine::write_with_options` or `Engine::write_bulk` in governed parity **requires HITL Gate 1 pre-authorization**.
3. **Candidate Pin Surface C — `fathomdb-mcp` SDK unblockers (Slice 40, `EP-01` / `EP-05` / `EP-07` / `EP-10`):** Promoting `EP-01` (`read.visibility_state` / `store_state`), `EP-05` (`read.kinds` + visible counts), or `EP-07` (`open_without_create`) into `fathomdb-sdk` governed parity **requires HITL Gate 1 pre-authorization**.
4. **Zero Pin Change Confirmed — Slice 50 & `recovery_denylist`:** Internal `fathomdb.logical-export/v1` stays non-public; `core` (5) and `recovery_denylist` (5) stay unchanged.

---

## 3. Concrete Documentation & Contract CRUD Drafts by Domain

### 3.1 Product Needs, Requirements, and Slice-Local Acceptance Criteria

- **Keep Locked (`dev/acceptance.md`):** Unchanged.
- **Update (`Slice 60`, `dev/needs.md` & `dev/requirements.md`):** Sync `Current applicability` to `0.8.32` / Schema 38 (`REQ-032` stdio MCP & internal export clarity, `REQ-036` 3-root CLI, `REQ-037`/`REQ-041` `fathomdb-mcp` wheel + empty `fathomdb` scripts, active `scope`, bulk mode, `dm-flakey` durability).
- **Create (`Slices 10–50`):** Slice-local `AC32-*` acceptance criteria:
  - **Slice 10 (`AC32-ORIENT-*`, `AC32-GATE-*`):** orientation aids, ADR index split, gate diagnostics, four `ADR-0.8.32-*.md`, green baseline CI.
  - **Slice 20 (`AC32-MANIFEST-01`, `AC32-SCOPE-01..04`):** single-source `TABLE_MANIFEST`, backward compatibility on `golden-0.8.31-schema38.sqlite`, write/anti-spoof scope stamping, full read/graph/evidence/vec0 isolation & nondisclosure, erasure subordination.
  - **Slice 30 (`AC32-BULK-01..02`, `AC32-INS-01..03`):** strict default + strict `actuate` preserved, explicit opt-in bulk mode reporting exact `dangling_edge_endpoints` + `doctor`, Inspector active-`scope` UI subordinate to `D-01` nondisclosure.
  - **Slice 40 (`AC32-MCP-SAFETY-01..09`, `AC32-MCP-PKG-01..03` — Owner Rulings 1 & 3):**
    1. `AC32-MCP-SAFETY-01` (Safe read-only default `allow_writes: false`; `tools/list` gating; deterministic `writes_disabled` refusal with zero mutation).
    2. `AC32-MCP-SAFETY-02` (Explicit per-store `allow_writes: true` + non-empty bound caller `scope` policy; zero cross-scope override/bleed).
    3. `AC32-MCP-SAFETY-03` (Fail-fast runtime startup conflict detection in `Config::load`/`validate`, `Adapter::new`, and `fathomdb-mcp --check-config` BEFORE opening engines or serving `tools/list`).
    4. `AC32-MCP-SAFETY-04` (Mandatory `operation_id` idempotent replay + different-payload `conflict` refusal with zero mutation).
    5. `AC32-MCP-SAFETY-05` (`expected_write_boundary` optimistic precondition + `stale_write_boundary` atomic abort).
    6. `AC32-MCP-SAFETY-06` (Strict-only MCP writes, zero bulk bypass, atomic rollback on invalid batch items, secret-scrubbed errors).
    7. `AC32-MCP-SAFETY-07` (Concurrent admission serialization, deadline/cancellation safety, graceful drain, restart durability of receipts/`operation_id`).
    8. `AC32-MCP-SAFETY-08` (Scoped frozen evidence `freeze_context`/`search_evidence`/`resolve_evidence` + scoped `latest_state` `checkpoint`).
    9. `AC32-MCP-SAFETY-09` (Forbidden escape hatches: never expose `recover`/`restore`/`repair`/`fix`/`rebuild`/`purge`/`erase_source`/`migrate`/bulk writes).
    10. `AC32-MCP-PKG-01..03` (crates.io `publish = true` + `LICENSE`, 5-target GH Releases + `SHA256SUMS`, PyPI `fathomdb-mcp` wheel + `fathomdb[mcp]` extra with `AC-041` empty base scripts).
  - **Slice 50 (`AC32-BACKUP-01..03`, `AC32-DMFLAKEY-01..04`, `AC32-FUZZ-01..03` — Owner Ruling 2):** `fathomdb.logical-export/v1` round-trip/rebuild proof, disposable Linux VM / `dm-flakey` power-loss fault oracle suite, broad coverage-guided fuzzing across query/property/scope/cursor/MCP/export codecs.

### 3.2 Interface Contracts (`dev/interfaces/*`)

1. **`dev/interfaces/mcp.md`:** Promote `status: accepted`, `target_release: 0.8.32`; rename evaluation `"write"`/`"scope": null` to `"allow_writes": false` + per-store `{allow_writes, scope, allowed_kinds, ...}` + fail-fast startup conflict matrix; add scoped read + governed write/checkpoint/evidence tool contracts; delete evaluation/read-only/LoCoMo remnants.
2. **`dev/interfaces/wire.md`:** Flip Schema-38 `scope` & reserved `vec0` slot from inert to active (`SCHEMA_VERSION = 38`); document `TABLE_MANIFEST`; document `WriteReceipt.dangling_edge_endpoints` under strict vs bulk; document `scope` claim in frozen-read/evidence tokens; finalize internal `fathomdb.logical-export/v1` spec.
3. **`dev/interfaces/{rust,rust-sdk,python,typescript}.md` + SDK docstrings:** Document `ScopePolicy` / `scope` & `ScopeViolation`, opt-in bulk write mode, and fix stale `"informational (default FLAG-AND-COUNT)"` docstrings in `write.rs`, `types.py`, and `write.ts`.
4. **`dev/interfaces/cli.md` & `dev/design/recovery.md`:** Document `doctor` handling of `scope` and bulk-imported dangling edges, plus the declared WAL/checkpoint `dm-flakey` durability model.

### 3.3 Engineering Governance, ADRs, Packaging & Public Docs

- **Slice 10 immediate factual fixes:** Correct `PropertyPredicate` operator list (`eq, ne, lt, lte, gt, gte` only — not `in`/`contains`/`exists`/`not_exists`) across 5 `docs/` files (`hybrid-search-filtering.md`, `rust-sdk.md`, `rust-api.md`, `python-api.md`, `typescript-api.md`), add missing `$.action_kind` to `docs/reference/errors.md`, scrub leftover `0.8.30` prose, and clean consumer-specific prose in `fathomdb-data-plane-architecture-v2.md`.
- **Slices 20–50 lockstep public docs:** Update `docs/concepts/`, `docs/guides/`, `docs/reference/`, `docs/inspector/`, create `docs/guides/mcp-server.md` & `docs/reference/mcp.md`, and update `docs/operations/` + `docs/positions/recovery-surface.md`.
- **Slices 60–70 release docs:** Pre-publish `docs/release-notes/0.8.32.md` + regenerated `docs/generated/*`, followed by post-HITL-authorization `RELEASE_SCHEMAS["0.8.32"] = 38` and published truth flip.

---

## 4. Numbered CRUD Proposals (`P3-01`–`P3-21`)

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation | Target Slice / Rationale |
| --- | --- | :---: | :---: | :---: | --- | --- |
| `P3-01` | **[Update]** Sync `dev/needs.md` & `dev/requirements.md` applicability to `0.8.32` / Schema 38 (`REQ-032`, `REQ-036`, `REQ-037`/`REQ-041`); keep `dev/acceptance.md` **LOCKED**. | H | L | S | include | **Slice 60** |
| `P3-02` | **[Create / Update]** Author slice-local `R32-*` refinements, `AC32-*` oracles (incl. `AC32-MCP-SAFETY-01..09`, `AC32-DMFLAKEY-*`, `AC32-FUZZ-*`), and named RED→GREEN tests in feature slice plans. | H | L | M | include | **Slices 10–50** |
| `P3-03` | **[Update — HITL Gate 1 PIN FLAG]** Update `src/conformance/governed-{operation-parity,surface-allowlist}.json` and `scripts/governed-surface-pin.json` ONLY for HITL-authorized `0.8.32` SDK additions (`scope`, bulk write API, or promoted `EP-01`/`EP-05`/`EP-07`); keep `core` (5) & `recovery_denylist` (5) fixed. | M | H | M | include (HITL Gate 1) | **Slices 10 (ruling), 20, 30, 40** |
| `P3-04` | **[Create / Update]** Author `cross-cutting-change-checklist.md`, expand `dev/design/README.md`, update `document-lifecycle.json`, link in `AGENTS.md`, split `ADR-0.6.0-decision-index.md`, and tighten gate diagnostics (`R32-07`). | H | L | S | include | **Slice 10 (`PREWORK-PREP`)** |
| `P3-05` | **[Create]** Draft and ratify four `dev/adr/ADR-0.8.32-*.md` ADRs (Caller `scope`; Read+Write `fathomdb-mcp` Security/Startup Conflicts/Distribution; Opt-In Bulk Endpoint Mode; Declared Power-Loss Durability & `dm-flakey` Fault Oracle). | H | M | M | include | **Slice 10 (`PREWORK-PREP`)** |
| `P3-06` | **[Update / Delete]** Immediate factual doc fixes: correct `PropertyPredicate` ops (6 variants only) across 5 `docs/` files, add `$.action_kind` to `docs/reference/errors.md`, scrub leftover `0.8.30` prose, and clean `fathomdb-data-plane-architecture-v2.md`. | H | L | S | include | **Slice 10 (`PREWORK-PREP`)** |
| `P3-07` | **[Update]** Update `dev/tools/module-boundary-policy.txt` and `dev/tools/module-boundary-gate/src/main.rs` for `search.rs`/`evidence.rs` splits and 10 `*_tests.rs` relocations (`R32-07`). | H | M | M | include | **Slice 20 (`SCOPE-AND-REFACTOR`)** |
| `P3-08` | **[Create / Update / Delete]** Document declarative single-source `TABLE_MANIFEST` in `fathomdb-schema` (`wire.md`, `architecture-v2.md`, `export_manifest.rs`) and delete duplicated manual table-list prose. | H | L | M | include | **Slice 20 (`SCOPE-AND-REFACTOR`)** |
| `P3-09` | **[Create / Update / Delete]** Specify non-breaking Schema-38 `scope` activation across `dev/interfaces/*` and `docs/*`; delete `"inert/unused scope"` claims. | H | M | L | include | **Slice 20 (`SCOPE-AND-REFACTOR`)** |
| `P3-10` | **[Create / Update / Delete]** Specify opt-in bulk / non-strict write mode across `dev/interfaces/*`, SDK docstrings (`write.rs`, `types.py`, `write.ts`), and `docs/reference/*`; delete stale `"always 0"` / `"default flag-and-count"` contradictions. | H | M | M | include | **Slice 30 (`BULK-AND-INSPECTOR`)** |
| `P3-11` | **[Create / Update / Delete]** Update `dev/inspector/*` and `docs/inspector/*` for active Schema-38+ `scope` UI subordinate to `D-01` nondisclosure + `TABLE_MANIFEST` parity. | H | L | M | include | **Slice 30 (`BULK-AND-INSPECTOR`)** |
| `P3-12` | **[Create / Rename / Update / Delete]** Upgrade `dev/interfaces/mcp.md`, `dev/design/mcp-adapter/*`, and `fathomdb-mcp` for `allow_writes: false` default + per-store opt-in + bound `scope`, fail-fast startup conflict checks, governed strict writes/`latest_state` checkpoints, scoped frozen evidence, and `AC32-MCP-SAFETY-01..09`. | H | H | L | include | **Slice 40 (`MCP-RW`)** |
| `P3-13` | **[Create / Update / Delete]** Package `fathomdb-mcp` per Owner Ruling 3 (`LICENSE`, `README.md`, `src/python-mcp/`, `publish = true`, `fathomdb[mcp]` extra with `AC-041` empty base scripts, release scripts & workflows). | H | M | M | include | **Slices 10 (prep) & 40 (ship)** |
| `P3-14` | **[Create / Update / Delete]** Create `docs/guides/mcp-server.md` & `docs/reference/mcp.md` and update `docs/{use-cases,install,reference,operations,positions}/*` for production read+write `fathomdb-mcp`. | H | L | M | include | **Slice 40 (`MCP-RW`)** |
| `P3-15` | **[Update / Delete]** Reconcile any `0.8.32`-accepted `dev/design/mcp-adapter/engine-proposals/EP-*.md` (`EP-01`, `EP-05`, `EP-07`, `EP-10`) into interfaces and reference docs. | M | M | M | include | **Slice 40 (`MCP-RW`)** |
| `P3-16` | **[Create / Update]** Finalize internal `fathomdb.logical-export/v1` wire/format contract and backup/restore/rebuild docs (`R32-04`) keeping CLI `export`/`import` non-public until `0.8.34`. | H | M | M | include | **Slice 50 (`DURABILITY-AND-FUZZ`)** |
| `P3-17` | **[Create / Update]** Document declared power-loss durability model, disposable Linux VM / `dm-flakey` fault-oracle spec (Owner Ruling 2), and fuzzing harness specs (`R32-05`). | H | M | M | include | **Slice 50 (`DURABILITY-AND-FUZZ`)** |
| `P3-18` | **[Create / Update]** Pre-publish `docs/release-notes/0.8.32.md`, `generate-public-docs.py` (`SOURCE_URL` + `src/python-mcp`), regenerated `docs/generated/*`, and `0.8.32` target wording with `published: null` intact. | H | L | M | include | **Slice 60 (`QUALIFICATION`)** |
| `P3-19` | **[Update]** Post-HITL publication truth flip (`RELEASE_SCHEMAS["0.8.32"] = 38`, published notices, `CHANGELOG.md`, live consumer receipts). | H | L | S | include | **Slice 70 (`PUBLICATION`)** |
| `P3-20` | **[Archive / Delete]** Bulk `dev/` archival, major OSS docs IA overhaul, `dev/hygeine-roadmap.md` rename, deprecated Steward harness removal (`P2-09`/`P2-10`). | H | M | L | postpone | **`0.8.33`** |
| `P3-21` | **[Create / Update]** Public `fathomdb export`/`import` CLI, personal-agent kits, public benchmarks, CUDA 13 delivery. | H | L | L | postpone | **`0.8.34`–`0.8.35+`** |
