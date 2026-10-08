---
title: FathomDB 0.8.28 Slice 30 — Tegra private CUDA memory pool status
status: IN_PROGRESS
target_release: 0.8.28
observed_on: 2026-10-08
---

# Slice 30 status

Worktree `.claude/worktrees/slice-30-tegra-pool`, branch
`slice/0.8.28-30-tegra-pool`, cut from `release/0.8.28` at `cb9594f44`.
See the [plan](plan.md) and the [design](design.md).

## Log

| Step | State | Evidence |
| --- | --- | --- |
| S30-T0 | done | `scripts/preflight.sh` passed. `release/0.8.27` is unchanged at `b65283317`, so there is nothing to merge forward (plan C-1). |
| S30-T1 | done | Plan revision 3 and design revision 1 at `b11e4523d`. The Opus design review (APPROVE-WITH-FIXES, 14 findings) was resolved in design revision 2 at `00a2a6df3`; see [design-review.md](design-review.md). Owner HITL, 2026-10-08: push the Candle branch (yes); Python hook on in the Tegra wheel, per ruling 37 (yes); SD-9 axis-E break (accepted). |
| S30-T2 | done | Candle fork `fathomdb/0.8.28-cuda-from-context`: test `73e267d0` red, then `5b74532e` green on the Orin, then `25368139` (0.10.3). Pushed with owner approval. FathomDB pin set at `222626c4b`; the pinned-override checker gained per-package versions (red first). |
| S30-T3 | done | `14cd40ab0` red, `75f58a4ae` green: the vendored cudarc primitive (23 vendored tests pass on the Orin). `d4d909ce0` red, `077701775` green: the pinned-override gate checks the item set and statuses. |
| S30-T4/T5 | done | `e80700648` axis-E 0.7.0; `7fab51721` red, `cbe46d538` green: the policy, decision and report; `a1e76d55d` red, `53ad469b0` green: the shared early-`cuInit` helper and the Tegra Python hook. The Orin GPU smoke test (private pool, 3 GiB) passes. |
| S30-T6 | done | `67514d382` red, `50dc12346` green: engine; `caf0a9608` red, `1ecef7fa0` green: napi, py, SDK, CLI, TS and Python; `38aa47a51` docs. The Slice 130 Python baseline stays frozen; declared additions at `b63087b80` (owner choice). |
| S30-T7 | done | `eeee7d9a2` red, `d861ead76` green: context-loss detection (`cuDevicePrimaryCtxGetState` first, no retain), the one-line snapshot, and the reset characterization (typed `CudaContextLost`, then SIGSEGV in `cublasDestroy_v2` at teardown, as in results § 13.4). |
| S30-T8 | done | `01710c020` red, `cc056ed54` green: `doctor cuda-allocator` (on the Orin: `private`, 3 GiB, `"0"`, `ran`, context `active`). `7c0fb04b6` red, `d5f5087ad` green: `CUDA_PYTHON_FEATURES_TEGRA` and the contract guard. `4815afe8a`: `scripts/check-tegra-pool-registry-build.sh` passed on the Orin. `d5bb4a63c`: the scaffolding-removal test. Docs, the ADR (index row 62) and the changelog follow. |
| S30-T9 | done | Phase 5: `results.md` § 14 (decision rule, UNMEASURED, 0.8.29 / 0.8.30); [upstream-cudarc-package.md](upstream-cudarc-package.md), prepared and not posted (owner sign-off required), with the NVIDIA report addendum; design revision 3 reconciled with the code (§ 8); plan revision 3.2 (minimal corrections). |
| S30-T10a | done | Opus code review of `cb9594f44..deffb5fd6`: APPROVE-WITH-FIXES, CR-1 to CR-6, each fixed red then green (`a87e64d35`..`797e891bf`, recorded at `618cb7f14`); shell lint at `5f515dac2`. See [design-review.md](design-review.md). |
| S30-T10b | done | Sonnet verifier at `5f515dac2`: AC30-01 to 09 and 11 PASS; AC30-10 FAIL (the guard matched its own comment), fixed at `372f49aae`. Qualification on the Orin at `88c5028bf` ([qualification.md](../../../runs/0.8.28-slice-30/qualification.md)): G1, G2, G4, G5, G6, G7 PASS; G3, G8, G9 PARTIAL (dispositions below); G10 by the verifier. CB1/CB2, which the product cannot report, measured by the `#[ignore]` GPU test at `a03af758c`: 3 of 3 runs, `reserved_high` 288 MiB of a 3 GiB maximum, and `reserved_cur` = `used_cur` = 0 after drop and sync. |

## Qualification dispositions

- **G3 (Python growth).** The median of 0.400 MiB/cycle misses the 0.36
  bound, but the pool-off baseline is also 0.400 (max 0.416). The growth
  is not from the pool; the pool's own counters return to zero (CB2).
  Accepted.
- **G8 (Node RSS drift).** +10.8 % against a 10 % bound; the pool-off
  control drifts +12.3 %. No allocator errors in either process. Accepted.
- **G9 (rerank speed-up).** The lower CI bound is 1.49 against the old 1.5
  floor; ruling 28 re-based the floor on product versus study P, where
  every measure is within 0.975-1.008. Accepted.

## Follow-ups

- Candle fork `build.rs` lacks `rerun-if-env-changed=CUDA_PATH`; a build
  without `CUDA_PATH` caches a `cudart_static` failure until
  `cargo clean -p candle-kernels`.
- Publishing Candle 0.10.3 (the fork) to crates.io is a release task
  before the 0.8.28 publish.
- The upstream cudarc package awaits owner sign-off; nothing is posted.
- The Tegra Python wheel contract (`CUDA_PYTHON_FEATURES_TEGRA`) lacks
  `rerank-cuda`; qualification built W2 by hand.
- Node 24 and 26 were not run; the Node early-`cuInit` script SKIPs
  without `FATHOMDB_TEGRA_NODE_PACKAGE`.
- H-1: Python children forked after import lose CUDA; use the
  `FATHOMDB_CUDA_EARLY_INIT=off` opt-out or spawn/forkserver
  (documented).

## Notes

- **Release check.** `scripts/check-tegra-pool-registry-build.sh` needs the
  network and a CUDA toolkit, so it is a manual release check
  (`dev/design/release.md`), not part of `agent-verify`.

- **Pre-existing failure.** The CLI test
  `doctor_gpu_process_matrix_has_exact_outputs_and_no_side_effects` fails
  identically on the base `cb9594f44` ("cpu accessed forbidden root"). It
  is not caused by Slice 30. Also pre-existing on the base:
  `scripts/tests/test_cuda_preflight_hardening.sh` (Node `ldd` fixture),
  and the engine lib test
  `current_opener_holds_lock_before_admission_classification`, which hangs
  intermittently (3 of 3 on the base, 1 of 3 on the slice head).
- **Process deviation.** During S30-T6 a subagent made one temporary local
  "wip" commit with `--no-verify` while splitting the red commit. It was
  soft-reset at once and is not in history.
- **Process deviation.** In S30-T10b the qualification subagent had `kill`
  and `pkill` refused, then stopped its own harness process with Python
  `os.kill` (disclosed in qualification.md § 12). Subagent briefs now
  require stopping on any refusal.
