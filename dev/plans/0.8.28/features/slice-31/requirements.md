---
title: FathomDB 0.8.28 Slice 31 — requirements and acceptance criteria (W1)
status: DRAFT FOR R-REQ (W1, 2026-10-13); from plan revision 16 § 5
target_release: 0.8.28
observed_on: 2026-10-13
---

# Slice 31 requirements and acceptance criteria

This is the final form of REQ31 and AC31, written in W1 from
[plan.md](plan.md) § 5 (revision 16) and reviewed by R-REQ. After R-REQ it
governs where it differs from plan § 5. It adds no requirement to § 5;
gaps are listed in [§ 7](#7-open-points-for-r-req) for R-REQ to rule on.

Sources traced: D28-13 (`dev/plans/0.8.28-draft-scope.md`), Slice 30's
R30/AC30 (`slice-30/plan.md` § 3), REQ-068 (`dev/requirements.md`), the
cudarc fork's CF-REQ/CF-AC (`fathomdb/requirements.md` on `coreyt/cudarc`
`fathomdb/0.19.10`, § 4 traceability) and the Candle fork's CK-REQ/CK-AC
(`fathomdb/requirements.md` on `coreyt/candle-fathomdb`
`fathomdb/0.8.28-cudarc-fork`).

## 1. Pinned commits and registry versions

| Name | Value | State (2026-10-13) |
| --- | --- | --- |
| `C` | cudarc `b777f8d94eedca8334e9331cf520d554b22544b7` on `coreyt/cudarc` `fathomdb/0.19.10` | R-FC-VERIFY APPROVE-WITH-NOTES, no open finding; published as `cudarc-fathomdb` 0.19.10 |
| `K` | Candle `853324fc2b9f813eedceb75bef660a06a413ff5b` on `coreyt/candle-fathomdb` `fathomdb/0.8.28-cudarc-fork`, pinning `C` by git | R-FK-VERIFY APPROVE-WITH-FIXES ("fit to pin"), all findings disposed |
| `K8` | Candle `cd64715ead4f86470b195a9010c4f60f7a4c0833` on the same branch (record `6b350638`): `K` with the three crates at 0.10.4 on registry `cudarc-fathomdb` 0.19.10 | FK5: local checks pass; CK-AC-07 GPU cells running; R-FK-PUB and publication follow (Q18) |
| `cudarc-fathomdb` | crates.io 0.19.10, published 2026-10-13 14:02Z from `C` | registry `.crate` SHA-256 `8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2` equals the dry-run `.crate`; `.cargo_vcs_info.json` sha1 = `C`; all 131 source files match `C` |
| `candle-core-fathomdb`, `candle-nn-fathomdb`, `candle-transformers-fathomdb` | crates.io 0.10.4, from `K8` | not yet published |
| `candle-kernels` | git patch on `https://github.com/coreyt/candle-fathomdb.git`, full `rev` of the qualified Candle commit (`K8` or `K`: [§ 7](#7-open-points-for-r-req) item 1); version 0.10.2 | `git diff 853324fc cd64715e -- candle-kernels` is empty |

FathomDB's W6 pins (plan § 9 W6 step 2): the embedder's aarch64-Linux
`cudarc = { package = "cudarc-fathomdb", version = "=0.19.10", … }`, the
Candle crates at `=0.10.4`, and a root `[patch.crates-io]` holding only
`candle-kernels` (git) and `libsqlite3-sys` (path). Exact versus caret for
`cudarc-fathomdb` is confirmed by D-11 in W2.

## 2. How criteria are judged

- **Fork-backed.** A fork-backed criterion passes when the fork's named
  criterion is recorded passing at the pinned commit, with that fork's
  R-…-VERIFY approving it and no open finding. FathomDB does not re-run
  the fork's tests (I-18). Evidence is the fork's `fathomdb/review.md`
  verdict and the receipts in its `fathomdb/qualification/`, linked by
  permalink at the receipt commit's sha in `status.md`.
- **Registry crates carry the fork evidence** when the published crate's
  `.cargo_vcs_info.json` names the qualified commit and its packaged files
  equal that commit's (for `cudarc-fathomdb`: the FC10 receipt). For `K8`,
  Candle CK-AC-07 is the re-qualification.
- **Product.** A product criterion passes on FathomDB's own tests and
  receipts (`status.md`, `qualification.md`, CI logs), each test named
  with a nonzero run count, never only a pass summary.
- **Post-publication.** Checked at R-CODE-4, after the owner publishes.
- **UNMEASURED by owner decision** is a pass state only where an owner
  ruling is cited (Q4, Q5, Q15); R-VERIFY confirms each such cell.

## 3. Requirements

Each entry: statement, rationale, kind, acceptance criterion and its
evidence. "Fork receipts" means the fork's `fathomdb/qualification/` at the
named commit.

### REQ31-01 Fork base

- **Statement:** `fathomdb/0.19.10` is cut from upstream `v0.19.10`
  (`ec1cac71`; no tag pushed to the fork); the delta is small
  upstream-style commits.
- **Rationale:** the delta stays reviewable and removable item by item
  (I-4).
- **Kind:** fork-backed.
- **AC31-01:** cudarc CF-AC-01 at `C`. Evidence: R-FC-VERIFY (cudarc
  `fathomdb/review.md`, "R-FC-VERIFY, 2026-10-12": Tree A rebuilt from
  FathomDB's patches, `diff -r` empty; 46 code commits match the status
  table); cudarc `fathomdb/status.md` "Code commits since `ec1cac71`
  (CF-AC-01)".

### REQ31-02 Pool correctness

- **Statement:** the private pool never touches the default or current
  pool; frees are stream-ordered; `CudaSlice`-owned allocations never
  outlive the pool; leaked pointers are a documented precondition and the
  `upgrade_device_ptr` doc is corrected; `maxSize`, threshold and trim
  behave; zero length works; `Send`/`Sync` are justified; multi-stream use
  is correct. Candle devices built from a private-pool context use it.
- **Rationale:** a memory pool that returns memory late or to the wrong
  pool defeats its purpose (I-13a, I-16); traces R30-01.
- **Kind:** fork-backed and product.
- **AC31-02:**
  - fork-backed: cudarc CF-AC-02 and CF-AC-03 at `C`, with the receipts
    R-FC-VERIFY verified; Candle CK-AC-02 at `K` (test
    `device_on_a_private_pool_context_allocates_from_that_pool`, K-G1 on
    RTX 3090: the x86 runtime evidence of a private-pool context driven
    through Candle), carried to `K8` by CK-AC-07;
  - product: FathomDB's private-pool GPU tests (T-POOL-PRODUCT: the four
    Slice 30 tests in `tests/tegra_fragmented_va_cuda.rs` and the Slice 30
    product tests) pass on the Orin (CUDA 12.6) against the W6 pins
    (registry `cudarc-fathomdb` 0.19.10 and Candle 0.10.4), each listed by
    name with a nonzero run count;
  - on RTX 3090 FathomDB's pool path is not built (plan § 2), so FathomDB
    asserts only the default allocator path and `doctor cuda-allocator`
    reporting `not_built` (T-CUDA).

### REQ31-03 CUDA compatibility

- **Statement:** cudarc compiles for all 19 features with default and
  `no-std` feature sets on Linux and Windows and runs its GPU tests on
  driver 580 for 12.6, 13.0, 13.2 and 13.4 bindings; Candle and the
  FathomDB workspace build and run with the 12.6 toolkit and the 13.0
  toolkit (on an owner-approved GCP VM, Q15); no removed or deprecated
  Driver API is used.
- **Rationale:** compatible with the latest CUDA 13.x (I-5) while the
  shipping target stays 12.6.
- **Kind:** AC31-03a fork-backed; AC31-03b fork-backed and product.
- **AC31-03a:** cudarc CF-AC-04a (fork CI receipt
  `2026-10-12-CI-b777f8d.md`), CF-AC-04b (G3 receipts
  `2026-10-12-G3-<v>-b777f8d.md`) and CF-AC-04c (G5b receipts
  `2026-10-12-G5b-<v>-b777f8d.md`, GCP T4 with a newer driver installed on
  the lab VM only, cudarc ruling Q-F1) at `C`. G3 and G5b are the runtime
  evidence for the 13.1+ builds, including the 13.2+ `CUmemLocation` union
  path; FathomDB does not run them (I-18).
- **AC31-03b:**
  - fork-backed: Candle CK-AC-05 at `K`, whose 13.0 toolkit cell K-G2 ran
    on a GCP T4 with driver 580.178.04 (receipt
    `2026-10-12-K-G2-853324fc.md`);
  - product: T-WS-BUILD and T-CUDA cells for 12.6 on `windchill3`, and for
    13.0 on a GCP VM with driver branch 580 and the 13.0 toolkit installed
    on the VM only (Q15). If the owner declines that VM session, the 13.0
    cells are UNMEASURED by owner decision;
  - workspace runtime for 13.1–13.4 is UNMEASURED by owner ruling Q4.

### REQ31-04 Jetson Tegra

- **Statement:** every Slice 30 gate holds on the Orin (JetPack 6, CUDA
  12.6) on the new base. Tegra on CUDA 13 is UNMEASURED in Slice 31 (Q5).
- **Rationale:** the pool exists for Jetson (I-13d); Slice 30 is the
  baseline not to regress (R30-12).
- **Kind:** fork-backed and product.
- **AC31-04:**
  - fork-backed: cudarc CF-AC-05b at `C` (G6 receipt
    `2026-10-12-G6-b777f8d.md`); Candle CK-AC-05's Orin cell K-G3 at `K`
    (`2026-10-12-K-G1-K-G3-853324fc.md`), re-run at `K8` by CK-AC-07;
  - product: `qualification.md` re-runs the Slice 30 gate rows on the
    Orin; `doctor cuda-allocator` reports `private`, 3 GiB;
  - the Tegra/CUDA 13 row reads UNMEASURED, citing Q5 and todo
    `TC-b267d940`.

### REQ31-05 Fallback

- **Statement:** items 1–4 (the allocator fallback) act only on aarch64
  Linux.
- **Rationale:** other targets keep upstream's allocator logic.
- **Kind:** fork-backed.
- **AC31-05:** cudarc CF-AC-05 at `C`, verified by R-FC-VERIFY.

### REQ31-06 Vendor excellence

- **Statement:** upstream APIs are used (result-layer `mem_pool`, #544)
  instead of new bindings; fork deltas are minimal; every item has its
  upstream status and removal path in each fork's `FATHOMDB.md`; fork revs
  are never rewritten after a pin; library allocations outside the pool
  are known (D-14).
- **Rationale:** forks are kept removable and honest about their delta
  (I-13b).
- **Kind:** fork-backed and product.
- **AC31-06:** cudarc CF-AC-06 at `C`; Candle CK-AC-03 and CK-AC-06 at `K`
  (fork-backed); R-DES records D-10 and D-14; the pinned-override gate
  (`scripts/check-pinned-override-rot.py` with
  `scripts/pinned-override-rot.json` updated for the registry pins and the
  `candle-kernels` git patch) passes.

### REQ31-07 One cudarc

- **Statement:** the dependency graph has exactly one cudarc 0.19: the
  registry `cudarc-fathomdb` 0.19.10.
- **Rationale:** two copies split CUDA contexts and types between
  FathomDB and Candle (plan § 14).
- **Kind:** product and fork-backed.
- **AC31-07:**
  - the single-cudarc guard passes on the slice branch;
  - its tests turn it red for each of: a registry `cudarc 0.19.*` node on
    the normal tree; a second `cudarc-fathomdb` source (a git source next
    to the registry one, or a second version);
  - `Cargo.lock` lists exactly one `cudarc-fathomdb` package, with the
    crates.io registry source;
  - Candle CK-AC-01 at `K`, and at `K8` through CK-AC-07 (one
    `cudarc-fathomdb v0.19.10` node per crate, registry form).

### REQ31-08 crates.io

- **Statement:** crates.io consumers of `fathomdb` get the patched cudarc:
  `tegra-pool` builds from the registry with no patch table.
- **Rationale:** the end state of D28-13's consumption plan (Q9).
- **Kind:** post-publication.
- **AC31-08:**
  - **cudarc half (evidenced 2026-10-13):** the published
    `cudarc-fathomdb` 0.19.10 `.cargo_vcs_info.json` sha1 equals `C`.
    Evidence: cudarc fork receipt
    `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md` (fork
    commit `9b0ef75`), with CF-AC-07b
    (`2026-10-13-CF-AC-07b-b777f8d.md`). R-CODE-4 re-checks it from the
    registry.
  - **Candle half (after the 0.10.4 publication):** the published
    `candle-core-fathomdb`, `candle-nn-fathomdb` and
    `candle-transformers-fathomdb` 0.10.4 `.cargo_vcs_info.json` sha1
    equals `K8`, re-qualified by Candle CK-AC-07 and approved by R-FK-PUB.
  - **Registry build (T-REGISTRY):**
    `scripts/check-tegra-pool-registry-build.sh`, changed RED then GREEN to
    require that a consumer with no patch table builds `tegra-pool`, passes,
    and the Orin GPU smoke passes.

### REQ31-09 Proposals

- **Statement:** (1) the pool primitive only, on `proposal/mem-pool` from
  upstream `main`, upstream CI clean, tests skip without a device, docs,
  example and benchmark, issue and PR drafts, coexisting with #594;
  (2) built-major library loading with the loaded-library report, on
  `proposal/lib-major` from upstream `main`, independent of (1). Neither
  is posted.
- **Rationale:** a tidy upstream path for the fork's deltas (I-13c).
- **Kind:** post-merge, fork-backed. **Held by owner direction (Q17):**
  FC9 and FC9b are not started until the owner releases them; the
  proposal branches carry no FathomDB material.
- **AC31-09:** cudarc CF-AC-08 and CF-AC-08b; R-PROP and R-PROP2 approve.

### REQ31-10 FathomDB product

- **Statement:** `tegra-pool`, the typed errors (`cuda_pool_exhausted`,
  `cuda_context_lost`), `doctor cuda-allocator` and the early-`cuInit`
  contract are unchanged; the ADR is amended for the registry consumption
  and for fork-held evidence; docs and changelog are updated; no reference
  to the vendored tree or its test script remains.
- **Rationale:** the slice replaces a dependency; product behaviour must
  not change (R30-02 to R30-07, R30-11).
- **Kind:** product.
- **AC31-10:**
  - full `./scripts/agent-verify.sh` green in the slice worktree with its
    own `.venv` (plan § 9 W0 step 6);
  - the Slice 30 product tests unchanged;
  - `git grep -il -e third_party/cudarc -e test_vendored_cudarc -e 'vendored cudarc' -e 'cudarc 0.19.7' -- . ':!dev/plans' ':!dev/adr' ':!dev/todos-and-considerations-ledger.jsonl' ':!CHANGELOG.md'`
    is empty (plans, ADRs, the ledger and the changelog are history);
  - the ADR amendment marks every vendored-tree statement, including
    `blast_radius` (`ADR-0.8.28-tegra-private-cuda-pool.md:6, 101, 142`),
    as superseded; the changelog entry is present.

### REQ31-11 Process

- **Statement:** FathomDB steps are TDD with recorded red and green shas;
  every review is an Opus subagent and recorded.
- **Rationale:** I-8, I-9, I-11, I-12.
- **Kind:** product and fork-backed.
- **AC31-11:** `status.md` lists red and green shas per FathomDB step,
  each red re-run by the reviewer; `review.md` records every FathomDB
  review with model `opus` and every fork review by link; cudarc CF-AC-09
  and Candle CK-AC-08 pass.

### REQ31-12 Scope safety

- **Statement:** nothing is posted or pushed upstream; no PR or issue
  object exists on either fork; nothing from this slice lands on
  `release/0.8.27`; `windchill3`'s driver and `/usr/local/cuda` never
  change.
- **Rationale:** owner rulings Q4, Q6, Q11 and the push policy (plan § 1).
- **Kind:** product and fork-backed; checked so far at R-VERIFY, final at
  R-CODE-4.
- **AC31-12:** no upstream issue, comment, PR or push; no PR or issue
  object on either fork; no commit from this slice on `release/0.8.27`;
  `windchill3` driver 580.178.04 and `/usr/local/cuda` → 12.6 unchanged in
  every guard log, FathomDB and fork (cudarc CF-AC-10, Candle CK-AC-08).

### REQ31-13 Performance

- **Statement:** no performance regression from the rebase.
- **Rationale:** the pool exists to avoid the synchronous path's latency
  (Slice 30 ruling 25).
- **Kind:** product.
- **AC31-13:** Slice 30 gate G9 in full (`slice-30/plan.md` § 4.2) re-run
  on the Orin against the Slice 30 product:
  - Node ratio ≤ 1.05 per measure with bootstrap 95 % CIs (if the CI
    upper bound exceeds 1.10, double N once);
  - speed-up over synchronous > 1.5 (CI lower bound);
  - import ≤ S + 5 ms; RSS ≤ S + 32 MiB;
  - Python P / S ≤ 1.15;
  - RTX 3090: median ratio new/0.19.7 ≤ 1.05 for embed steady state.

  The proposal benchmark is the cudarc fork's T-PERF (CF-REQ-08), not part
  of this criterion.

### REQ31-14 Fork management

- **Statement:** fork work is planned, coded, reviewed and qualified in
  each fork (plan § 3.1); FathomDB consumes only qualified code commits;
  the pin chain is consistent; published crates contain no fork
  engineering records.
- **Rationale:** I-17; prevents evidence drift between fork and product
  (plan § 14).
- **Kind:** fork-backed and product.
- **AC31-14:**
  - both fork plans reviewed (R-FC-PLAN, R-FK-PLAN) and every fork review
    recorded with no open finding;
  - `C` is the cudarc commit pinned by the Candle fork at `K`, and the
    commit named by the registry `cudarc-fathomdb` 0.19.10 that `K8` and
    FathomDB depend on;
  - `git diff C <cudarc tip> -- . ':(exclude)fathomdb'` and
    `git diff K <Candle tip> -- . ':(exclude)fathomdb' ':(exclude)FATHOMDB.md'`
    are empty, or the tips were re-qualified (for `K8`: CK-AC-07);
  - the commit-class checks pass (cudarc fork plan § 1.3 over
    `ec1cac71..`, Candle fork plan § 1 over `859b8ea1..`);
  - cudarc CF-AC-07a at `C` shows no `fathomdb/` path in the package
    (`package-list-b777f8d.txt`), and CF-AC-07b equals it;
  - a fork CI receipt has `head_sha` = `C` (`2026-10-12-CI-b777f8d.md`);
  - FathomDB's pins pass R-PIN's checklist (plan § 9 W6 step 2): registry
    versions as stated in § 1, one registry `cudarc-fathomdb` in
    `Cargo.lock`, `scripts/pinned-override-rot.json` updated in the same
    commit, the `candle-kernels` patch `rev` equal to the qualified Candle
    commit.

### REQ31-15 CUDA version support

- **Statement** (`ADR-0.8.28-cuda-version-support-policy.md`): the 12.6
  artifact works with any CUDA 12.x cuBLAS from 12.1 to 12.9 on driver 560
  or later and Turing or newer GPUs; it never loads a cuBLAS of another
  major; an environment with no CUDA 12 cuBLAS, or an unsupported GPU,
  gets a typed, plain error; the user docs state the floors.
- **Rationale:** owner rulings Q10 and Q12; a cross-major load fails late
  and obscurely.
- **Kind:** fork-backed and product.
- **AC31-15:**
  - T-CUDA-COMPAT C1–C5 pass or are UNMEASURED by owner decision;
  - cudarc CF-AC-11 (CF-REQ-11, built-major library loading) at `C`;
  - D-18's production check and report pass their RED-first tests;
  - `docs/embedder.md` states the CUDA library floor (12.x), the driver
    floor (560.28.03 for the 12.6 artifact) and the GPU floor (Turing,
    compute capability 7.5, on x86; Orin on Jetson).

## 4. Acceptance sets per review

| Review | Criteria | State (2026-10-13) |
| --- | --- | --- |
| R-PIN (owner, approved, Q20) | the AC31-14 pin checklist, per pin commit | W6 |
| R-VERIFY (with R-CODE-3, before merge) | AC31-01…07, 10, 11, 12 (so far), 13, 14, 15, with T-FORK-EVIDENCE | W6; the GREEN pins wait for the Candle 0.10.4 publication |
| R-PROP, R-PROP2 (after merge) | AC31-09 | held by owner direction (Q17) |
| R-CODE-4 (after publication) | AC31-08, AC31-12 (final), T-REGISTRY | AC31-08 cudarc half evidenced (FC10 receipt); Candle half after the 0.10.4 publication |

## 5. Traceability

Test-plan topics are plan § 7's names, expanded in `test-plan.md` (W3).

| Requirement | Criterion | Fork criteria | Other sources | Test-plan topic | Review |
| --- | --- | --- | --- | --- | --- |
| REQ31-01 | AC31-01 | CF-AC-01 | D28-13 | T-FORK-EVIDENCE | R-VERIFY |
| REQ31-02 | AC31-02 | CF-AC-02, CF-AC-03, CK-AC-02 | R30-01 | T-FORK-EVIDENCE, T-POOL-PRODUCT, T-CUDA | R-VERIFY |
| REQ31-03 | AC31-03a, AC31-03b | CF-AC-04a, CF-AC-04b, CF-AC-04c, CK-AC-05 | I-5 | T-FORK-EVIDENCE, T-WS-BUILD, T-CUDA | R-VERIFY |
| REQ31-04 | AC31-04 | CF-AC-05b, CK-AC-05 (K-G3), CK-AC-07 | R30-12 | T-ORIN, T-POOL-PRODUCT, T-CUDA | R-VERIFY |
| REQ31-05 | AC31-05 | CF-AC-05 | — | T-FORK-EVIDENCE | R-VERIFY |
| REQ31-06 | AC31-06 | CF-AC-06, CK-AC-03, CK-AC-06 | R30-09; REQ-068 (see § 7 item 7) | T-FORK-EVIDENCE, T-PRODUCT | R-DES, R-VERIFY |
| REQ31-07 | AC31-07 | CK-AC-01, CK-AC-07 | — | T-PRODUCT | R-PIN, R-VERIFY |
| REQ31-08 | AC31-08 | CF-AC-07b, CK-AC-07 | D28-13, R30-07 | T-REGISTRY | R-FC-PUB, R-FK-PUB, R-CODE-4 |
| REQ31-09 | AC31-09 | CF-AC-08, CF-AC-08b | — | (fork tests) | R-PROP, R-PROP2 |
| REQ31-10 | AC31-10 | — | R30-02…07, R30-11 | T-PRODUCT, T-TEARDOWN-PRODUCT | R-CODE-3, R-VERIFY |
| REQ31-11 | AC31-11 | CF-AC-09, CK-AC-08 | — | (records) | R-VERIFY |
| REQ31-12 | AC31-12 | CF-AC-10, CK-AC-08 | — | (guard logs) | R-VERIFY, R-CODE-4 |
| REQ31-13 | AC31-13 | — | Slice 30 G9 | T-PERF | R-VERIFY |
| REQ31-14 | AC31-14 | CF-AC-04a, CF-AC-07a, CF-AC-07b, CF-AC-09, CK-AC-07, CK-AC-08 | I-17 | T-FORK-EVIDENCE | R-PIN, R-VERIFY |
| REQ31-15 | AC31-15 | CF-AC-11 | Q10, Q12 | T-CUDA-COMPAT | R-DES, R-VERIFY |

## 6. Changes from plan § 5

- AC31-02, 04, 07 and 14 name the registry pins (`cudarc-fathomdb`
  `=0.19.10`, Candle `=0.10.4`) where § 5 names `C`/`K` git pins, and add
  `K8` through CK-AC-07.
- AC31-07's second-source test is a git `cudarc-fathomdb` next to the
  registry one, or a second version, instead of a different URL or `rev`.
- AC31-08 is split into the cudarc half (evidenced) and the Candle half.
- AC31-14 adds the R-PIN checklist (Q20).
- AC31-15 names CF-AC-11, the criterion of CF-REQ-11.
- REQ31-07, 11 and 12 had no statement in § 5; each now states its
  criterion's intent, adding nothing to it.
- REQ31-09 records the owner's hold (Q17).

## 7. Open points for R-REQ

1. **`candle-kernels` at `K8` or `K`** (W2, D-10/D-11). The tree is
   identical, and `candle-kernels/Cargo.toml` inherits nothing from the
   workspace. Recommendation: `K8`, one commit for the whole Candle
   cohort and the commit the crates are published from; record the empty
   diff as the reason K-G2 (13.0, run at `K` only) still applies.
2. **K-G2 at `K8`.** CK-AC-07 re-runs K-G1 and K-G3 at `K8` but not
   K-G2. AC31-03b relies on K-G2 at `K` carrying to `K8` because
   `K..K8` changes only versions and the dependency source, and the
   registry crate's files equal `C`'s. R-REQ should accept or reject
   that carry.
3. **The D-16 lock-drift guard has no criterion.** Q8 rules it in, W6
   adds it and plan § 6 D-16 specifies its tests, but no AC31 names it.
   Candidates: AC31-07 or AC31-14.
4. **T-TEARDOWN-PRODUCT (D-15) has no explicit criterion.** It is traced
   here to AC31-10 ("product unchanged"); R-REQ may prefer AC31-04.
5. **When the registry-build check runs.** Both publications now precede
   W6, so `scripts/check-tegra-pool-registry-build.sh` (AC31-08,
   T-REGISTRY) could move into W6. AC30-07's expectation (a registry
   build of `tegra-pool` fails with the marker message) is inverted by
   that change, so AC31-10's "Slice 30 product tests unchanged" needs
   an explicit exception for it wherever the change lands. Plan § 9 W8
   leaves the decision to W3.
6. **The marker feature's future** (D-11). With
   `package = "cudarc-fathomdb"`, the dependency can no longer resolve to
   upstream `cudarc`, so `fathomdb-private-pool` no longer guards
   anything; the published crate still declares it. No criterion depends
   on the outcome.
7. **REQ-068 (CUDA-capable Linux artifacts stay CPU-loadable) has no
   AC31.** It rests on the static CUDART in the forked `candle-kernels`
   (Candle CK-AC-04, re-run at `K8` by CK-AC-07), which reaches workspace
   builds only through the `candle-kernels` git patch (Q2). It is traced
   here to REQ31-06 (D-10); R-REQ may name CK-AC-04 in AC31-06.
