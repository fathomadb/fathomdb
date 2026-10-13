# Slice 31 status

Step log for [plan.md](plan.md) (revision 16; its § 8.1 and the
"Update 2026-10-13 (revision 16)" part give the slice state on
2026-10-13). Red and green shas per
FathomDB step, merge-forward shas, host guards, the fork commits `C` and
`K`, and the pin-chain checks (plan § 4). Fork-side step logs are in each
fork's `fathomdb/status.md`.

## Plan and fork plans

| Item | Commit |
| --- | --- |
| Slice 31 plan revision 14, `GOAL.md` | `80ae2a18e` on `release/0.8.28` |
| Slice 31 plan revision 15 (§ 8.1 state) | `eb75e31d5` on `release/0.8.28`, merged into this branch |
| Merge-forward of `release/0.8.27` (W0 step 2) | `9fa71e93a` |
| cudarc fork plan | `coreyt/cudarc` `fathomdb/0.19.10`, `fathomdb/plan.md` (tip `d3a5a055`) |
| Candle fork plan | `coreyt/candle-fathomdb` `fathomdb/0.8.28-cudarc-fork`, `fathomdb/plan.md` (tip `549f3548`) |

## W0 (prerequisites)

| Step | State (2026-10-13) |
| --- | --- |
| 1 Plan, GOAL, merge-forward plan | done (revision 14 at `80ae2a18e`) |
| 2 Merge-forward | done at `9fa71e93a`; repeat before R-CODE-3 if `release/0.8.27` advances |
| 3 Preflight, slice worktree, `status.md`, `review.md` | done: worktree `../fathomdb-worktrees/slice-0.8.28-31` on `slice/0.8.28-31-cudarc-fork` from `origin/release/0.8.28` at `80ae2a18e`. `scripts/preflight.sh --worktree … --min-disk-gb 15`: disk 76 GB free and the checks pass except one hard failure, "release-state discovery failed: no live release": no 0.8.28 release-state file exists yet (plan header: this plan is D28-13's working plan until the 0.8.28 plan, state file and board exist). Recorded, not worked around |
| 4 Toolchains; CUDA 13.0 | `windchill3` guard: driver 580.178.04, `/usr/local/cuda` → 12.6 (every fork receipt records it before and after). The runfile download was not approved; the 13.0 toolkit cells run on a GCP VM (ruling Q15) |
| 5 Orin check | done during the fork cells (L4T R36.5.2, 12.6, `~/fathomdb-forks/`) |
| 6 Slice `.venv` via `scripts/bootstrap.sh` | not yet; before the first full gate |
| 7 Ledger | done in `80ae2a18e`: steward ledger seq 303–317, todos seq 292–299 |

## Fork commits and the pin chain

| Fork | Qualified commit | Reviews | Pin-chain check (2026-10-13) |
| --- | --- | --- | --- |
| cudarc | `C` = `b777f8d94eedca8334e9331cf520d554b22544b7` | R-FC-VERIFY APPROVE-WITH-NOTES, no open finding | `git diff C d3a5a055 -- . ':(exclude)fathomdb'` is empty; CI receipt `2026-10-12-CI-b777f8d.md` has `head_sha` = `C` |
| Candle | `K` = `853324fc2b9f813eedceb75bef660a06a413ff5b` | R-FK-VERIFY APPROVE-WITH-FIXES ("fit to pin"), all findings disposed | `K`'s root `Cargo.toml` names `cudarc-fathomdb` at `https://github.com/coreyt/cudarc.git`, `rev = "b777f8d94eedca8334e9331cf520d554b22544b7"`. `git diff K 549f3548 -- . ':(exclude)fathomdb'` touches only the root `FATHOMDB.md` (`887e5781`, R-FK-VERIFY F1: its commit table), which no published crate packages; no code changed after `K` |

Receipts: cudarc `fathomdb/qualification/` (G1–G6, G8, CI, CF-AC-07a and
07b); Candle `fathomdb/qualification/2026-10-12-K-G1-K-G3-853324fc.md` and
`2026-10-12-K-G2-853324fc.md` with their `.log` files.

## Publication preparation

- cudarc FC10 (CF-AC-07b) was run early on the owner's request: receipt
  `2026-10-13-CF-AC-07b-b777f8d.md` (`a6d49c41`, fixes `d3a5a055`).
  R-FC-PUB: APPROVE-WITH-FIXES; finding 1 (the README's relative
  `FATHOMDB.md` link resolves against `coreyt/cudarc`'s default branch
  `main`) is an owner decision before R-PUB. R-PUB waits for R-VERIFY.
- Candle FK5: K8 `cd64715e` (0.10.4 on registry `cudarc-fathomdb` 0.19.10)
  passed CK-AC-07 on the RTX 3090 and the Orin on 2026-10-13 (fork receipt
  `fathomdb/qualification/2026-10-13-CK-AC-07-cd64715e.md`, fork commit
  `ddff4cbd`); R-FK-PUB is running.

## Next

W1 (`requirements.md`, drafted) → R-REQ, W2 → R-DES, W3 → R-TEST, then W6
with registry `cudarc-fathomdb` `=0.19.10`, the Candle crates `=0.10.4` and
the `candle-kernels` git patch at the qualified Candle commit, under the
owner-approved R-PIN checklist (plan § 9 W6, Q20).

## Log

- 2026-10-13: plan revision 16 (registry consumption after the early
  `cudarc-fathomdb` 0.19.10 publication; rulings Q17–Q20); `GOAL.md`
  requires revision 16.
- 2026-10-13: W1 drafted: [requirements.md](requirements.md) (DRAFT FOR
  R-REQ), REQ31-01…15 with AC31 sets per review and seven open points.
  Next: R-REQ.
