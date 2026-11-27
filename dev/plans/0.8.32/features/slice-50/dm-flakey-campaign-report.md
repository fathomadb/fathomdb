# Slice 50 (`WP-50D`) QEMU/KVM `dm-flakey` Power-Loss Campaign Report (`AC32-DMFLAKEY-01`–`04`)

**Date:** 2026-11-26  
**Generated (UTC):** `2026-11-27T02:16:08.067410+00:00`  
**Release / Slice:** `0.8.32` / Slice 50 (`WP-50D`)  
**Authoritative Contracts:** `dev/plans/0.8.32/features/slice-50/plan.md` (`AC32-DMFLAKEY-01`–`04`), `dev/adr/ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md` (`#101`, §§1–3)

## 1. Environment, Isolation & Methodology (`AC32-DMFLAKEY-01`)

- **Host environment (zero host `sudo`, zero host mutation):** Linux `7.0.0-43-generic`, `QEMU emulator version 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18)` executing user-mode `qemu-system-x86_64` over `/dev/kvm`.
- **Guest image (staged outside git under `/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm/`):**
  - Base image: Ubuntu `24.04` Noble (`noble-server-cloudimg-amd64.img`), SHA-256 verified against upstream `SHA256SUMS`: `04e8f2461a16a44e78a370284471f29616cdd9baf1893df25807b71f985b6c94`.
  - Guest kernel: `Linux 6.8.0-147-generic` (`x86_64`), booted via `xorriso` NoCloud `cidata.iso` onto an ephemeral copy-on-write `qcow2` overlay per case (`bootindex=1` on `root0`).
- **Guest binaries (zero shipping-crate mutation):**
  - Fault workload & recovery verifier: `src/rust/crates/fathomdb-tc5-benchmark/src/bin/fathomdb-dmflakey-harness.rs` (built inside `eval-only` crate `fathomdb-tc5-benchmark`, linked against `GLIBC_2.34 <= 2.39` for Noble compatibility).
  - Real shipped CLI verifier: `target/release/fathomdb` (`fathomdb-cli`) running `fathomdb doctor check-integrity --full` and `fathomdb doctor data-plane-integrity` inside the rebooted guest after each power cut.
- **Faulted data path & power-off discipline:**
  - Dedicated raw data disk (`data.raw`) attached over `virtio-blk-pci` with `cache=none` (bypassing host page cache so QEMU `quit` matches sudden power loss on an `ext4` filesystem mounted over `dm-flakey`).
  - Sudden power termination executed exclusively via QEMU QMP `quit` (falling back to `SIGKILL`, never graceful ACPI `system_powerdown`).
  - Two-phase external host oracle over dedicated `virtio-serial` (`org.fathomdb.oracle`): the guest emits and flushes `INFLIGHT <seq> <op> <id>` **before** invoking `Engine`, and emits and flushes `ACK <seq> <cursor> <op> <id>` (or `ERR <seq> <op> <id> <kind>`) **after** `Engine` returns — strictly before issuing the next call or triggering power loss.
- **Reproducible invocation:**

```bash
bash scripts/run-dmflakey-campaign.sh \
  --vm-dir /mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm \
  --scenarios all
```

## 2. Live Campaign Results Matrix (`11 / 11` Cases Passed — `all_ok = true`)

Every case rebooted into a fresh guest instance, ran `e2fsck -p` (`fsck_rc = 0`), confirmed a stale `.lock` file was present before `Engine::open` (`stale_lock_present_before_open = true`, enforced fail-closed on `erasure_lock_fault` in both `verify_database` and `scripts/run-dmflakey-campaign.sh`), ran `PRAGMA integrity_check` (`ok`), verified recovered canonical state against the external `INFLIGHT`/`ACK`/`ERR` oracle log, confirmed post-recovery writes succeeded (`post_recovery_write_ok = true`), and passed both `fathomdb doctor check-integrity --full` (`logical`, `physical`, `semantic` = `clean`, `0` findings) and `fathomdb doctor data-plane-integrity` (`clean`, `0` findings).

Note on `Pending Erasures` column: `Engine::pending_erasures()` (`operator.rs:247-280`) returns the union of `dependency_closure::pending_physical_closures` and `physical_erasure::diagnostics`. Un-checkpointed hard power cuts report `1` diagnostic entry cleared by `recover_complete_erasures`; mid-stream `erasure_lock_fault` cuts before trailing `checkpoint` report `2` entries (`pending_erasures_before_recovery = 2 -> 0`).

| # | Case ID | `SyncMode` | Scenario / Cut Point | Fault Mechanism | `matched` / `acked` / `inflight` | Active / Deleted Nodes | WAL Size (Bytes) | Pending Erasures (Before -> After) | `fsck` / `integrity` / `doctor` / Post-Write |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `normal_active_commit_poweroff` | `normal` | Active commit (mid-stream cut at op `33 / 60`) | QMP `quit` | `18 / 33 / 33` | `36 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 2 | `normal_wal_accum_drop_writes` | `normal` | Un-checkpointed WAL accumulation | `dm-flakey drop_writes` + QMP `quit` | `14 / 40 / 40` | `28 / 0` | `4,305,432` | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 3 | `normal_checkpoint_erasure_drop_writes` | `normal` | `wal_checkpoint(TRUNCATE)` + erasure | `dm-flakey drop_writes` + QMP `quit` | `16 / 56 / 56` | `32 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 4 | `normal_reopen_replay_double_cut` | `normal` | Reopen / WAL-replay double cut | QMP `quit` (P1) + `dm-flakey drop_writes` + QMP `quit` (P2) | `0 / 24 / 24` (P2; P1 kept `14 / 24`) | `28 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 5 | `normal_erasure_lock_fault` | `normal` | Mid-stream `purge` / `erase_source` + stale `.lock` (before checkpoint) | QMP `quit` | `60 / 60 / 60` | `18 / 9` | n/a | `2 -> 0` | `0` / `ok` / `0` / `ok` |
| 6 | `full_active_commit_poweroff` | `full` | Active commit (mid-stream cut at op `33 / 60`) | `up_interval` + QMP `quit` | `33 / 33 / 33` | `66 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 7 | `full_wal_accum_poweroff` | `full` | Un-checkpointed WAL accumulation (mid-stream cut at op `38 / 40`) | `up_interval` + QMP `quit` | `38 / 38 / 38` | `76 / 0` | `4,305,432` | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 8 | `full_checkpoint_erasure_poweroff` | `full` | `wal_checkpoint(TRUNCATE)` + erasure (live `INFLIGHT 52` cut) | `up_interval` + QMP `quit` | `51 / 51 / 52` (`INFLIGHT 52` rolled back atomically) | `26 / 9` | n/a | `0 -> 0` | `0` / `ok` / `0` / `ok` |
| 9 | `full_reopen_replay_double_cut` | `full` | Reopen / WAL-replay double cut | `up_interval` + double QMP `quit` (P1 + P2) | `24 / 24 / 24` (P2; P1 kept `24 / 24`) | `96 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 10 | `full_erasure_lock_fault` | `full` | Mid-stream `purge` / `erase_source` + stale `.lock` (live `INFLIGHT 58` cut) | `up_interval` + QMP `quit` | `58 / 57 / 58` (`INFLIGHT 58` committed atomically) | `20 / 10` | n/a | `2 -> 0` | `0` / `ok` / `0` / `ok` |
| 11 | `full_error_writes_eio` | `full` | Post-`ACK` `EIO` (`error_writes`) | `dm-flakey error_writes` + QMP `quit` | `16 / 12 / 16` (`1..12` `ACK`; `13..16` `ERR`) | `24 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |

## 3. Empirical Validation of `ADR #101` Contracts (`AC32-DMFLAKEY-02`–`04`)

1. **`SyncMode::Normal` proves honest checkpoint-bounded atomic prefix recovery (`AC32-DMFLAKEY-02`):**
   - Under mid-stream QMP `quit` (`18 / 33` on `active_commit`) and `dm-flakey drop_writes` with up to `4,305,432` bytes of un-checkpointed WAL (`14 / 40` on `wal_accum`, `16 / 56` on `checkpoint_erasure`, and Phase-2 `0 / 24` under `drop_writes` while preserving Phase-1 `14 / 24` = `28` active nodes on `reopen_replay`), `SyncMode::Normal` rolled back un-checkpointed suffixes atomically.
   - Every recovered store matched a strictly contiguous prefix `0..=k` of the external oracle with **zero partial commits**, **zero out-of-order gaps**, `PRAGMA integrity_check == ok`, and **`0`** findings from both `doctor check-integrity --full` and `doctor data-plane-integrity`.
2. **`SyncMode::Full` proves `0` lost acknowledged commits and atomic `INFLIGHT` outcomes under `FLUSH`/`FUA`-honoring power loss (`AC32-DMFLAKEY-03`):**
   - Across all 4 cut points — mid-stream `active_commit` (`33 / 33`), `4,305,432`-byte WAL `wal_accum` (`38 / 38`), `checkpoint_erasure` (`51 / 51` acked), double QMP `quit` `reopen_replay` (`24 / 24` P1 + `24 / 24` P2 = `96` active nodes), and mid-stream `erasure_lock_fault` (`57 / 57` acked) — `SyncMode::Full` preserved **100% of externally `ACK`-recorded commits** (`0` lost acknowledged commits).
   - Two live mid-`INFLIGHT` cuts empirically exercised **both** permitted outcomes of a single un-acknowledged `INFLIGHT` operation:
     - `full_checkpoint_erasure_poweroff` cut during `INFLIGHT 52 purge dmf-000020-b`: recovered cleanly at `matched_prefix_index = 51` (atomic rollback of `INFLIGHT 52`, `ACK 51` preserved).
     - `full_erasure_lock_fault` cut during `INFLIGHT 58 purge dmf-000017-b`: recovered cleanly at `matched_prefix_index = 58` (atomic commit of `INFLIGHT 58` after WAL `fsync` before `ACK` could flush over virtio-serial).
   - Under `full_error_writes_eio`, after `12` `ACK`-recorded commits (`24` nodes), switching `dm-flakey` to `error_writes` (`EIO`) caused all `4` subsequent write attempts (`13..=16`) to fail closed with `StorageError` (`ERROR_WRITES_OBSERVED 4 0`); after QMP `quit` and reboot, all `12` previously acknowledged commits survived intact (`24` active nodes, `0` failed writes committed, `0` `doctor` findings).
3. **Physical erasure, stale `.lock` recovery & post-recovery health (`AC32-DMFLAKEY-04`):**
   - Across all `11 / 11` cases, stale sidecar `.lock` files were present prior to `Engine::open` (`stale_lock_present_before_open = true`, fail-closed enforced on `erasure_lock_fault`) and reclaimed cleanly without wedging open.
   - Both `normal_erasure_lock_fault` (`18` active / `9` deleted) and `full_erasure_lock_fault` (`20` active / `10` deleted) cut mid-stream during `purge`/`erase_source` prior to trailing checkpoint, leaving `pending_erasures_before_recovery = 2`, which `recover_complete_erasures` resumed and drained cleanly to `pending_erasures_after_recovery = 0` with zero resurrected rows.
   - Every case completed a post-recovery write/read round-trip (`post_recovery_write_ok = true`).

## 4. Harness & Script Verification

- `src/rust/crates/fathomdb-tc5-benchmark/src/bin/fathomdb-dmflakey-harness.rs`: `8 / 8` unit tests pass (`cargo test -p fathomdb-tc5-benchmark --bin fathomdb-dmflakey-harness`), covering two-phase oracle parsing, `SyncMode::Normal` prefix rollback acceptance, `SyncMode::Full` lost-ACK rejection, out-of-order gap rejection, `EIO` error-write oracle matching, fail-closed `erasure_lock_fault` stale-lock requirement, and mid-stream `INFLIGHT` `purge`/`erase_source` verification.
- `scripts/run-dmflakey-campaign.sh`: passes all 3 shrink-only legs of `scripts/agent-lint-shell.sh` (`--severity=style`, `SC2312`, `shell-early-consumer`), `shellcheck --include=SC2312`, and `bash scripts/run-dmflakey-campaign.sh --self-test` (`SELF_TEST_OK cases=11`).
- Zero host `sudo`, zero host system changes, and zero VM artifacts tracked in git (all staging artifacts remain outside the repository under `/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm/results/campaign-summary.json`).
