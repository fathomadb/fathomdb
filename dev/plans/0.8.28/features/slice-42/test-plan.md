---
title: FathomDB 0.8.28 Slice 42 — test plan
status: PLANNED (rewritten 2026-10-31 for the Unit B re-scope; awaiting design review)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 test plan

Every run records the host, `nproc`, the load average, the commit and the
command. Results go into [status.md](status.md); the logs of the
consumer, Windows and perf runs go under `results/` in this directory, so
they can be re-read.

## 1. Run rules

- **Embedding work runs on the RTX 3090 `cuda:0` (owner, 2026-10-31).**
  Any run that encodes with BGE uses the CUDA embedder on `cuda:0`: build
  with `embed-cuda` (and `rerank-cuda` where a reranker runs), set
  `FATHOMDB_EMBED_DEVICE=cuda:0` (and `FATHOMDB_RERANK_DEVICE=cuda:0`),
  exclude the K620 with `FATHOMDB_GPU_EXCLUDE`, check `nvidia-smi` for
  room before the run and confirm the 3090 engaged during it. Never on
  the CPU and never on the K620. In this slice that covers the
  feature-complete gate's default-embedder targets (the gate already
  selects the first RTX 3090; § 4.2) and any eu7 run. The `AGENT_LONG`
  perf gates in `perf_gates.rs` embed with deterministic synthetic
  embedders (`DeterministicEmbedder`, `VaryingEmbedder`), not BGE. The
  Unit I tests (AC40-95 to AC40-97) do not embed.
- **Tools the owner authorized on 2026-10-31.** `ptrace` (for example
  `strace` for AC-036 in the full gate): if the sandbox denies
  `PTRACE_TRACEME`, rerun the unchanged gate unsandboxed. `nvidia-smi`
  and the other NVIDIA tools, where a run needs them.
- **Own virtual environment.** The full gate runs from this worktree with
  its own `.venv` (`./scripts/bootstrap.sh` in the worktree). Never point
  the shared `.venv` at this worktree.
- **Scratch outside the repository.** The consumer crate and the probes
  live in the session scratch directory. Probes that edit tracked files
  are reverted before any commit.

## 2. Unit A

### 2.1 Build

```bash
cargo test -p fathomdb-engine --features test-hooks --test slice135_robustness_first --no-run
```

Run the binary that this prints directly, so the 30 runs measure the
tests and not the build.

### 2.2 RED baseline (AC42-02)

At the slice base, before the edit:

```bash
fails=0
for i in $(seq 30); do
  timeout 300 "$BIN" --nocapture >"$LOG/run-$i.log" 2>&1 || fails=$((fails + 1))
done
echo "failures=$fails/30"
```

Expected: failures, near 9 in 10, all on line 437. A run that fails on
another line is reported, not counted.

### 2.3 GREEN (AC42-01, AC42-05)

After the ruled edit (design § 1.4), rebuild and repeat § 2.2. Expected
`failures=0/30`. Then the test alone 5 times
(`"$BIN" --exact slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`)
and the binary with `--test-threads=1` 3 times. All pass. The
`SLICE135_ROBUSTNESS` record of the persistent case shows
`"fd_before":0,"fd_after":0` on Linux.

### 2.4 Leak checks (AC42-03, AC42-04)

Each is an uncommitted edit, run with the test alone, then reverted with
`git restore` on the test file:

| Check | Edit | Expected |
| --- | --- | --- |
| Leaked file | Before line 429: `std::mem::forget(std::fs::File::open(&path).expect("leak"));` | FAIL on line 437, `left: Some(0)`, `right: Some(1)` |
| Leaked engine | Line 416: `std::mem::forget(reopened.engine);` instead of `close()` | FAIL on line 437, `right` above 0 |

Record the panic lines in status.md; confirm with `git status` that
neither edit is left.

### 2.5 Scope (AC42-06)

`git show --stat <Unit A sha>` lists only the test file, and its diff is
the helper and lines 366 and 429.

## 3. Unit B: RED and GREEN

Each RED commit is checked in a throwaway detached checkout at its own
sha before the GREEN lands, and the failing output goes into the commit
body as "RED evidence:". Each GREEN commit body cites the passing run.

### 3.1 SQLite version (AC42-07, AC42-08)

```bash
cargo test -p fathomdb-engine --test slice42_sqlite_bump
```

- B-RED-1: fails; the message shows 3053002.
- B-GREEN-1: passes. Then:

```bash
cargo tree -i libsqlite3-sys --workspace        # one libsqlite3-sys v0.39.1, registry
grep -A3 '^name = "libsqlite3-sys"' Cargo.lock  # 0.39.1, crates.io source, checksum
grep -A3 '^name = "rusqlite"' Cargo.lock        # 0.41.1, crates.io source, checksum
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings -A missing-docs
cargo test -p fathomdb-engine --features test-hooks --test slice40_reader_page_cache
```

Expected: one `libsqlite3-sys`, both crates from crates.io, clean check
and clippy, AC40-95 to AC40-97 pass. The live pinned-override check
fails here (design § 2.6); that is expected until B-GREEN-2.

### 3.2 Pinned-override guard (AC42-09, AC42-10)

```bash
bash scripts/tests/test_check_pinned_override_rot.sh
python -m pytest tests/experiments/test_sdk_rust_patch_policy.py
python3 scripts/check-pinned-override-rot.py
```

- B-RED-2: the shell test fails on the new refusal case (the checker
  admits the root `libsqlite3-sys` patch), and the pytest fails on the
  driver refusal assertion.
- B-GREEN-2: all three pass. Then `git ls-files third_party` prints
  nothing, and `git grep -n -A3 '^\[patch' -- '*Cargo.toml'` shows no
  `libsqlite3-sys` key. `bash scripts/security/gitleaks-current.sh` (or
  at least `check-gitleaks-current-config.py` on the edited config) and
  `bash scripts/tests/test_gitleaks_guards.sh` pass.

### 3.3 Windows guard predicate (AC42-12)

```bash
cargo test -p fathomdb-engine --lib local_verbatim_path_needs_newer_sqlite
```

- B-RED-3: does not compile (E0425 for the new function and constant).
- B-GREEN-3: passes on Linux. Then
  `git grep -n FATHOMDB_WIN_VERBATIM_SHM_SAFE -- src scripts` prints
  nothing.

## 4. Verification on the final tree

### 4.1 Scratch consumer of the registry crates (AC42-11)

In the session scratch directory, never in the repository:

```text
<scratch>/slice42-consumer/
  Cargo.toml   # [dependencies] fathomdb-engine = { path = "<tree>/src/rust/crates/fathomdb-engine", features = ["test-hooks"] }
               #   fathomdb-schema = { path = "<tree>/src/rust/crates/fathomdb-schema" }
               # [dev-dependencies] serde_json, tempfile (as the engine's)
               # no [patch]
  tests/slice40_reader_page_cache.rs   # copied unchanged
```

The crate is its own workspace, so no root `[patch]` reaches it, and it
gets a fresh lockfile: the same resolution a published-crate consumer
gets. Run it twice, with `<tree>` a scratch export of each commit:

| Tree | `cargo tree -i libsqlite3-sys` | `cargo test --test slice40_reader_page_cache` |
| --- | --- | --- |
| slice base `59079d5c5` | registry 0.38.2 | AC40-95 fails (`ENABLE_MEMORY_MANAGEMENT` used); record AC40-96 and AC40-97 |
| final tree | registry 0.39.1 | AC40-95 to AC40-97 pass |

Also print `rusqlite::version_number()` from the consumer (3054001 at the
final tree). No `LIBSQLITE3_FLAGS` in the environment for either run.

### 4.2 Full gate (AC42-14)

Once, on the final clean tree, from this worktree:

```bash
./scripts/agent-verify.sh
cargo clippy --workspace --all-targets -- -D warnings -A missing-docs
cargo check --workspace --all-targets
```

AC-036 uses `strace`; see § 1 for the unsandboxed rerun. The
feature-complete gate, if run, embeds with BGE on the first RTX 3090
(`scripts/lib/feature_complete.py` checks the device by PCI order); see
§ 1. Rerun only a suite that fails, and compare it with the base.
Known parallel-run flakes at the base, not caused by this slice:
`slice50_evidence.rs` (`evidence_search_races_are_snapshot_atomic_or_wholly_refused`,
`evidence_linearizes_at_sidecar_and_resolver_return_seams`; 6 of 15
parallel runs fail at the base, serial passes) and Unit A's test before
its fix. Record their state, do not fix them here.

### 4.3 Windows (AC42-13)

On the local Windows VM `gh-runner-wonl-win11` (shut off on 2026-10-31;
start it with `virsh start`, find the address with
`virsh net-dhcp-leases default`, connect as described in the Windows VM
memory note). Transfer an exact export of the final tree under an owned
temporary path, verify its hash on both sides, and remove it afterwards.

```powershell
cargo test -p fathomdb-engine --lib -- local_verbatim_path_needs_newer_sqlite sqlite_runtime_path_simplifies_only_safe_local_drive_paths unpatched_sqlite_refuses_unsimplifiable_local_verbatim_paths windows_trailing_component_cannot_change_database_identity engine_open_refuses_trailing_dot_before_database_or_lock_creation local_canonical_path_does_not_strand_idle_wal_readers --nocapture
cargo test -p fathomdb-engine --features operator --test erasure_completeness erasure_busy_cross_process_windows_yields_typed_diagnostic -- --exact
```

Expected: all pass, and
`local_canonical_path_does_not_strand_idle_wal_readers` runs its 3 × 50 cycles (it no longer has a version skip). If the VM cannot
be started or reached, record that in status.md and carry AC42-13 to
0.8.28 final verification (the Windows leg of the platform matrix);
the slice does not claim it.

### 4.4 macOS

No local macOS host. The macOS leg runs in the CI matrix during 0.8.28
final verification; status.md names it as open.

### 4.5 `AGENT_LONG` perf baseline (AC42-16)

On an idle host, once, on the final tree:

```bash
AGENT_LONG=1 cargo test --release -p fathomdb-engine --test perf_gates -- --test-threads=1 --nocapture
```

These gates use synthetic embedders (§ 1). Record every gate's reported
numbers, pass or fail, with the SQLite version (3.54.1), host, load and
commit, in `results/agent-long-perf-baseline.md`. They are the baseline
for Slices 50 to 90, not a pass/fail gate for this slice; a gate that
fails is reported to the coordinator with its value.

### 4.6 Records (AC42-15)

```bash
./scripts/agent-lint-md.sh
./scripts/agent-lint-docs.sh
./scripts/check-release-state-views.sh
git grep -n "libsqlite3-sys-0.38.1" -- . \
  ':!dev/plans/0.8.27' ':!dev/plans/runs' ':!dev/plans/0.8.28/features/slice-31' \
  ':!dev/plans/0.8.28/features/slice-40' ':!dev/plans/0.8.28/features/slice-42' \
  ':!experiments/configs/release-0825-slice71b-probe.Cargo.lock' \
  ':!dev/todos-and-considerations-ledger.jsonl' ':!dev/steward'
```

Expected: the lints pass and the search prints nothing. The excluded
paths are historical records, the append-only ledgers, and this slice's
own docs.
