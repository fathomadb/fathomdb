---
title: FathomDB 0.8.28 Slice 31 — requirements and acceptance criteria (W1)
status: REVISED AFTER R-REQ (APPROVE-WITH-FIXES); ready for W2
target_release: 0.8.28
observed_on: 2026-10-13
---

# Slice 31 requirements and acceptance criteria

This is the final form of REQ31 and AC31, written in W1 from
[plan.md](plan.md) § 5 (revision 16), reviewed by R-REQ (Opus,
APPROVE-WITH-FIXES, [review.md](review.md)) and revised for its findings
and for plan revision 17 (the Candle crates 0.11.0 from `K9`, Q21). It
governs where it differs from plan § 5. It adds no requirement to § 5
except REQ31-16, which gives REQ-068 the criterion R-REQ asked for;
[§ 7](#7-r-req-rulings-on-the-open-points) records R-REQ's rulings on the
W1 open points.

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
| `K8` | Candle `cd64715ead4f86470b195a9010c4f60f7a4c0833` on the same branch (record `6b350638`): `K` with the three crates at 0.10.4 on registry `cudarc-fathomdb` 0.19.10 | **Passed** CK-AC-07 (fork receipt `fathomdb/qualification/2026-10-13-CK-AC-07-cd64715e.md`, fork commit `ddff4cbd`); R-FK-PUB approved it apart from finding 1; **superseded for publication by `K9`** (Q21). Its receipt stands as evidence for the same code |
| `K9` | Candle `182ceab0547348993b6dbec4b1aed610dcc8b23f` on the same branch (record `4abb8b38`): `K8` with the three crates at 0.11.0 and a fork `description`; its `Cargo.lock` differs from `K8`'s only in the three version lines | the publication commit (Q21); CK-AC-07 cells at `K9` (control `K8`) running; R-FK-PUB re-check and publication follow (Q18, Q21) |
| `cudarc-fathomdb` | crates.io 0.19.10, published 2026-10-13 14:02Z from `C` | registry `.crate` SHA-256 `8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2` equals the dry-run `.crate`; `.cargo_vcs_info.json` sha1 = `C`; all 131 source files match `C` |
| `candle-core-fathomdb`, `candle-nn-fathomdb`, `candle-transformers-fathomdb` | crates.io 0.11.0, from `K9` (Q21) | not yet published; `K9` dry-run `.crate` SHA-256s in AC31-08 |
| `candle-kernels` | git patch on `https://github.com/coreyt/candle-fathomdb.git`, full `rev` of `K9`; version 0.10.2 | `git diff 853324fc 182ceab0 -- candle-kernels` is empty |

FathomDB's W6 pins (plan § 9 W6 step 2): the embedder's aarch64-Linux
`cudarc = { package = "cudarc-fathomdb", version = "=0.19.10", … }`, the
Candle crates at `=0.11.0`, and a root `[patch.crates-io]` holding only
`candle-kernels` (git, `rev` = `K9`) and `libsqlite3-sys` (path). Exact
versus caret for `cudarc-fathomdb` is confirmed by D-11 in W2.

## 2. How criteria are judged

- **Fork-backed.** A fork-backed criterion passes when the fork's named
  criterion is recorded passing at the pinned commit, with that fork's
  R-…-VERIFY approving it and no open finding. FathomDB does not re-run
  the fork's tests (I-18). Evidence is the fork's `fathomdb/review.md`
  verdict and the receipts in its `fathomdb/qualification/`, linked by
  permalink at the receipt commit's sha in `status.md`.
- **Registry crates carry the fork evidence** when the published crate's
  `.cargo_vcs_info.json` names the qualified commit and its packaged files
  equal that commit's (for `cudarc-fathomdb`: the FC10 receipt). For `K9`,
  Candle CK-AC-07 (at `K9`, control `K8`) is the re-qualification.
- **Product.** A product criterion passes on FathomDB's own tests and
  receipts (`status.md`, `qualification.md`, CI logs), each test named
  with a nonzero run count, never only a pass summary.
- **Post-publication.** Checked at R-CODE-4, after the owner publishes.
- **UNMEASURED by owner decision** is a pass state only where an owner
  ruling is cited (Q4, Q5, Q13, Q15); R-VERIFY confirms each such cell.
  Any owner action the owner declines is recorded in `status.md` and with
  `ledgerwrite` (`GOAL.md`, "If the owner declines an owner action"),
  and the dependent cells are marked UNMEASURED by owner decision.

## 3. Requirements

Each entry: statement, rationale, kind, acceptance criterion and its
evidence. "Fork receipts" means the fork's `fathomdb/qualification/` at the
named commit. `<slice base>` is the `release/0.8.28` commit last merged
into the slice branch (`git merge-base origin/release/0.8.28 <tip>`).

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
    through Candle), carried to `K8` and `K9` by CK-AC-07;
  - product (T-POOL-PRODUCT): on the Orin (CUDA 12.6), against the W6
    pins (registry `cudarc-fathomdb` 0.19.10, Candle 0.11.0, the
    `candle-kernels` patch at `K9`), with `FATHOMDB_REQUIRE_LIVE=1` so a
    skip fails, the four Slice 30 Orin GPU tests of `fathomdb-embedder`
    each pass with a nonzero run count:
    - `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable`
      (`tests/tegra_fragmented_va_cuda.rs`);
    - `gpu_orin_64_builds_every_device_on_a_three_gib_private_pool`,
      `gpu_orin_64_pool_stays_under_max_size_and_is_empty_after_the_embedder_drops`
      and
      `gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults`
      (`src/cuda_pool_policy_tests.rs`, `#[ignore]`, run with
      `--ignored`; the last runs `gpu_orin_64_context_reset_child`);
  - product: the Slice 30 product tests listed in AC31-10 pass in the full
    gate;
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
  `2026-10-12-G3-<v>-b777f8d.md`) and CF-AC-04c at `C`: the G5a receipts
  `2026-10-12-G5a-<v>-b29c41c.md` (at `b29c41c`, carried to `C` by
  R-FC-VERIFY) and the G5b receipts `2026-10-12-G5b-<v>-b777f8d.md` (GCP
  T4 with a newer driver installed on the lab VM only, cudarc ruling
  Q-F1). G3, G5a and G5b are the runtime evidence for the 13.1+ builds,
  including the 13.2+ `CUmemLocation` union path; FathomDB does not run
  them (I-18).
- **AC31-03b:**
  - fork-backed: Candle CK-AC-05 at `K`, whose 13.0 toolkit cell K-G2 ran
    on a GCP T4 with driver 580.178.04 (receipt
    `2026-10-12-K-G2-853324fc.md`). K-G2 carries to `K8` and `K9`:
    `git diff 853324fc 182ceab0 -- . ':(exclude)fathomdb' ':(exclude)FATHOMDB.md'`
    changes only the root `Cargo.toml` (the three fork crates' workspace
    versions, and `cudarc-fathomdb` from git `rev` `C` to registry
    0.19.10, whose files equal `C`'s, FC10 receipt) and the three crates'
    `version` and `description` lines; `candle-kernels` is unchanged. If
    the owner approves the 13.0 VM session, T-WS-BUILD 13000 exercises the
    registry form and replaces the carry;
  - product: T-WS-BUILD and T-CUDA cells for 12.6 on `windchill3`, and for
    13.0 on a GCP VM with driver branch 580 and the 13.0 toolkit installed
    on the VM only (Q15). If the owner declines that VM session, the 13.0
    cells are UNMEASURED by owner decision;
  - workspace runtime for 13.1–13.4 is UNMEASURED by owner ruling Q4.

### REQ31-04 Jetson Tegra

- **Statement:** every Slice 30 gate holds on the Orin (JetPack 6, CUDA
  12.6) on the new base, and so does the product's teardown and process
  lifecycle (D-15). Tegra on CUDA 13 is UNMEASURED in Slice 31 (Q5).
- **Rationale:** the pool exists for Jetson (I-13d); Slice 30 is the
  baseline not to regress (R30-12).
- **Kind:** fork-backed and product.
- **AC31-04:**
  - fork-backed: cudarc CF-AC-05b at `C` (G6 receipt
    `2026-10-12-G6-b777f8d.md`); Candle CK-AC-05's Orin cell K-G3 at `K`
    (`2026-10-12-K-G1-K-G3-853324fc.md`), re-run at `K8` and at `K9` by
    CK-AC-07;
  - product: `qualification.md` re-runs the Slice 30 gate rows G1–G10
    (`slice-30/plan.md` § 4.2; G9 per AC31-13) on the Orin; `doctor
    cuda-allocator` reports `private`, 3 GiB;
  - product (T-TEARDOWN-PRODUCT, D-15) on the Orin, each a named test with
    a nonzero run count:
    - engine close and process exit with the pool alive end cleanly (exit
      status 0, no signal); the test names are fixed in `test-plan.md`
      (W3);
    - a lost context reports `cuda_context_lost` on each following embed
      without SIGSEGV
      (`gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults`).
      That test also asserts Slice 30's measured SIGSEGV at teardown after
      the loss (`cublasDestroy_v2`, fix planned for 0.8.29); that
      assertion stands unless R-DES's D-15 fixes it in this slice, in
      which case it flips RED first;
    - the fork-after-init cases R-DES fixes in `design.md` (D-15), each
      with its named test;
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
- **AC31-06:**
  - fork-backed: cudarc CF-AC-06 at `C`; Candle CK-AC-03 and CK-AC-06 at
    `K`;
  - R-DES records D-10 and D-14; the D-14 measurements of allocations
    outside the pool (per-process device memory as VmRSS on the Orin,
    with default-pool counters as a leak check; plan § 6 D-14) are in
    `qualification.md`;
  - the pinned-override gate (`scripts/check-pinned-override-rot.py` with
    `scripts/pinned-override-rot.json` updated for the registry pins and
    the `candle-kernels` git patch at `K9`) passes;
  - no comment or document claims that the `fathomdb-private-pool` marker
    feature guards against registry `cudarc` (with
    `package = "cudarc-fathomdb"` it cannot resolve to upstream `cudarc`;
    its future is D-11's).

### REQ31-07 One cudarc

- **Statement:** the dependency graph has exactly one cudarc 0.19: the
  registry `cudarc-fathomdb` 0.19.10.
- **Rationale:** two copies split CUDA contexts and types between
  FathomDB and Candle (plan § 14).
- **Kind:** product and fork-backed.
- **AC31-07:**
  - the single-cudarc guard, written in W6 as `scripts/check-single-cudarc.sh`
    (no such guard exists today; W3 may rename it), runs in the fast tier
    (`run_tier_suite fast` in `scripts/agent-test.sh`) and passes on the
    slice branch;
  - it fails when any of these holds:
    - `Cargo.lock` has any package named `cudarc` with a 0.19.* version;
    - `Cargo.lock` has more than one `cudarc-fathomdb` package, or a
      `cudarc-fathomdb` package whose source is not the crates.io
      registry (for example a git source next to the registry one, or a
      second version);
    - for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`,
      `cargo tree -e normal --target <triple> -p fathomdb-embedder` with
      any of the features `embed-cuda`, `rerank-cuda`, `tegra-pool` has a
      `cudarc v0.19` node. The `ug` and `flash-attn` chains are excluded,
      as in Candle CK-AC-01 (FathomDB enables neither; `flash-attn` pulls
      upstream `candle-core` 0.10.2 and a second cudarc, R-FK-PUB
      finding 5, and is unsupported);
  - each failure condition has a red test whose input is a committed
    fixture lock (with its fixture manifest, for the tree condition);
  - `Cargo.lock` lists exactly one `cudarc-fathomdb` package, with the
    crates.io registry source;
  - Candle CK-AC-01 at `K`, and at `K8` and `K9` through CK-AC-07 (one
    `cudarc-fathomdb v0.19.10` node per crate, registry form).

### REQ31-08 crates.io

- **Statement:** crates.io consumers of `fathomdb` get the patched cudarc:
  `tegra-pool` builds from the registry with no patch table.
- **Rationale:** the end state of D28-13's consumption plan (Q9).
- **Kind:** product, run in W6 and at R-VERIFY; re-checked from the
  registry at R-CODE-4.
- **AC31-08:**
  - **cudarc half (evidenced 2026-10-13):** the published
    `cudarc-fathomdb` 0.19.10 `.cargo_vcs_info.json` sha1 equals `C`.
    Evidence: cudarc fork receipt
    `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md` (fork
    commit `9b0ef75`), with CF-AC-07b
    (`2026-10-13-CF-AC-07b-b777f8d.md`). R-CODE-4 re-checks it from the
    registry.
  - **Candle half (after the 0.11.0 publication):** for each of
    `candle-core-fathomdb`, `candle-nn-fathomdb` and
    `candle-transformers-fathomdb` 0.11.0, downloaded from crates.io:
    - every packaged source file equals `git show K9:<path>` (the robust
      check);
    - `.cargo_vcs_info.json` has sha1 = `K9` and no `dirty` key;
    - the `.crate` SHA-256 equals the final R-FK-PUB dry-run `.crate`.
      The `K9` dry-run SHA-256s are: core
      `b5feacc063f44c77a54cb8b43d03c5e32fa5421f4163f0524194783e21563341`,
      nn `69c7432aa63cb543065f49453fa2e258e4dc3bd458abbcb7c150d17e90026200`,
      transformers
      `513008c79a9751ec5fdc08918dcdb1a8872c95e90f7697a9f8c5ef1951e3d222`.
      These match only while the crates.io index is unchanged between the
      dry run and the publication, as the cudarc FC10 receipt explains; a
      mismatch with the source-file check passing is recorded, not failed;
    - re-qualified by Candle CK-AC-07 at `K9` and approved by R-FK-PUB.
  - **Registry build (T-REGISTRY, in W6):** a scratch consumer with **no
    `[patch]` table** depends on `fathomdb-embedder` by path
    (`scripts/check-tegra-pool-registry-build.sh`, changed RED then GREEN
    in W6):
    - (a) its `Cargo.lock` resolves `cudarc-fathomdb` 0.19.10, the three
      Candle crates 0.11.0 and upstream `candle-kernels` 0.10.2, all from
      the crates.io registry, and no `cudarc` 0.19.* package;
    - (b) on the Orin, the consumer with `embed-cuda,tegra-pool` builds
      and links (`cargo build`, and the link of (c)'s test binary) with
      `LIBRARY_PATH` set to the CUDA library directory
      (`/usr/local/cuda/lib64` on the Orin), as R-FK-PUB finding 2
      requires; the receipt states the value;
    - (c) `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable`
      runs against that resolution with `FATHOMDB_REQUIRE_LIVE=1` and
      passes with a nonzero run count (`test-plan.md` fixes the
      invocation);
    - (d) the receipt and `docs/embedder.md` state that crates.io
      consumers get dynamic `libcudart` through upstream `candle-kernels`
      0.10.2 (no K4/K6 static CUDART); REQ-068 is claimed only for
      FathomDB-built artifacts (Q2, REQ31-16).

    The script's current line 102 ("`[patch.crates-io]` must carry
    exactly one cudarc entry") breaks as soon as W6 removes the cudarc
    patch entry, so the change lands in W6. It inverts Slice 30's AC30-07
    (a registry `tegra-pool` build failed with the marker message); that is
    AC31-10's named exception. R-CODE-4 keeps AC31-12 final and re-checks
    this criterion from the registry.

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
  - the Slice 30 product tests are unchanged:
    `git diff <slice base>..<tip> -- <files>` is empty for
    - `src/rust/crates/fathomdb-embedder/tests/tegra_fragmented_va_cuda.rs`
    - `src/rust/crates/fathomdb-embedder/src/cuda_pool_policy_tests.rs`
    - `src/rust/crates/fathomdb-embedder/tests/slice30_cuda_allocator_api.rs`
    - `src/rust/crates/fathomdb-engine/tests/slice30_cuda_error_kinds.rs`
    - `src/rust/crates/fathomdb-engine/src/rerank_tests.rs`
    - `src/rust/crates/fathomdb-sdk/tests/cuda_errors.rs`
    - `src/python/tests/test_tegra_pool_early_init.py`
    - `src/python/tests/test_typed_errors.py`
    - `src/ts/tests/typed-errors.test.ts`

    with these named exceptions, each listed with its diff in
    `status.md`: `scripts/check-tegra-pool-registry-build.sh` (AC30-07
    inverted, AC31-08); `scripts/tests/test_check_pinned_override_rot.sh`
    (the pinned-override test, for the registry pins); a rename in test
    code forced by the fork API (D-17) that changes no assertion; and the
    teardown assertion of AC31-04, only if R-DES's D-15 fixes it;
  - `git grep -il -e third_party/cudarc -e test_vendored_cudarc -e 'vendored cudarc' -e 'cudarc 0.19.7' -- . ':!dev/plans' ':!dev/adr' ':!dev/todos-and-considerations-ledger.jsonl' ':!CHANGELOG.md'`
    is empty (plans, ADRs, the ledger and the changelog are history);
  - the ADR amendment marks every vendored-tree statement, including
    `blast_radius` (`ADR-0.8.28-tegra-private-cuda-pool.md:6, 101, 142`),
    as superseded; the changelog entry is present.

### REQ31-11 Process

- **Statement:** FathomDB steps are TDD with recorded red and green shas;
  every review is an Opus subagent and recorded; neither the steward nor
  an orchestrator is used.
- **Rationale:** I-8, I-9, I-10, I-11, I-12.
- **Kind:** product and fork-backed.
- **AC31-11:**
  - `status.md` lists red and green shas per FathomDB step, each red
    re-run by the reviewer;
  - `review.md` records every FathomDB review with model `opus` and every
    fork review by link; R-REQ, R-DES and R-TEST are recorded there before
    the first W6 commit (Q19);
  - no steward or orchestrator agent is spawned (I-10): every agent named
    in `status.md` and `review.md` is an implementer, research, review or
    Orin agent (plan § 11);
  - cudarc CF-AC-09 and Candle CK-AC-08 pass.

### REQ31-12 Scope safety

- **Statement:** nothing is posted or pushed upstream; no PR or issue
  object exists on either fork; nothing from this slice lands on
  `release/0.8.27`; `windchill3`'s driver and `/usr/local/cuda` never
  change.
- **Rationale:** owner rulings Q4, Q6, Q11 and the push policy (plan § 1).
- **Kind:** product and fork-backed; checked so far at R-VERIFY, final at
  R-CODE-4.
- **AC31-12:**
  - `gh pr list -R coreyt/cudarc --state all`,
    `gh issue list -R coreyt/cudarc --state all`,
    `gh pr list -R coreyt/candle-fathomdb --state all` and
    `gh issue list -R coreyt/candle-fathomdb --state all` each print
    nothing;
  - no upstream issue, comment, PR or push from this slice;
  - in each fork clone, `git remote get-url --push upstream` is the
    disabled placeholder (`DISABLED-never-push-upstream`), not an
    upstream URL;
  - for each commit of this slice (FathomDB slice and post branches),
    `git merge-base --is-ancestor <sha> origin/release/0.8.27` is false;
  - `windchill3` driver 580.178.04 and `/usr/local/cuda` → 12.6 unchanged
    in every guard log, FathomDB and fork (cudarc CF-AC-10, Candle
    CK-AC-08).

### REQ31-13 Performance

- **Statement:** no performance regression from the rebase.
- **Rationale:** the pool exists to avoid the synchronous path's latency
  (Slice 30 ruling 25).
- **Kind:** product.
- **AC31-13:** Slice 30 gate G9 in full (`slice-30/plan.md` § 4.2 row G9,
  with revision 3.3's rulings) re-run on the Orin:
  - **arms:** new = the slice tip with the W6 pins; baseline = the Slice
    30 product built from `<slice base>` (the pre-slice build); S = the
    same builds with `FATHOMDB_POOL_MODE=off`;
  - **design:** N = 20 processes per arm (Node 25 and Python), in
    interleaved randomised blocks; per process import time, 5 warm-up and
    50 timed steady embeds, rerank, batches 1/8/32/128; bootstrap 95 %
    CIs;
  - **Node:** ratio new / baseline ≤ 1.05 per measure, CIs reported (if a
    CI upper bound exceeds 1.10, double N once); the CI lower bound of the
    speed-up of new over S is above 1.5; import ≤ S + 5 ms; RSS ≤ S +
    32 MiB;
  - **Python:** P / S ≤ 1.15; import ≤ S + measured `cuInit` cost + 5 ms
    (Slice 30 SD-4);
  - **rerank speed-up:** measured and reported, non-gating (Slice 30
    revision 3.3);
  - **RTX 3090:** embed steady state, median ratio new / baseline ≤ 1.05,
    where the baseline is the pre-slice commit `<slice base>` (the
    vendored 0.19.7 tree is deleted by the slice); N = 20 processes per
    arm, interleaved, with a bootstrap 95 % CI reported.

  The proposal benchmark is the cudarc fork's T-PERF (CF-REQ-08), not part
  of this criterion.

### REQ31-14 Fork management

- **Statement:** fork work is planned, coded, reviewed and qualified in
  each fork (plan § 3.1); FathomDB consumes only qualified code commits;
  the pin chain is consistent; published crates contain no fork
  engineering records; lock drift between FathomDB and the cudarc fork is
  guarded (Q8).
- **Rationale:** I-17; prevents evidence drift between fork and product
  (plan § 14).
- **Kind:** fork-backed and product.
- **AC31-14:**
  - both fork plans reviewed (R-FC-PLAN, R-FK-PLAN) and every fork review
    recorded with no open finding;
  - `C` is the cudarc commit pinned by the Candle fork at `K`, and the
    commit named by the registry `cudarc-fathomdb` 0.19.10 that `K9` and
    FathomDB depend on;
  - pin-chain diffs:
    `git diff C <cudarc tip> -- . ':(exclude)fathomdb'` is empty or the
    tip was re-qualified (the registry crate is immutable, so this diff
    bears only on re-qualification of a later version);
    `git diff K9 <Candle tip> -- . ':(exclude)fathomdb' ':(exclude)FATHOMDB.md'`
    is empty or the tip was re-qualified (`K9` is both the publication
    commit and the `candle-kernels` `rev`);
  - no rewrite: `git merge-base --is-ancestor C origin/fathomdb/0.19.10`
    in the cudarc clone, and `git merge-base --is-ancestor <X>
    origin/fathomdb/0.8.28-cudarc-fork` for `X` = `K`, `K8` and `K9` in
    the Candle clone, each succeed;
  - the commit-class checks pass (cudarc fork plan § 1.3 over
    `ec1cac71..`, Candle fork plan § 1 over `859b8ea1..`);
  - cudarc CF-AC-07a at `C` shows no `fathomdb/` path in the package
    (`package-list-b777f8d.txt`), and CF-AC-07b equals it;
  - a fork CI receipt has `head_sha` = `C` (`2026-10-12-CI-b777f8d.md`);
  - FathomDB's pins pass R-PIN's checklist (plan § 9 W6 step 2): registry
    versions as stated in § 1, one registry `cudarc-fathomdb` in
    `Cargo.lock`, `scripts/pinned-override-rot.json` updated in the same
    commit, the `candle-kernels` patch `rev` equal to `K9`;
  - **D-16 lock-drift guard (Q8):** a fast-tier guard written in W6
    compares the resolved dependency set of `cudarc-fathomdb` in
    FathomDB's `Cargo.lock` (names, versions and checksums of its
    dependencies) with the set recorded by the cudarc fork's latest CI
    receipt run with FathomDB's lock. R-DES (D-16) fixes the guard's path
    and the record's format and location; the current receipt
    `2026-10-12-CI-b777f8d.md` states the run but not the set, so the fork
    records the set before the guard goes GREEN. Tests: RED, a fixture
    lock changing one dependency (for example `libloading`) with no
    matching fork record; GREEN, the W6 lock. If the W6 lock's set
    differs from the fork's record, the fork re-runs with it first.

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
  - T-CUDA-COMPAT C1, C2 and C4 pass. C3 (the CUDA-13-only Docker image,
    whose pull needs owner approval, Q13) and C5 (the 13.0 GCP VM session,
    Q15) pass or are UNMEASURED by owner decision; no other cell may be
    UNMEASURED;
  - cudarc CF-AC-11 (CF-REQ-11, built-major library loading) at `C`;
  - D-18's production check and report pass their RED-first tests;
  - `docs/embedder.md` states the CUDA library floor (12.x), the driver
    floor (560.28.03 for the 12.6 artifact) and the GPU floor (Turing,
    compute capability 7.5, on x86; Orin on Jetson).

### REQ31-16 CPU-loadable CUDA artifacts (REQ-068) through the candle-kernels patch

- **Statement:** FathomDB-built CUDA-capable Linux artifacts stay
  CPU-loadable (REQ-068): they link the CUDA runtime statically through
  the forked `candle-kernels` (K4, K6), which reaches workspace builds only
  through the `candle-kernels` git patch (Q2).
- **Rationale:** REQ-068 and `ADR-0.8.25-driverless-cuda-runtime-linkage.md`;
  the patch is the only path by which the fork's static CUDART reaches
  FathomDB's artifacts, and the registry `candle-kernels` 0.10.2 links
  `libcudart` dynamically (R-FK-PUB finding 2).
- **Kind:** fork-backed and product.
- **AC31-16:**
  - `Cargo.lock` has exactly one `candle-kernels` package, with source
    `git+https://github.com/coreyt/candle-fathomdb.git?rev=182ceab0547348993b6dbec4b1aed610dcc8b23f#182ceab0547348993b6dbec4b1aed610dcc8b23f`,
    and no registry `candle-kernels`;
  - fork-backed: Candle CK-AC-04 at `K`, and at `K8` and `K9` through the
    CK-AC-07 receipts;
  - product: on the artifacts built from the W6 pins,
    `scripts/release/inspect-cuda-artifacts.py` passes on the x86 Python
    wheel and NAPI tarball, and `scripts/release/check-tegra-wheel-linkage.sh`
    (the Tegra build scripts' linkage check) passes on the extension of
    the Tegra wheel and of the Tegra NAPI package.

## 4. Acceptance sets per review

| Review | Criteria | State (2026-10-13) |
| --- | --- | --- |
| R-PIN (owner, approved, Q20) | the AC31-14 pin checklist, per pin commit | W6 |
| R-VERIFY (with R-CODE-3, before merge) | AC31-01…08, 10, 11, 12 (so far), 13, 14, 15, 16, with T-FORK-EVIDENCE and T-REGISTRY | W6; the GREEN pins wait for the Candle 0.11.0 publication |
| R-PROP, R-PROP2 (after merge) | AC31-09 | held by owner direction (Q17) |
| R-CODE-4 (after publication) | AC31-08 re-checked from the registry, AC31-12 (final) | AC31-08 cudarc half evidenced (FC10 receipt); Candle half after the 0.11.0 publication |

## 5. Traceability

Test-plan topics are plan § 7's names, expanded in `test-plan.md` (W3).
T-PROCESS (new) covers the record and guard checks of AC31-11 and
AC31-12: `status.md` and `review.md` entries, the `gh` listings, the
push-URL and ancestry checks, and the host guard logs.

| Requirement | Criterion | Fork criteria | Other sources | Test-plan topic | Review |
| --- | --- | --- | --- | --- | --- |
| REQ31-01 | AC31-01 | CF-AC-01 | D28-13 | T-FORK-EVIDENCE | R-VERIFY |
| REQ31-02 | AC31-02 | CF-AC-02, CF-AC-03, CK-AC-02 | R30-01 | T-FORK-EVIDENCE, T-POOL-PRODUCT, T-CUDA | R-VERIFY |
| REQ31-03 | AC31-03a, AC31-03b | CF-AC-04a, CF-AC-04b, CF-AC-04c, CK-AC-05 | I-5 | T-FORK-EVIDENCE, T-WS-BUILD, T-CUDA | R-VERIFY |
| REQ31-04 | AC31-04 | CF-AC-05b, CK-AC-05 (K-G3), CK-AC-07 | R30-12 | T-ORIN, T-POOL-PRODUCT, T-TEARDOWN-PRODUCT, T-CUDA | R-VERIFY |
| REQ31-05 | AC31-05 | CF-AC-05 | — | T-FORK-EVIDENCE | R-VERIFY |
| REQ31-06 | AC31-06 | CF-AC-06, CK-AC-03, CK-AC-06 | R30-09 | T-FORK-EVIDENCE, T-PRODUCT, T-ORIN (D-14) | R-DES, R-VERIFY |
| REQ31-07 | AC31-07 | CK-AC-01, CK-AC-07 | — | T-PRODUCT | R-PIN, R-VERIFY |
| REQ31-08 | AC31-08 | CF-AC-07b, CK-AC-07 | D28-13, R30-07 | T-REGISTRY | R-FC-PUB, R-FK-PUB, R-VERIFY, R-CODE-4 |
| REQ31-09 | AC31-09 | CF-AC-08, CF-AC-08b | — | (fork tests) | R-PROP, R-PROP2 |
| REQ31-10 | AC31-10 | — | R30-02…07, R30-11 | T-PRODUCT | R-CODE-3, R-VERIFY |
| REQ31-11 | AC31-11 | CF-AC-09, CK-AC-08 | — | T-PROCESS | R-VERIFY |
| REQ31-12 | AC31-12 | CF-AC-10, CK-AC-08 | — | T-PROCESS | R-VERIFY, R-CODE-4 |
| REQ31-13 | AC31-13 | — | Slice 30 G9 | T-PERF | R-VERIFY |
| REQ31-14 | AC31-14 | CF-AC-04a, CF-AC-07a, CF-AC-07b, CF-AC-09, CK-AC-07, CK-AC-08 | I-17, Q8 (D-16) | T-FORK-EVIDENCE, T-PRODUCT | R-PIN, R-VERIFY |
| REQ31-15 | AC31-15 | CF-AC-11 | Q10, Q12, Q13 | T-CUDA-COMPAT | R-DES, R-VERIFY |
| REQ31-16 | AC31-16 | CK-AC-04, CK-AC-07 | REQ-068, Q2 | T-PRODUCT | R-PIN, R-VERIFY |

## 6. Changes from plan § 5

- AC31-02, 04, 07 and 14 name the registry pins (`cudarc-fathomdb`
  `=0.19.10`, Candle `=0.11.0`, Q21) where § 5 names `C`/`K` git pins,
  and add `K8` and `K9` through CK-AC-07.
- AC31-02 names the four Slice 30 Orin GPU tests (two files; § 5 and
  plan § 7 revision 16 put all four in `tests/tegra_fragmented_va_cuda.rs`,
  which has one).
- AC31-04 adds T-TEARDOWN-PRODUCT (D-15); AC31-06 adds the D-14
  measurements and the marker-feature comment rule.
- AC31-07's second-source test is a git `cudarc-fathomdb` next to the
  registry one, or a second version, instead of a different URL or `rev`;
  the guard's script, tier and failure conditions are named.
- AC31-08 is split into the cudarc half (evidenced) and the Candle half,
  and T-REGISTRY moves into W6 (it runs at R-VERIFY; R-CODE-4 re-checks).
- AC31-10 lists the Slice 30 test files and the named exceptions.
- AC31-11 and AC31-12 name their commands and records.
- AC31-13 states G9's arms, N and the RTX 3090 baseline.
- AC31-14 adds the R-PIN checklist (Q20), the no-rewrite checks and the
  D-16 lock-drift guard (Q8).
- AC31-15 names CF-AC-11, the criterion of CF-REQ-11, and allows
  UNMEASURED only for C3 and C5.
- REQ31-16 gives REQ-068 a criterion (R-REQ F1).
- REQ31-07, 11 and 12 had no statement in § 5; each now states its
  criterion's intent, adding nothing to it.
- REQ31-09 records the owner's hold (Q17).

## 7. R-REQ rulings on the open points

R-REQ (2026-10-13) ruled on the seven W1 open points; the criteria above
apply them.

1. **`candle-kernels` at the publication commit, now `K9`.** The
   `candle-kernels` diff `K..K9` is empty, so `K9` names one Candle commit
   for the whole cohort without changing the kernels (§ 1, AC31-16).
2. **K-G2 carries from `K` to `K8` and `K9`,** because `K..K9` changes
   only manifest version, dependency-source and description lines; the
   diff evidence is in AC31-03b. If the owner approves the 13.0 VM
   session, T-WS-BUILD 13000 exercises the registry form and replaces the
   carry.
3. **The D-16 lock-drift guard** is a criterion of AC31-14.
4. **T-TEARDOWN-PRODUCT** is a criterion of AC31-04 (Orin), with named
   tests; R-DES fixes the fork-after-init cases in `design.md`.
5. **The registry-build check runs in W6** (AC31-08), with AC30-07's
   inversion as AC31-10's named exception.
6. **The marker feature's future** is left to D-11; AC31-06 adds that no
   comment may claim the marker guards against registry `cudarc`.
7. **REQ-068** has its own requirement, REQ31-16, with AC31-16.
