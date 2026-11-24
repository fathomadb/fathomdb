---
title: ADR-0.8.32 — Declared WAL durability sync mode, QEMU/KVM dm-flakey power-loss oracle, and isolated fuzzing workspace
date: 2026-11-24
target_release: 0.8.32
desc: Amends ADR-0.6.0-durability-fsync-policy with an honest WAL SyncMode::Normal checkpoint-bounded durability contract, opt-in EngineConfig::sync_mode = SyncMode::Full (PRAGMA synchronous = FULL) justified by WP-10D 100k checkpoint measurements, explicit AC32-DMFLAKEY-01..04 power-loss campaign (disposable /dev/kvm Ubuntu 24.04 + dm-flakey + external acknowledged-write oracle), and AC32-FUZZ-01..03 standalone fuzz/ workspace policy.
blast_radius: src/rust/crates/fathomdb-engine/src/{open,runtime_configuration,lib}.rs; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-mcp/**; Cargo.toml; fuzz/**; scripts/sbom-survey/tiers.toml; dev/adr/ADR-0.6.0-durability-fsync-policy.md; docs/operations/index.md; docs/compatibility/index.md
status: accepted
---

# ADR-0.8.32: Declared WAL durability `SyncMode`, QEMU/KVM `dm-flakey` power-loss oracle, and isolated fuzzing workspace

## Context

`ADR-0.6.0-durability-fsync-policy.md` (`#7b`) standardized `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = NORMAL` on the engine writer connection (`src/rust/crates/fathomdb-engine/src/open.rs`). However, `ADR-0.6.0-durability-fsync-policy.md` predated real block-level power-loss fault injection (`0.8.31` `quality-backlog.md`, `R32-05`) and described `synchronous = NORMAL` via a time-based heuristic ("up to 100ms of final-commit loss on power-cut; zero commit loss on OS-crash").

Under actual SQLite WAL semantics:

1. `PRAGMA synchronous = NORMAL` issues `fsync` only across WAL checkpoint boundaries, **not** on every individual WAL transaction commit. After an OS crash (where kernel page cache was flushed by checkpointing or did not drop uncommitted frames) it usually retains committed frames, **but across a real power failure or volatile block-cache drop** (`dm-flakey` `drop_writes` plus hard VM power-off), `NORMAL` guarantees **zero database corruption and atomic prefix recovery**, not zero loss of un-checkpointed commits acknowledged before power loss.
2. Meanwhile, unconditionally switching all FathomDB workloads to `PRAGMA synchronous = FULL` would severely regress unbatched single-row write latency. In Slice 10 `WP-10D` (`dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md`), paired `N = 100` measurements across three 100k-node / 150k-edge corpora demonstrated that **`SyncMode::Full` adds `+431.7%` to `+2007.8%` (`5.3x`–`21.1x`) p50 latency on single-row checkpoints** (`0.0396–0.1057 ms` under `NORMAL` vs. `0.3560–0.8347 ms` p50 and up to `4.1005 ms` p95 under `FULL`), whereas `200`-row batches (`82.3–159.6 ms`) amortize the single WAL `fsync` into SQLite/FTS/property CPU work.

Separately, broad coverage-guided fuzzing (`quality-backlog.md`, `R32-05`) requires `cargo-fuzz` / `libfuzzer-sys` / `arbitrary` toolchain dependencies that must never enter shipping crate dependency graphs (`Cargo.lock` / `.crate` / wheels).

## Decision (Owner Ruling 2 & HITL Gate 1 Decision 2 Option 1, 2026-11-24)

Amends `ADR-0.6.0-durability-fsync-policy.md` (`#7b`) with the following declared durability contract, fault-injection oracle, and fuzzing workspace rules:

### 1. Honest WAL Durability Contract (`SyncMode::Normal` Default + Opt-In `SyncMode::Full`)

Add `SyncMode` (`Normal` [default] and `Full`) on `EngineConfig` (`src/rust/crates/fathomdb-engine/src/runtime_configuration.rs`, mirrored in `fathomdb-sdk`, Python/TS bindings if surfaced via config, and `fathomdb-mcp` write-enabled store configuration):

- **`SyncMode::Normal` (library default, `PRAGMA synchronous = NORMAL`):**
  - Preserves `0.8.31` write speed (`32.8 ms` / `15.5 ms` 200-row batches; `~0.04–0.11 ms` unbatched single-row checkpoints).
  - **Contract:** Guarantees **zero database corruption** (`PRAGMA integrity_check == ok` and clean `fathomdb doctor check-integrity --full`), intact B-tree/FTS5/`vec0`/trigger invariants, and **atomic prefix recovery** to the last flushed/checkpointed WAL frame upon unclean shutdown or power loss. Un-checkpointed commits buffered in volatile page/disk caches at power loss may roll back atomically as a contiguous suffix, never partially or out of order.
- **`SyncMode::Full` (opt-in, `PRAGMA synchronous = FULL`):**
  - Issues per-commit WAL `fsync` (`SQLITE_SYNC_FULL`).
  - **Contract:** Guarantees **strict zero loss of every client-acknowledged commit** (alongside zero corruption) across both OS crash and storage power loss when backed by an `fsync`-honoring filesystem and block device, at the measured single-row checkpoint cost (`5.3x`–`21.1x` on unbatched checkpoints; amortized on 200-row batches).
- **`fathomdb-mcp` configuration support:** Write-enabled `fathomdb-mcp` store configurations support selecting `sync_mode` (`"normal"` default or `"full"`) on `EngineConfig` so durability-sensitive deployments can opt into strict power-loss persistence per store.

### 2. Declared Filesystem, Storage Stack, and Non-Recoverable Fault Boundaries

- **Supported host storage assumptions:** Local POSIX filesystem (`ext4` with barriers/`data=ordered` journaling) over a block device that honors write barriers and cache flushes (`FUA` / `FLUSH`). SQLite runs in `WAL` mode with `PRAGMA secure_delete = ON` (`open.rs`) and mandatory sidecar `.lock` cross-process exclusion (`ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md`).
- **Explicit exclusions:** Network filesystems lacking reliable `flock`/`fsync` semantics, write-back storage caches configured to ignore `FLUSH`/`FUA`, silent hardware media corruption returning bad sectors without I/O error, and external processes bypassing the sidecar `.lock` fall outside automatic `Engine::open` recovery and require `fathomdb recover` or full rebuild/restore from backup (`fathomdb.logical-export/v1`).

### 3. Power-Loss / Torn-Write Verification (`AC32-DMFLAKEY-01`–`04`)

Qualify **both** `SyncMode::Normal` and `SyncMode::Full` in Slice 50 using a reproducible, zero-host-`sudo`, zero-host-mutation disposable virtual machine environment:

- **Fault-injection venue:** User-mode `qemu-system-x86_64` over `/dev/kvm` booting a SHA-256-verified Ubuntu 24.04 (`noble-server-cloudimg-amd64.img`, `glibc >= 2.39`, kernel with `dm_flakey.ko` via `linux-generic`/`linux-modules-extra`) guest image + `xorriso` NoCloud `cidata` ISO staged under `/mnt/data/worktrees/`. Inside the guest, test harnesses attach loopback backing files (`losetup`) to `dmsetup create ... flakey` (`ext4`), execute real FathomDB workloads, inject dropped/reordered/torn block writes and flush drops, and terminate the VM abruptly with hard QEMU power-off (strictly stronger than `SIGKILL`).
- **External acknowledged-write oracle:** Every client-acknowledged mutation (`write`, `actuate`, `checkpoint`, `purge`, `erase_source`) is recorded sequentially by an external host/serial oracle *outside* the faulted block device before or immediately upon ack return so post-power-loss recovery can compare the reopened store against the exact acknowledged sequence.
- **Required acceptance criteria:**
  - **`AC32-DMFLAKEY-01` (Lifecycle cut points):** Faults are injected across all four boundaries: (a) active commit, (b) un-checkpointed WAL accumulation, (c) `PRAGMA wal_checkpoint(TRUNCATE)` including physical erasure completion, and (d) reopen/WAL-replay recovery after an earlier power cut.
  - **`AC32-DMFLAKEY-02` (`SyncMode::Normal` contract proof):** After power loss, reboot, and `Engine::open`, `PRAGMA integrity_check == ok`, `fathomdb doctor check-integrity --full` and `data-plane-integrity` report zero findings, recovered canonical state matches an intact contiguous prefix (`0..=acked_high_water`) of the external oracle (zero partial commits, zero out-of-order gaps), and subsequent writes/erasures succeed.
  - **`AC32-DMFLAKEY-03` (`SyncMode::Full` contract proof):** After power loss, reboot, and `Engine::open`, **every** externally acknowledged commit up to power-off survives (`recovered_boundary == acked_boundary`, zero lost acknowledged commits), `doctor check-integrity --full` passes, and subsequent writes succeed.
  - **`AC32-DMFLAKEY-04` (Erasure & lock safety under fault):** Power cuts during `purge` / `erase_source` and `wal_checkpoint(TRUNCATE)` never resurrect hidden rows, never wedge reopen on stale sidecar `.lock` files, and leave physical erasure queue state resumable to completion.

### 4. Standalone Coverage-Guided Fuzzing Workspace (`AC32-FUZZ-01`–`03`)

- **`AC32-FUZZ-01` (Dependency & toolchain isolation):**
  - Place all fuzz crates (`libfuzzer-sys`, `arbitrary`) inside a standalone `fuzz/Cargo.toml` (`[workspace]`, `publish = false`), explicitly excluded from the root workspace via `exclude = ["fuzz"]` in root `Cargo.toml` and classified non-shipping in `scripts/sbom-survey/tiers.toml`.
  - Keep root `rust-toolchain.toml` at `1.95.0`. Fuzz builds invoke `cargo +nightly-2026-08-01 fuzz run --sanitizer address ...` (`g++-13` + `rustc` internal `librustc-nightly_rt.asan.a`). CI enforces zero fuzzing dependencies in any shipping crate via `cargo tree --locked`.
  - Note on coverage dumps: `cargo +nightly-2026-08-01 fuzz coverage` emits `.profraw` coverage data, and generating merged `.profdata` / summary reports requires the `llvm-tools` rustup component (`llvm-profdata` / `llvm-cov`) on `nightly-2026-08-01`.
- **`AC32-FUZZ-02` (Seven target surfaces + sanitizers):** Run declared fuzzing budgets under `ASan` (and `UBSan` sweep) across seven structured targets:
  1. `fuzz_query_filter` — FTS5 query compiler and `Filter` parsing/lowering;
  2. `fuzz_property_predicates` — `PropertyPredicate` scalar typing, numeric edge values (`i64`/`f64`), and JSON bounds;
  3. `fuzz_scope_policy` — `ScopePolicy` token grammar, conflict rules, and path/control-byte rejection;
  4. `fuzz_auth_tokens` — `FrozenReadContextV1`, `EvidenceRefV1`, `GraphEvidenceRefV1`, and `PageCursor` encoding, MAC/digest verification, and scope binding;
  5. `fuzz_wire_codecs` — `PreparedWrite`, `ActuationBatchV1`, and internal `fathomdb.logical-export/v1` archive frames;
  6. `fuzz_mcp_jsonrpc` — `fathomdb-mcp` JSON configuration and tool-argument deserialization/strictness;
  7. `fuzz_export_and_open_recovery` — malformed/truncated logical-export archives and recovery metadata inputs.
- **`AC32-FUZZ-03` (Committed deterministic regressions):** Every crash, panic, timeout, or sanitizer failure discovered during fuzzing is minimized via `cargo fuzz tmin`/`cmin` and committed as an ordinary deterministic `#[test]` case runnable under stable `cargo test` with zero `libfuzzer-sys` or `arbitrary` linkage.

## Consequences

- `ADR-0.6.0-durability-fsync-policy.md` (`#7b`) is amended: `SyncMode::Normal` keeps its default role with an accurate WAL checkpoint-bounded prefix-recovery contract rather than a heuristic millisecond claim, while `SyncMode::Full` provides strict per-acknowledged-commit power-loss durability when explicitly selected.
- Both `SyncMode` profiles and the 7 fuzz targets have explicit, falsifiable Slice 50 acceptance oracles (`AC32-DMFLAKEY-01`–`04`, `AC32-FUZZ-01`–`03`) executed without altering shipping crate graphs or requiring host `sudo`.

## Citations

- `ADR-0.6.0-durability-fsync-policy.md` (`#7b`, amended)
- `ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md` (`#31`)
- Owner Ruling 2 (`dev/plans/0.8.32/prework/slice-0.md`) & HITL Gate 1 Decision 2 Option 1 (`dev/plans/0.8.32/prework/slice-9-triage.md`)
- `dev/plans/0.8.32/features/slice-10/scope-schema-experiment.md` (`WP-10D` `NORMAL` vs. `FULL` single-row checkpoint measurements)
- `dev/plans/0.8.31/quality-backlog.md` & `dev/plans/0.8.32/plan.md` (`R32-05`)
