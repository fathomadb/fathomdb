# Slice 31 status

Step log for [plan.md](plan.md) (revision 18; its § 8.1 and the
"Update 2026-10-13" parts for revisions 16 to 18 give the slice state on
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
| 6 Slice `.venv` via `scripts/bootstrap.sh` | done 2026-10-13 in the slice worktree: `bash scripts/bootstrap.sh` exit 0, `.venv` on Python 3.12.3 with `src/python[dev]` editable; gitleaks, actionlint 1.7.12, shellcheck 0.11.0 and lychee already present, no apt step. Its `npm install` in `src/ts` reordered two keys in `src/ts/package-lock.json`; restored, not committed |
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
  R-FC-PUB: APPROVE-WITH-FIXES; finding 1 closed by making
  `fathomdb/0.19.10` the fork's default branch (Q17). By owner direction
  (Q17) R-PUB was approved before R-VERIFY, and `cudarc-fathomdb` 0.19.10
  was published from `C` on 2026-10-13 at 14:02Z (receipt
  `2026-10-13-FC10-publish-b777f8d.md`, fork commit `9b0ef75`).
- Candle FK5: K8 `cd64715e` (0.10.4 on registry `cudarc-fathomdb` 0.19.10)
  passed CK-AC-07 on the RTX 3090 and the Orin on 2026-10-13 (fork receipt
  `fathomdb/qualification/2026-10-13-CK-AC-07-cd64715e.md`, fork commit
  `ddff4cbd`). R-FK-PUB approved it apart from finding 1: 0.10.3 → 0.10.4
  replaces candle-core's re-exported cudarc with `cudarc-fathomdb`, a
  semver-breaking change. Owner ruling Q21: publish 0.11.0. `K9` =
  `182ceab0547348993b6dbec4b1aed610dcc8b23f` (record `4abb8b38`) is `K8`
  with versions 0.11.0 and a fork `description`; it is the publication
  commit. CK-AC-07 cells at `K9` (control `K8`) are running; the R-FK-PUB
  re-check of `K9` and the owner's publication of 0.11.0 follow. `K8`'s
  receipt stands as evidence for the same code. `git diff 853324fc
  182ceab0 -- candle-kernels` is empty.

## W6 (implementer, 2026-10-13)

Test-plan § 2 order. RED is the observed failure; the commit bodies hold
the full output.

| Row | RED | GREEN | Observed RED |
| --- | --- | --- | --- |
| R1 single-cudarc guard | `0fe295edd` (arm rewired `f1a27631d`) | `d3381cc3b`; real-tree arm GREEN at `aa898677f` | `FAIL guard script scripts/check-single-cudarc.sh is absent` |
| R2 pinned-override schema 5 | `36f48f95b` | `aa898677f` (pin commit) | `UNVERIFIED pinned-override-rot: metadata schema_version must be 4`; each new arm's message absent |
| R3 lock-set guard | `0ef134ea0` | `f8a2ad147` (hermetic arms); real-tree arm open until the fork records the set | `FAIL guard scripts/check-cudarc-fathomdb-lock-set.py is absent` |
| R4 `CUDARC_CUDA_VERSION` | `f243750e4` | `229cb0890` | `FAIL rejects a CUDA job without CUDARC_CUDA_VERSION 12060 (cuda-contract-preflight) (the checker passed)` |
| R5 release-contract Candle block | arms `f243750e4`; RED of record at `aa898677f` | `b9da8c591` | `cuda-release-contract: [patch.crates-io] lacks a source pin for 'candle-core-fathomdb'` (baseline) |
| R6 retired references | `88e871a57` | partly at `aa898677f`; references commit pending | 15 files, 10 marker lines, `tegra-pool` = `['dep:cudarc', 'cudarc/fathomdb-private-pool']` |
| R7 PTX floor | `8346b4661` | `fffc6dd0f` | `FAIL checker scripts/check-ptx-driver-floor.py is absent` |
| R8 compute floor (C4) | `2cc4e9ae0` (compile), `9166d660c` (behaviour) | `26f917eae` | E0432/E0407; then 5 failed: `left: Ok(... compute_capability: Some("5.0") ...) right: Err(ForcedCudaUnavailable { ordinal: 0, reason: CudaIncompatible })` |
| R9 D-18 pure part | `943e5c9c1` | `a48ef0bc3` | E0425/E0433 (types and constants absent) |
| R11 T-REGISTRY | old script at `aa898677f` | `68f3233b0` (REG-a GREEN on windchill3) | `[patch.crates-io] must carry exactly one cudarc entry`, rc=1 |
| R18 required-symbol guard | `9f0e58219` | `b95bcedc2`; real tree `c9d67f2f5` | `FAIL checker scripts/check-cuda-required-symbols.py is absent` |

**Pin commit (Phase B, R-PIN): `aa898677f3eff2fb8fb40e2108a1b29072f0e7fa`,
pushed 2026-10-13** so the cudarc fork's Run 2 can fetch its `Cargo.lock`
from `raw.githubusercontent.com` (design § 7.2). `Cargo.lock` SHA-256
`b887a101a1ea9684a8e64d0bb8a849da6ef4e61a777bdc2ff62062126a8d142a`.
R-PIN evidence (commit body): one `cudarc-fathomdb` 0.19.10 from crates.io,
checksum `8780d2fe…caf2`; `candle-{core,nn,transformers}-fathomdb` 0.11.0
from crates.io with the FK5 checksums; `candle-kernels` 0.10.2 from
`git+https://github.com/coreyt/candle-fathomdb.git?rev=182ceab0…`; the
only other `cudarc` is 0.17.8 (ug-cuda); patch table `['candle-kernels',
'libsqlite3-sys']`; `tegra-pool = []`; no target dependency tables;
`.cargo_vcs_info.json` sha1 `b777f8d9…` and `182ceab0…`; relied-on items
1, 2, 3, 5, 9 and K1, K2, K3, K5, K7 present in the fork tables;
`check-pinned-override-rot.sh` and `check-single-cudarc.sh` ok;
workspace and `embed-cuda,rerank-cuda` `cargo check` pass (build only).

**Deviations so far.**

- R1: the flash-attn exclusion arm passes `--lock lock/ok.lock`; the
  fixture's own lock must hold the optional `cudarc` 0.19.4, which trips
  condition A (`f1a27631d`). The expectation is unchanged.
- R8: an intermediate commit (`9166d660c`) adds only the trait method and
  parser, so the behavioural REDs are observed, not only the compile
  failure.
- D-18 `required_major`: the design says the fork's
  `LibLoadError.cuda_major` gives 10 for cuRAND; at `b777f8d` it is the
  CUDA major (12) for every library, so FathomDB maps cuRAND to its soname
  major 10 itself.
- R9's two `LibLoadError` tests and R10 (CLI) need the fork types and the
  driver part, so they are written RED on the pinned tree (Phase C).
- Existing policy-test fakes that report compute capability 8.6 and do not
  override `kernel_compute_floor()` would be refused in a build with
  `CUDA_COMPUTE_CAP=87`; no Orin window runs those suites.

## Next

W1 (`requirements.md`) and W2 (`design.md` revision 2, revised after
R-DES) done; W3 drafted ([test-plan.md](test-plan.md), DRAFT FOR R-TEST);
next R-TEST, then W6
with registry `cudarc-fathomdb` `=0.19.10`, the Candle crates `=0.11.0` and
the `candle-kernels` git patch at `K9`, under the owner-approved R-PIN
checklist (plan § 9 W6, Q20, Q21).

## Log

- 2026-10-13: plan revision 16 (registry consumption after the early
  `cudarc-fathomdb` 0.19.10 publication; rulings Q17–Q20); `GOAL.md`
  requires revision 16.
- 2026-10-13: W1 drafted: [requirements.md](requirements.md) (DRAFT FOR
  R-REQ), REQ31-01…15 with AC31 sets per review and seven open points.
  Next: R-REQ.
- 2026-10-13: plan revision 17: the Candle fork crates publish as 0.11.0
  from `K9` (`182ceab0`), not 0.10.4 from `K8` (owner ruling Q21, R-FK-PUB
  finding 1); W6 pins the Candle crates `=0.11.0` and `candle-kernels` at
  `K9`; `GOAL.md` and `requirements.md` follow.
- 2026-10-13: R-REQ (Opus) on `requirements.md` at `f41563903`:
  APPROVE-WITH-FIXES, F1–F12 and seven open-point rulings, all applied
  ([review.md](review.md)); new REQ31-16 (REQ-068 through the
  `candle-kernels` patch); T-REGISTRY moves into W6. `requirements.md` is
  REVISED AFTER R-REQ, ready for W2. Next: W2 → R-DES.
- 2026-10-13: W2 drafted: [design.md](design.md) (DRAFT FOR R-DES,
  revision 1): D-8W, D-10, D-11 (exact `=0.19.10`/`=0.11.0` registry pins,
  marker feature dropped, pinned-override schema 5, single-cudarc guard,
  T-REGISTRY), D-12, D-14, D-15 (Slice 30 teardown assertion kept for
  0.8.29), D-16 (lock-set guard and fork record format), D-17 (14 call
  sites, 2 changed), D-18, the ADR amendment draft and seven open points.
  The Candle crates 0.11.0 are on crates.io (Candle fork receipt
  `2026-10-13-FK5-publish-182ceab0.md`, record commit `e9c6ed5e`). Next:
  R-DES.
- 2026-10-13: R-DES (Opus) on `design.md` revision 1 at `1e0537856`:
  APPROVE-WITH-FIXES, findings 1–10 and seven open-point rulings, all
  applied with owner rulings Q22–Q25 (steward ledger seq 325–328) in
  `design.md` revision 2 (REVISED AFTER R-DES, ready for W3),
  `requirements.md` (REQ31-10, AC31-07, AC31-10, AC31-14, AC31-15) and
  plan revision 18 ([review.md](review.md)). Main changes: hermetic C4 with
  an unconditional compute-capability floor; RTX 3090 by UUID for every
  new `windchill3` process; the embedder's direct `cudarc` dependency
  removed; `LibraryUnavailable` reached through Candle's re-exported
  cudarc; the Linux-normal lock-set closure; C3 as a GCP T4 cell (design
  § 9.3); fork todos FT-1–FT-3 to file when W6 lands. Next: W3 → R-TEST.
- 2026-10-13: W3 drafted: [test-plan.md](test-plan.md) (DRAFT FOR R-TEST,
  revision 1): hosts and guards (RTX 3090 by UUID, Orin by git bundle, GCP
  sessions S-13 and S-C3), the W6 RED-first order R1–R17, a catalogue of
  every AC31 and plan § 7 topic with exact commands, the qualification
  runs (G1–G10, G9 in full, the RTX 3090 arm, D-14, usable capacity, C3),
  traceability, the single-cudarc fixtures verified in a scratch
  directory, and eleven open points (OP-1: the release contract still pins
  the Candle git cohort at `859b8ea1`; OP-2: AC31-10's `git grep` must
  exclude the steward ledger; OP-3: the feature-complete gate selects GPUs
  by index). T-REGISTRY resolution measured in a scratch consumer with no
  `[patch]` table. Next: R-TEST.
- 2026-10-13: R-TEST (Opus) on `test-plan.md` revision 1 at `72a471508`:
  APPROVE-WITH-FIXES, findings 1–17, rulings on OP-1 to OP-11 and
  proportionality cuts, all applied with owner rulings Q26–Q28 (steward
  ledger seq 329–331) in `test-plan.md` revision 2 (REVISED AFTER R-TEST,
  ready for W6), `design.md` revision 3, `requirements.md` (AC31-02,
  AC31-04, AC31-10, REQ31-10, REQ31-15, AC31-15) and plan revision 19
  ([review.md](review.md)). Main changes: embeds and reranks through the
  CUDA wheel and the embedder's live test binaries (no CLI embed verb);
  the `libcuda.so.1`-only cell moved to S-C3; D-18 refuses a cuBLAS or
  cuRAND missing a required symbol (Q26; FathomDB-side check through the
  fork's `is_symbol_present`, no fork change); the cuBLAS and cuRAND
  remedy; message-asserting release-contract arms; Orin windows P, A, B
  and C; `QUAL_QUIET_STRICT=1` for G9; S-13 sizing (Q27); G5's Slice 30
  equality report-only (Q28). Main-thread todo to ledger: move the
  feature-complete gate from GPU indices to UUID selection (OP-3). Next:
  W6.
