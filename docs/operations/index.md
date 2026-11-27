# Operations

Operations docs cover the **0.8.32** documentation target (pre-publish; newest
published release: **0.8.31**).

## Single-scope default and multi-agent isolation (0.8.32, Schema 39)

**By default a FathomDB database is single-scope / single-agent** (one agent per
database, storing `scope = ""` under the default `ScopePolicy.unscoped()` /
`ScopePolicy::Unscoped`). Schema **39** `scope` is the **opt-in** isolation
layer for multi-agent or shared-store deployments (`ScopePolicy.single`,
`read_only` / `readOnly`, or `read_write` / `readWrite`). Because FathomDB
retains one-Engine-per-file exclusive locking (`<db-name>.sqlite.lock`), a
shared-store process shares its open handle across callers or opens sequential
scoped handles per phase, while single-agent deployments can run without
configuring `ScopePolicy`.

- [Tegra Pages publication](tegra-pages-publication.md) — the separately
  authorized, exact-commit route for the matching Tegra wheel and Node addon.
- [Erasure](erasure.md) — what `erase_source` / `purge` guarantee, what they
  do not, the erasure-audit record, the non-PII `source_id` rule, and
  `fathomdb doctor orphan-provenance`.
- [Immutable integrity inspection](integrity-inspection.md) — exact-version
  installation, quiescence, bounded invocation, and exit/reason handling for
  `doctor data-plane-integrity`.
- [Worktree and branch consolidation](worktree-consolidation.md) — the local,
  manifest-gated preservation workflow for safely reducing repository
  worktrees and local branches.

## Concurrency and durability limits

Use one Engine per database file, and share that handle across application
calls. The sidecar lock rejects a second Engine in the same process or another
process. Reader workers and projection workers have their own connections;
projection readiness can lag committed canonical writes. Use `drain` or the
receipt/generation readiness APIs when your next step requires derived state.
Close handles explicitly and wait for admitted work to finish.

A committed WAL can contain records absent from the main file. Quiesce and
close the application Engine before running `doctor safe-export` for a logical
snapshot. For offline inspection, follow the
[quiescence procedure](integrity-inspection.md). Do not treat a copied
main file alone as a complete WAL-mode backup. `doctor check-integrity --full`
and `doctor data-plane-integrity` inspect different layers; neither repairs a
store. Ordinary-open doctor commands may bootstrap a missing path or start
workers, while data-plane integrity uses the immutable inspection route.

### Declared WAL durability contract (`SyncMode`, 0.8.32)

FathomDB runs SQLite in `WAL` mode with `PRAGMA secure_delete = ON` and
mandatory sidecar `.lock` cross-process exclusion. Starting in the **0.8.32**
documentation target (`ADR-0.8.32` `#101`), `EngineConfig` exposes an explicit
`sync_mode` / `syncMode` knob (`SyncMode::Normal` default, `SyncMode::Full`
opt-in), applied to both the primary writer connection and all
projection-worker/runtime connections:

- **`SyncMode::Normal` (default, `PRAGMA synchronous = NORMAL`):** Guarantees
  **zero database corruption** (`PRAGMA integrity_check == ok` and clean
  `fathomdb doctor check-integrity --full`) and **atomic prefix recovery** to
  the last flushed or checkpointed WAL frame upon unclean shutdown or power
  loss. Un-checkpointed commits buffered in volatile kernel or disk caches at
  power loss may roll back atomically as a contiguous suffix, never partially
  or out of order.
- **`SyncMode::Full` (opt-in, `PRAGMA synchronous = FULL`):** Issues per-commit
  WAL `fsync` (`SQLITE_SYNC_FULL`). Guarantees **strict zero loss of every
  client-acknowledged commit** (alongside zero corruption) across OS crash and
  storage power loss when backed by an `fsync`-honoring local POSIX filesystem
  and block device, at higher single-row unbatched commit latency (amortized
  across batched writes).

### Declared storage stack and non-recoverable fault boundaries

- **Supported host storage assumptions:** Local POSIX filesystem (such as
  `ext4` with barriers / `data=ordered` journaling) over a block device that
  honors write barriers and cache flushes (`FLUSH` / `FUA`).
- **Explicit exclusions:** Network filesystems lacking reliable `flock`/`fsync`
  semantics, write-back storage caches configured to ignore `FLUSH`/`FUA`,
  silent hardware media corruption returning bad sectors without I/O error, and
  external processes bypassing the sidecar `.lock` fall outside automatic
  `Engine::open` recovery and require `fathomdb recover` or full rebuild/restore
  from backup.

### Empirical durability & fuzz verification (`0.8.32` pre-publish vs. `0.8.31` published)

- **Published `0.8.31`:** Release qualification tests cover selected CPU
  process-kill/SIGKILL, WAL, checkpoint, persistent SQLite-full/provider-fault,
  and reopen/recovery cases. They do not prove behavior under torn/reordered
  device writes, sudden power loss, or a general hardware fault model.
- **Pre-publish `0.8.32` (`dev/0.8.32`, `ADR-0.8.32` `#101`):**
  - **QEMU/KVM `dm-flakey` power-loss campaign (`11 / 11` live cases passed):**
    User-mode QEMU/KVM (`cache=none`, QMP `quit`, two-phase `INFLIGHT`/`ACK`/`ERR`
    virtio-serial oracle, `ext4` over `dm-flakey`) empirically verified
    `SyncMode::Normal` atomic checkpoint-bounded prefix rollback (including
    `4.3 MiB` WAL accumulation and `drop_writes`) and `SyncMode::Full` `0` lost
    acknowledged commits across active commit, `4.3 MiB` WAL accumulation,
    `wal_checkpoint(TRUNCATE)` + erasure, double-cut reopen/WAL replay, live
    mid-`INFLIGHT` atomic rollback and commit, mid-erasure cuts with stale
    `.lock` reclamation (`pending_erasures` `2 -> 0`), and fail-closed `EIO`
    (`error_writes`), passing `e2fsck`, `PRAGMA integrity_check`,
    `doctor check-integrity --full`, and `doctor data-plane-integrity` with `0`
    findings on every case.
  - **Coverage-guided fuzzing (`2,185,400` runs across 7 structured targets):**
    Combined `ASan` and `UBSan`/`-Zub-checks=on` sweeps over query filters,
    property predicates, `ScopePolicy`, auth/frozen tokens, wire/export codecs,
    `fathomdb-mcp` JSON/tool parsing, and corrupt open recovery reported `0`
    FathomDB defects (`23.18%` merged `llvm-cov` line coverage; upstream
    `sqlite-vec 0.1.9` 280 B `LSan` allocation in `vec0_init` documented
    separately). No general MC/DC or hardware-wide fault-model claim follows.

## Backup, restore, and rebuild runbook (0.8.32)

Because a WAL-mode database can hold committed transactions in `<db>-wal` that
are not yet checkpointed into the main file, **never back up a live store by
copying only the main SQLite file**. Use `fathomdb doctor safe-export` to
create a self-contained logical snapshot (`VACUUM INTO`) with a SHA-256
manifest, and rebuild derived indexes on a restored copy via separate `recover`
invocations when needed:

1. **Quiesce and close the application `Engine`.** Ensure no process holds
   `<db>.lock` and all admitted writes have returned.
2. **Export a consistent snapshot and manifest:**

   ```bash
   fathomdb doctor safe-export ./backup.sqlite \
     --manifest ./backup.manifest.json \
     ./source.sqlite
   ```

   Verify the exported file's SHA-256 matches `sha256` in
   `backup.manifest.json`.
3. **Copy the verified snapshot to the restore destination:**

   ```bash
   cp ./backup.sqlite ./restored.sqlite
   ```

4. **Rebuild derived projections and `vec0` storage (when required):**
   `fathomdb recover` executes one selected action per command, so run
   `--rebuild-projections` and `--rebuild-vec0` as **separate** invocations
   (each exits `64` on accepted-loss completion):

   ```bash
   fathomdb recover --accept-data-loss --rebuild-projections --json ./restored.sqlite
   fathomdb recover --accept-data-loss --rebuild-vec0 --json ./restored.sqlite
   ```

5. **Re-materialize dense vectors (if the store uses embeddings):** Reopen
   `./restored.sqlite` with its matching embedder enabled
   (`use_default_embedder=True` in Python, `useDefaultEmbedder: true` in
   TypeScript, or `OpenOptions::default().with_default_embedder(true)` in Rust),
   call `drain(...)` until dense projections finish catching up, and close the
   handle cleanly. If FTS sanitation debt remains after rebuild, discharge it
   offline before integrity verification:

   ```bash
   fathomdb recover --accept-data-loss --complete-erasures --json ./restored.sqlite
   ```

6. **Verify post-restore integrity across both operator layers:**

   ```bash
   fathomdb doctor check-integrity --full --json ./restored.sqlite
   fathomdb doctor data-plane-integrity --json ./restored.sqlite
   ```

   Both commands must exit `0` with `clean` status and `0` findings before
   returning the restored store to application traffic.

See also [Reference — CLI](../reference/cli.md) for the full `doctor` /
`recover` / `migrate` verb tables, [Guide — MCP Server](../guides/mcp-server.md)
for `fathomdb-mcp` stdio lifecycle and opt-in scoped writes, and
[Positions — recovery surface](../positions/recovery-surface.md) for why
recovery is CLI-only while erasure is not.
