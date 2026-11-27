---
title: Slice 50 Status — Declared WAL SyncMode, Logical Export/Rebuild Proof, Isolated Fuzzing, and QEMU/KVM dm-flakey Power-Loss Campaign
status: LANDED
target_release: 0.8.32
slice: 50
date: 2026-11-26
---

# Slice 50 Status — `DURABILITY-AND-FUZZ`

## Outcome

**LANDED locally on `dev/0.8.32`** (`/mnt/data/worktrees/release-0.8.32`). Delivers `R32-04` and `R32-05` (`ADR-0.8.32 #101`, `Gate1-D2`, `Gate1-D8`) across an independent `pro`-approved Slice 50 plan, four sequentially executed TDD (`RED -> GREEN`) work packages, a `FIX-1` adversarial-review remediation pass (including a live 11/11 QEMU/KVM `dm-flakey` campaign re-run), a 19-file 4-tier documentation refresh, and full unconfined workspace verification:

1. **Pre-Code Plan (`1082fb4d9`):**
   - Approved [plan.md](./plan.md) (`AC32-SYNC-01..03`, `AC32-BACKUP-REBUILD-01..04`, `AC32-FUZZ-01..03`, `AC32-DMFLAKEY-01..04`) after independent `pro` adversarial review (`VERDICT: APPROVE`, `0` P1 / `0` P2).
2. **`WP-50A` (`87d7c4c77`) — Declared WAL `SyncMode::{Normal, Full}` End-to-End Parity (`AC32-SYNC-01..03`):**
   - Implemented `SyncMode::{Normal, Full}` (`sync_mode: Option<SyncMode>`, default `None` resolving to `SyncMode::Normal`) in `fathomdb-engine/src/runtime_configuration.rs`, applying `PRAGMA synchronous = NORMAL` (`1`) or `FULL` (`2`) across primary writer (`open.rs:open_locked`) and all projection-worker/runtime/dispatcher/probe connections (`connection_runtime.rs:open_runtime_connection`), verified via `slice50_sync_mode.rs`.
   - Threaded `SyncMode` / `sync_mode` / `syncMode` across `fathomdb`, `fathomdb-sdk` (`OpenOptions::with_sync_mode`), Python (`fathomdb-py`, `config.py`, `_sdk_open.py`, `python_surface_additions.json`, `eval/earp/knobs.py` `HELD_CONSTANT`, `test_config_resolver.py`, `test_slice90_engine_config.py`, owned `.venv` wheel rebuilt), TypeScript (`fathomdb-napi`, `core.ts`, `binding.ts`, `690/690` `npm test`), and `fathomdb-mcp` (`SC-03`/`SC-05`/`SC-06`/`SC-10`, `HostedStorePolicy`). `scripts/governed-surface-pin.json` (`58`/`97`) and `python-test-hooks-v1.json` remain strictly invariant.
3. **`WP-50B` (`d8f2143c6`, `FIX-1` `979d61719`) — Internal `fathomdb.logical-export/v1`, `R32-04` Backup/Restore/Rebuild Proof & Lifecycle Leak Checks (`AC32-BACKUP-REBUILD-01..04`):**
   - Implemented gated `fathomdb-engine/src/logical_export.rs` (`#[cfg(any(test, feature = "test-hooks", feature = "operator"))]`, registered in `module-boundary-policy.txt`, zero default SDK exposure) covering all `36` `TABLE_MANIFEST` tables, preserving exact `write_cursor` values and orphaned `non_canonical` `_fathomdb_projection_terminal` cursors (multi-op `AdminSchema`, overwritten `LatestState`, retention-pruned `AppendOnlyLog`), enforcing all 4 pending-erasure checks atomically inside the snapshot transaction, never emitting `embedding_bin`, `mean_vec`, or `_fathomdb_embed_probe`, and deriving `mean_vec` on restore iff vector count `>= 256` (`NULL` below `256`).
   - Fixed 2 latent `data_plane_integrity.rs` defects (`utf8_bytes` hash body check and non-closed committable terminal rows across rebuilds; `68/68` `slice55_data_plane_integrity` pass) and updated `slice65_managed_connection_owners.py` (`test_windows_wal_attribution_ci_job.sh` `345/345` pass).
   - Verified end-to-end installed-artifact CLI backup/restore/rebuild workflow in `fathomdb-cli/tests/slice50_backup_restore_rebuild.rs` (`doctor safe-export` -> SHA-256 copy -> separate `--rebuild-projections` / `--rebuild-vec0` -> dense embedder reopen + `drain()` -> `doctor check-integrity --full` + `data-plane-integrity`, `0` findings + full semantic parity) and repeated `open`/`write`/`search`/`close`/`drop` FD, thread (`/proc/self/task`), and `.lock` leak checks across Engine (`None` + dense embedder) and `fathomdb-mcp` (`mcp_lifecycle_leaks.rs`).
4. **`WP-50C` (`745344329`) — Standalone `fuzz/` Workspace, 7 Targets & Minimized Stable Regressions (`AC32-FUZZ-01..03`):**
   - Added isolated `fuzz/` workspace (`exclude = ["fuzz"]`, `dev-tooling` SBOM tier, zero `[patch]`/`[replace]`/`git =` overrides, zero fuzz deps in shipping crates verified via `cargo tree --locked`, root toolchain unchanged at `1.95.0`).
   - Executed `2,185,400` total runs (`1,248,923` `ASan` + `936,477` `UBSan`/GCC13+`-Zub-checks=on`) across all 7 structured targets with **0 FathomDB defects** and `23.18%` merged `llvm-cov` line coverage (documenting the upstream `sqlite-vec = "=0.1.9"` 280 B `vec0_init` `LSan` allocation honestly in [fuzz-campaign-report.md](./fuzz-campaign-report.md)), plus deterministic stable `#[test]` regressions (`fathomdb-engine/tests/slice50_fuzz_regressions.rs` `7/7`, `fathomdb-mcp/tests/slice50_fuzz_regressions.rs` `2/2`).
5. **`WP-50D` (`dd6e40c8e`, `FIX-1` `979d61719`) — Disposable QEMU/KVM `dm-flakey` Power-Loss Campaign (`AC32-DMFLAKEY-01..04`):**
   - Built `eval-only` `fathomdb-dmflakey-harness` (`8/8` unit tests) and shellcheck-clean `scripts/run-dmflakey-campaign.sh` (`--severity=style`, `SC2312`, `shell-early-consumer`, `--self-test` `11` cases; zero host `sudo`, zero host mutation, `cache=none` virtio-blk, QMP `quit` hard power-off, two-phase `INFLIGHT`/`ACK`/`ERR` virtio-serial oracle).
   - Passed **11 / 11 live QEMU/KVM `dm-flakey` cases** (`all_ok = true`, [dm-flakey-campaign-report.md](./dm-flakey-campaign-report.md)), proving `SyncMode::Normal` atomic checkpoint-bounded prefix rollback, `SyncMode::Full` `0` lost acknowledged commits across all 4 cut points (including `4.3 MiB` WAL, double-cut reopen/replay, live mid-`INFLIGHT` atomic rollback `51/51/52` & atomic commit `58/57/58`, mid-erasure cuts with stale `.lock` `pending_erasures 2 -> 0`, and fail-closed `EIO` `error_writes`), `PRAGMA integrity_check == ok`, and `0` `doctor` findings on every case.
6. **4-Tier Documentation Refresh (`ba77948e3`):**
   - Updated 19 files across Project (`CHANGELOG.md`), Contributor (`docs/contributing/index.md`), Maintainer/Interface/Design (`dev/interfaces/{typescript,rust-sdk}.md`, `dev/design/{engine,recovery,cross-cutting-change-checklist,current-owner-authority.json,mcp-adapter/*}`, `dev/notes/fathomdb-quality-testing-roadmap.md`), and User-Facing (`docs/operations/index.md`, `docs/compatibility/index.md`, `docs/reference/{config,mcp}.md`, `docs/guides/mcp-server.md`, `README.md`, crate READMEs) tiers, with all 9 doc/truth/lifecycle/lint/MkDocs validators exiting `0`.

## Verification & Adversarial Review Receipts

- **Full unconfined `./scripts/agent-verify.sh` on `ba77948e3`:** **EXIT `0`** — `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)` (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` PASS); `agent-test.sh: 246/249 suites passed (skipped=3 excluded=0)`.
- **`mkdocs build --strict -d /tmp/fathomdb-slice50-mkdocs`:** **EXIT `0`** (`0` warnings, built in `0.61s`).
- **Independent `pro` adversarial reviews (both gates `VERDICT: APPROVE`, `0` P1 / `0` P2):**
  - Slice 50 Plan (`1082fb4d9`): `VERDICT: APPROVE`
  - Slice 50 Code & Campaign Audit (`1082fb4d9..979d61719` after `FIX-1`): `VERDICT: APPROVE` (`14/14` `AC32-*` verified)

## Commits on `dev/0.8.32`

- `1082fb4d9` — `docs(0.8.32): approve Slice 50 DURABILITY-AND-FUZZ plan`
- `87d7c4c77` — `feat(0.8.32): WP-50A wire SyncMode::{Normal, Full} across Rust, Python, TS, and MCP`
- `d8f2143c6` — `feat(0.8.32): WP-50B add logical-export/v1, backup/restore/rebuild proofs, and leak checks`
- `745344329` — `feat(0.8.32): WP-50C add isolated fuzz/ workspace, 7 targets, campaign report, and stable regressions`
- `dd6e40c8e` — `feat(0.8.32): Slice 50 WP-50D QEMU/KVM dm-flakey power-loss harness and 11/11 campaign report`
- `979d61719` — `fix(0.8.32): Slice 50 FIX-1 remediate review findings and re-run 11/11 dm-flakey campaign`
- `ba77948e3` — `docs(0.8.32): Slice 50 4-tier documentation sync (SyncMode, backup/rebuild, fuzzing, dm-flakey)`
