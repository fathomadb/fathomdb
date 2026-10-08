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

## Notes

- **Pre-existing failure.** The CLI test
  `doctor_gpu_process_matrix_has_exact_outputs_and_no_side_effects` fails
  identically on the base `cb9594f44` ("cpu accessed forbidden root"). It
  is not caused by Slice 30.
- **Process deviation.** During S30-T6 a subagent made one temporary local
  "wip" commit with `--no-verify` while splitting the red commit. It was
  soft-reset at once and is not in history.
