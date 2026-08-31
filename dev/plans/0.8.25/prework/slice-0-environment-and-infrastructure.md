---
title: 0.8.25 Slice 0 — environment and project-infrastructure inventory
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 0 — environment and project-infrastructure inventory

## Outcome

The durable release branch and worktree are ready for planning. Rust,
TypeScript, Markdown, storage, CPU, GPU, and unconfined ptrace capacity exist.
The current Python shim is not release-bound: its interpreter and imported
`fathomdb` package resolve under the primary `main` checkout. No product,
dependency, workflow, runner, registry, public-documentation, or credential
change was made.

## Release workspace

| Item | Verified state | Disposition |
| --- | --- | --- |
| Branch | `release/0.8.25`; remote tracking existed at `af53d173` before Slice 0 records | Keep; later pushes remain explicit HITL actions. |
| Worktree | `/home/coreyt/projects/fathomdb-worktrees/release-0.8.25` | Durable and isolated; primary checkout remains read-only. |
| Merged baseline | The plan records 0.8.24-containing main at `fca10bd2`, performance import `9ce9fcbd`, and main reconciliation through `4fc1b890`. Current `origin/main` is `c89286d9` (`v0.8.24`). | Preserve the release history; do not reset or reconstruct it. |
| Local data | `data` links to `/home/coreyt/projects/fathomdb/data`; root `node_modules` links to the primary checkout. | Planning-only shared inputs. A mutating run must use a private copy/cache. |
| Secret hygiene | `.gitignore` covers `.env*`; no root `.env*` file was present in this worktree during inspection. | Keep values outside Git. Record only required variable names. |

## Environment inventory

| Area | Observed evidence | Required state and disposition |
| --- | --- | --- |
| Host resources | Ubuntu 24.04.4 x86_64, kernel 7.0.0-30, 24 logical CPUs, 62 GiB RAM, 206 GiB free | Adequate for planning and bounded local builds. Resource-heavy Slice 75 runs still need workload isolation and receipts. |
| Rust | Repository override selects Rust/Cargo 1.95.0 with rustfmt and Clippy; only `x86_64-unknown-linux-gnu` is installed | Local x86 verification is ready. AArch64, Windows, and packaging targets remain workflow/executor responsibilities. |
| Python | Host 3.12.3; project supports >=3.10 and pins Ruff 0.15.17/Pyright 1.1.410. The shim executes `/home/coreyt/projects/fathomdb/.venv/bin/python` and imports `/home/coreyt/projects/fathomdb/src/python/fathomdb/__init__.py`. | **Gap P25-ENV-01.** Do not treat current Python checks as release proof. Use a wheel built from this worktree and installed into a fresh external venv; propose any agent-script integration in Slice 7. |
| Node/TypeScript | Node 25.9.0 and npm 11.19.0; manifests declare npm 11.12.1; TypeScript dependencies are available through a shared ignored link | Use the lockfile/pinned package-manager contract for reproducible work. Slice 1 determines whether host/tool version drift requires preparation. |
| SQLite | No `sqlite3` CLI. Python supplies SQLite 3.45.1 with FTS5, RTREE, session, and load-extension support; `MAX_WORKER_THREADS=8` and `DEFAULT_WORKER_THREADS=0`. Engine uses its Rust/native dependency closure. | CLI absence does not block product tests. Query-plan/FTS experiments must record the SQLite library actually used by the Engine, not Python's module. |
| CUDA | Unconfined `nvidia-smi` reports two RTX 3090 24 GiB devices and one Quadro K620 on driver 580.173.02; sandboxed access fails. `nvcc` is absent. | GPU evaluation is available through an authorized unconfined path. CUDA source/artifact builds require the existing CUDA 12.6 runner/image route, not this host shell. Do not use the K620 for evaluation. |
| ptrace | Sandboxed `PTRACE_TRACEME` fails; the unchanged unconfined `strace true` probe passes. | AC-036/strict ptrace gates must run unconfined. Do not disable them. |
| Tegra/L4T | Repository contains the dedicated self-hosted ARM64 Jetson workflow and host-native nonpublishing wheel builder; this x86 host is not Tegra evidence. | Preserve the proven 0.8.24/0.8.23 route. Any 0.8.25 contract extension belongs to the owning feature slice; public distribution policy is not decided here. |
| Windows | Hosted Windows x64 native build/smoke and WAL diagnostics exist; memory records a local Windows VM. No Windows CUDA artifact route is established by this host. | Keep CPU/native parity in each feature slice. Treat Windows CUDA as a separate requirement and do not import 0.8.26 work without an explicit allocation. |
| Evaluation support | Shared `data` exists; performance assets are committed; Airlock/model caches and credential files are local operational inputs | Do not make them release prerequisites unless a measurement slice explicitly requires them. Copy mutable run inputs into an isolated location and retain configuration/receipt hashes. |

## Project-infrastructure inventory

| Surface | Current mechanism | Gap or allocation |
| --- | --- | --- |
| Release authority | `release-state-*.json` plus generated board views | Established for 0.8.25 in this slice. JSON is the only writer for generated facts. |
| Release-branch state | Generic `landed` rendering means reachable from `origin/main`; only 0.8.23 has a release-specific completion exception | **P25-INFRA-03 → Slice 7 proposal:** generalize branch-completion identity or retain explicit ladder statuses without false main-landed claims. |
| Verification | `agent-{lint,typecheck,test,verify}.sh`; `check.sh`; release-installed smokes | **P25-ENV-01 → Slice 7 proposal:** add or document a worktree-safe Python/native verification mode using built wheels and an external venv. |
| CI/platform matrix | Linux x64/ARM64, macOS x64/ARM64, Windows x64, CUDA x64, and a separate Tegra route | Feature slices must add only missing contract-specific selectors/tests. Do not duplicate the matrix in prework. |
| Registry proof | Ordered release workflow, artifact upload/download, registry-installed smokes, and recovery guards | No registry action in prework. Slice 75 audits new surfaces; publication remains separately authorized. |
| Documentation | `dev/DOC-INDEX.md`, detailed indexes, plan, board, and generated-view lint | Add each new prework record to both developer indexes in its closing commit. |
| Data and secrets | `.env*` ignored; experiment inputs/results have explicit tracked/gitignored boundaries | No secret values in receipts. Mutable local datasets must not be changed through the shared `data` link. |
| Feature prerequisites | Foldback plan allocates every current Memex/performance need to Slices 10–75 | Slices 3–5 may refine requirements, architecture, and proof inside those owning plans; no generic unallocated feature backlog. |

## Selected verification arrangement

For Slices 0–6, use release-bound Rust and documentation commands; Python
tooling from the shim may lint/typecheck named source files but is not native
runtime evidence. Before Slice 7 or any Python-affecting feature closes:

1. build a wheel from the exact release worktree commit with `maturin build`;
2. create a fresh venv outside every repository/worktree;
3. install that wheel plus the reviewed test-tool set without editable mode;
4. prove `fathomdb.__file__` and `_fathomdb` resolve inside that venv; and
5. run the selected tests with the release source/test paths explicit.

This uses existing artifact-smoke practice and does not rebind `main`. Wiring a
convenient, repeatable agent command is proposal P25-ENV-01 for Slice 7; the
current slice does not implement it.

## Valuable observations

- The generic `agent-build.sh` performs editable Python installation when its
  sentinel is stale. It is unsafe with the current shim and must not be used as
  release-branch Python proof.
- Sandboxed hardware failures are execution-envelope facts, not evidence that
  CUDA or ptrace is absent.
- The Python SQLite module is useful capability evidence but not a substitute
  for recording the Engine's linked SQLite in performance claims.
- Release-state can track the next slice on a release branch, but its generic
  completion claim is still main-specific. Prework uses explicit per-entry
  `COMPLETE_ON_RELEASE_BRANCH` status and keeps `landed` truthful.
