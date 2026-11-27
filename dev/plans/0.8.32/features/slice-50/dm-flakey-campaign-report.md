# Slice 50 (`WP-50D`) QEMU/KVM `dm-flakey` Power-Loss Campaign Report (`AC32-DMFLAKEY-01`–`04`)

**Date:** 2026-11-26  
**Generated (UTC):** `2026-11-27T01:28:54.697486+00:00`  
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
  --stage-dir /mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm \
  --cases all
```

## 2. Live Campaign Results Matrix (`11 / 11` Cases Passed — `all_ok = true`)

Every case rebooted into a fresh guest instance, ran `e2fsck -p` (`fsck_rc = 0`), confirmed a stale `.lock` file was present before `Engine::open` (`stale_lock_present_before_open = true`) and did not wedge recovery, ran `PRAGMA integrity_check` (`ok`), verified the recovered canonical state against the external `INFLIGHT`/`ACK` oracle log, confirmed post-recovery writes/erasures succeeded (`post_recovery_write_ok = true`), and passed both `fathomdb doctor check-integrity --full` (`logical`, `physical`, `semantic` = `clean`, `0` findings) and `fathomdb doctor data-plane-integrity` (`clean`, `0` findings).

| # | Case ID | `SyncMode` | Scenario / Cut Point | Fault Mechanism | `matched` / `acked` / `inflight` | Active / Deleted Nodes | WAL Size (Bytes) | Pending Erasures (Before -> After) | `fsck` / `integrity` / `doctor` / Post-Write |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | `normal_active_commit_poweroff` | `normal` | Active commit | QMP `quit` | `46 / 60 / 60` | `92 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 2 | `normal_wal_accum_drop_writes` | `normal` | Un-checkpointed WAL accumulation | `dm-flakey drop_writes` + QMP `quit` | `14 / 40 / 40` | `28 / 0` | `4,305,432` | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 3 | `normal_checkpoint_erasure_drop_writes` | `normal` | `wal_checkpoint(TRUNCATE)` + erasure | `dm-flakey drop_writes` + QMP `quit` | `16 / 56 / 56` | `32 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 4 | `normal_reopen_replay_double_cut` | `normal` | Reopen / WAL-replay double cut | QMP `quit` (P1) + `drop_writes` + QMP `quit` (P2) | `0 / 24 / 24` (P2; P1 kept `14 / 24`) | `28 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 5 | `normal_erasure_lock_fault` | `normal` | `purge` / `erase_source` + stale `.lock` | QMP `quit` | `53 / 53 / 53` | `24 / 8` | n/a | `0 -> 0` | `0` / `ok` / `0` / `ok` |
| 6 | `full_active_commit_poweroff` | `full` | Active commit | `up_interval` + QMP `quit` | `60 / 60 / 60` | `120 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 7 | `full_wal_accum_poweroff` | `full` | Un-checkpointed WAL accumulation | `up_interval` + QMP `quit` | `40 / 40 / 40` | `80 / 0` | `4,305,432` | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 8 | `full_checkpoint_erasure_poweroff` | `full` | `wal_checkpoint(TRUNCATE)` + erasure | `up_interval` + QMP `quit` | `55 / 55 / 55` | `28 / 8` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 9 | `full_reopen_replay_double_cut` | `full` | Reopen / WAL-replay double cut | `up_interval` + double QMP `quit` (P1 + P2) | `24 / 24 / 24` (P2; P1 kept `24 / 24`) | `96 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |
| 10 | `full_erasure_lock_fault` | `full` | `purge` / `erase_source` + stale `.lock` | `up_interval` + QMP `quit` | `53 / 53 / 53` | `24 / 8` | n/a | `0 -> 0` | `0` / `ok` / `0` / `ok` |
| 11 | `full_error_writes_eio` | `full` | Post-`ACK` `EIO` (`error_writes`) | `dm-flakey error_writes` + QMP `quit` | `16 / 12 / 16` (`1..12` `ACK`; `13..16` `ERR`) | `24 / 0` | n/a | `1 -> 0` | `0` / `ok` / `0` / `ok` |

## 3. Empirical Validation of `ADR #101` Contracts (`AC32-DMFLAKEY-02`–`04`)

1. **`SyncMode::Normal` proves honest checkpoint-bounded atomic prefix recovery (`AC32-DMFLAKEY-02`):**
   - Under hard QMP `quit` and `dm-flakey drop_writes` with up to `4,305,432` bytes of un-checkpointed WAL, `SyncMode::Normal` genuinely rolled back un-checkpointed suffixes (`46 / 60`, `14 / 40`, `16 / 56`, and Phase 2 `0 / 24` while preserving the intact Phase 1 prefix of `28` nodes).
   - Every recovered store matched a strictly contiguous prefix `0..=k` of the external oracle with **zero partial commits**, **zero out-of-order gaps**, `PRAGMA integrity_check == ok`, and **`0`** findings from both `doctor check-integrity --full` and `doctor data-plane-integrity`.
2. **`SyncMode::Full` proves `0` lost acknowledged commits under `FLUSH`/`FUA`-honoring power loss (`AC32-DMFLAKEY-03`):**
   - Under identical hard QMP `quit` power cuts across all 4 cut points — including `4,305,432` bytes of un-checkpointed WAL (`40 / 40`), active commit (`60 / 60`), `wal_checkpoint(TRUNCATE)` + erasure (`55 / 55`), and double hard power loss across reopen/WAL replay (`24 / 24` in Phase 1 + `24 / 24` in Phase 2 = `96` active nodes) — `SyncMode::Full` preserved **100% of externally `ACK`-recorded commits** (`0` lost acknowledged commits).
   - Under `full_error_writes_eio`, after `12` `ACK`-recorded commits (`24` nodes), switching `dm-flakey` to `error_writes` (`EIO`) caused all `4` subsequent write attempts (`13..=16`) to fail closed with `StorageError` (`ERROR_WRITES_OBSERVED 4 0`); after QMP `quit` and reboot, all `12` previously acknowledged commits survived intact (`24` active nodes, `0` failed writes committed, `0` `doctor` findings).
3. **Physical erasure, stale `.lock` recovery & post-recovery health (`AC32-DMFLAKEY-04`):**
   - Across all `11 / 11` cases, stale sidecar `.lock` files were confirmed present prior to `Engine::open` (`stale_lock_present_before_open = true`) and reclaimed cleanly without wedging open.
   - Across `normal_erasure_lock_fault`, `full_erasure_lock_fault`, and `full_checkpoint_erasure_poweroff`, erased/purged rows remained absent (`8` deleted nodes, zero resurrections), and in all `9` cases where pending physical erasures were staged before power loss (`pending_erasures_before_recovery = 1`), reopen resumed and drained physical erasure cleanly to `pending_erasures_after_recovery = 0`.
   - Every case completed a post-recovery write/read round-trip (`post_recovery_write_ok = true`).

## 4. Harness & Script Verification

- `src/rust/crates/fathomdb-tc5-benchmark/src/bin/fathomdb-dmflakey-harness.rs`: `6 / 6` unit tests pass (`cargo test -p fathomdb-tc5-benchmark --bin fathomdb-dmflakey-harness`), covering two-phase oracle parsing, `SyncMode::Normal` prefix rollback acceptance, `SyncMode::Full` lost-ACK rejection, out-of-order gap rejection, and `EIO` error-write oracle matching.
- `scripts/run-dmflakey-campaign.sh`: passes all 3 shrink-only legs of `scripts/agent-lint-shell.sh` (`--severity=style`, `SC2312`, `shell-early-consumer`).
- Zero host `sudo`, zero host system changes, and zero VM artifacts tracked in git (all staging artifacts remain outside the repository under `/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm/results/campaign-summary.json`).
