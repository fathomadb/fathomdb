# Slice 31 reviews

FathomDB reviews record model, findings and dispositions here. Fork
reviews get one line each; their findings and dispositions are in the
fork's `fathomdb/review.md` (plan § 12).

## Fork reviews

| Review | Fork | At | Model | Verdict | Record |
| --- | --- | --- | --- | --- | --- |
| R0-rev8 (also R-FC-PLAN, R-FK-PLAN part 1) | both + FathomDB | plan revision 8 | opus | APPROVE-WITH-FIXES; fixed in revision 9 and fork plan revisions 2 | each fork's `fathomdb/review.md` |
| R-FC-REQ | cudarc | FC1 | opus | APPROVE-WITH-FIXES; all fixed | cudarc `fathomdb/review.md` |
| R-FC-DES | cudarc | FC2 | opus | APPROVE-WITH-FIXES; all fixed | cudarc `fathomdb/review.md` |
| R-FC-TEST | cudarc | FC3 | opus | APPROVE-WITH-FIXES; all fixed | cudarc `fathomdb/review.md` |
| R-FC-CODE | cudarc | FC7 | opus | APPROVE-WITH-FIXES; all fixed | cudarc `fathomdb/review.md` |
| R-FC-CODE-2 | cudarc | FC8 G6 fix round | opus | APPROVE-WITH-FIXES; all fixed; new `C` = `b777f8d` | cudarc `fathomdb/review.md` |
| R-FC-VERIFY | cudarc | `b777f8d` | opus | APPROVE-WITH-NOTES; fit to pin; all notes disposed | cudarc `fathomdb/review.md` |
| R-FC-PUB | cudarc | `b777f8d` (FC10, early) | opus | APPROVE-WITH-FIXES; findings 2–4 fixed in `d3a5a055`; finding 1 is an owner decision before R-PUB | cudarc `fathomdb/review.md` |
| R-FK-PLAN | Candle | FK1 `362fc4f2` | opus | APPROVE-WITH-FIXES (H1, M1–M6, L1–L10); fixed in `c49d168c`, M6 in FathomDB plan revision 14 | Candle `fathomdb/review.md` |
| R-FK-CODE | Candle | FK2 `0d6fa138` | opus | APPROVE-WITH-FIXES; M1 fixed test-first at `853324fc` | Candle `fathomdb/review.md` |
| R-FK-CODE re-check | Candle | `853324fc` | opus | APPROVE | Candle `fathomdb/review.md` |
| R-FK-VERIFY | Candle | `853324fc` | opus | APPROVE-WITH-FIXES; fit to pin; F1, F3, F4, F6 fixed at `887e5781`; F2 and F5 carried to FathomDB (plan D-11, T-CUDA) | Candle `fathomdb/review.md` |
| R-FK-PUB | Candle | `cd64715e` (K8) | opus | APPROVE; finding 1 (re-exported `cudarc` swapped for `cudarc-fathomdb`: semver-breaking) led the owner to renumber to 0.11.0 (Q21), so K9 `182ceab0` is the publication commit, re-qualified (CK-AC-07 at K9); finding 2 carried to AC31-08 (T-REGISTRY); 3–5 fixed at `182ceab0`/`4abb8b38`; 6 exempt | Candle `fathomdb/review.md` at `e302d54c` |

## FathomDB reviews

| Review | At | Model | Verdict | Record |
| --- | --- | --- | --- | --- |
| R-REQ | `requirements.md` at `f41563903` (W1) | opus | APPROVE-WITH-FIXES; all findings fixed in the revision after plan revision 17 | below |
| R-DES | `design.md` revision 1 at `1e0537856` (W2) | opus | APPROVE-WITH-FIXES; all ten findings fixed in `design.md` revision 2 (plan revision 18) | below |
| R-TEST | `test-plan.md` revision 1 at `72a471508` (W3) | opus | APPROVE-WITH-FIXES; all seventeen findings fixed in `test-plan.md` revision 2, `design.md` revision 3 and plan revision 19 | below |
| R-DES-4 | `design.md` revision 4 § 14 at `e8cf01446` | opus | APPROVE-WITH-FIXES; findings and owner rulings Q29-Q32 applied in revision 5 (`3f0c5a468`, `94edbbf1b`) | session record; steward ledger Q29-Q32 |
| R-DES-15 | `design.md` § 15 at `3f0c5a468` | fable (owner-directed, 2026-10-14: "an *informed* adversarial review by fable-medium"; ledger Q40) | APPROVE-WITH-FIXES, 11 findings (2 HIGH); applied in `94edbbf1b` and `6ce7d7023` | session record |
| R-DES-15 re-review | `design.md` §§ 14.1, 15 at `6ce7d7023` | fable (Q40) | APPROVE-WITH-FIXES, 8 findings; applied in `0cac82b14` | session record |
| R-DES-14 re-review | `design.md` revision 6 §§ 14.2, 14.3 at `a1f9bf589` | fable (Q40) | APPROVE-WITH-FIXES; applied in `f144c8210` | session record |
| R-CODE-3 | implementation of design revision 6, `831255b9e..392832c72` | opus | APPROVE-WITH-FIXES, no HIGH; MEDIUM-1, MEDIUM-2, LOW-3 to LOW-7 all fixed: LOW-4 RED `9e835cf09` → GREEN `51d5df79e` (with LOW-6), LOW-5 `836af60a1`, MEDIUM-1, MEDIUM-2, LOW-3, LOW-7 `fc60245a4`; MEDIUM-1's strengthened Orin test re-run GREEN at `fc60245a4` | `status.md`; `qualification/2026-10-15-ORIN-M1-fc60245a4.log` |
| R-VERIFY | `1f1646f74` | opus | B1-B7 record gaps, all fixed in the following commit; ACs met except those pending PERF-03, S-C3 and S-13 | `qualification.md` |
| R-CODE-4 | `574b09e78` (release/0.8.28 after the merge) | opus | APPROVE: REG-a, REG-e (three Candle crates), REG-f, the pins and PROC-05 to PROC-08 all pass from the registry; two notes fixed on owner direction in `ef9a440dd` (RED `98a661c8a`): the exact `cudarc-fathomdb =0.19.10` pin under `embed-cuda`/`rerank-cuda`, and the registry check exits 2 in full mode on a non-Tegra host; feature-complete gate re-run at `ef9a440dd` | `qualification/2026-10-15-R-CODE-4-574b09e78.log`, `qualification/2026-10-15-FC-ef9a440dd5fc.md` |

### R-REQ (2026-10-13)

Opus subagent, read-only, on `requirements.md` at `f41563903`
(DRAFT FOR R-REQ) against plan revision 16 § 5, D28-13, Slice 30's
R30/AC30, REQ-068 and the forks' CF/CK criteria. Verdict:
APPROVE-WITH-FIXES. The fixes were applied together with plan revision 17
(the Candle crates 0.11.0 from `K9`, Q21), so the criteria name `K9`
where the finding named `K8`.

| # | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| F1 | High | REQ-068 (CPU-loadable CUDA artifacts) rests on the `candle-kernels` git patch but has no criterion | Fixed: new REQ31-16 / AC31-16: one `candle-kernels` package in `Cargo.lock`, git source at `K9`, no registry copy; Candle CK-AC-04 at `K`, `K8`, `K9`; `scripts/release/inspect-cuda-artifacts.py` on the x86 wheel and NAPI tarball and `scripts/release/check-tegra-wheel-linkage.sh` on the Tegra wheel and NAPI extensions |
| F2 | High | AC31-08/T-REGISTRY vague: no stated resolution, link, smoke or consumer CUDART statement; runs too late | Fixed: AC31-08 (a)–(d): no-`[patch]` consumer, registry resolution of `cudarc-fathomdb` 0.19.10, Candle 0.11.0 and upstream `candle-kernels` 0.10.2, no `cudarc` 0.19.*; Orin build and link with `LIBRARY_PATH` stated (R-FK-PUB finding 2); smoke `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable` with `FATHOMDB_REQUIRE_LIVE=1`; dynamic `libcudart` for crates.io consumers, REQ-068 only for FathomDB-built artifacts. Moved into W6 (runs at R-VERIFY; R-CODE-4 re-checks); `check-tegra-pool-registry-build.sh:102` breaks when W6 drops the cudarc patch, stated; AC30-07 exception named in AC31-10 |
| F3 | Medium | AC31-02 puts "four Slice 30 tests" in `tests/tegra_fragmented_va_cuda.rs`, which has one `#[test]` | Fixed: the four Orin GPU tests named (one in `tests/tegra_fragmented_va_cuda.rs`, three `#[ignore]` tests in `src/cuda_pool_policy_tests.rs`); AC31-04 names gate rows G1–G10; plan § 7 T-POOL-PRODUCT fixed |
| F4 | Medium | AC31-15 lets any T-CUDA-COMPAT cell be UNMEASURED; § 2 omits Q13 and GOAL.md's declined-action rule | Fixed: C1, C2, C4 must pass; only C3 (Q13) and C5 (Q15) may be UNMEASURED by owner decision; § 2 adds Q13 and the `status.md` + ledger record of a declined owner action |
| F5 | Medium | AC31-07's guard has no script, tier or failure conditions | Fixed: `scripts/check-single-cudarc.sh` (none exists today), fast tier, three failure conditions (`cudarc` 0.19.* in the lock; more than one or a non-registry `cudarc-fathomdb`; a `cudarc v0.19` node on `-e normal` for `embed-cuda`, `rerank-cuda`, `tegra-pool` on x86_64 and aarch64 Linux), `ug` and `flash-attn` excluded, committed fixture locks |
| F6 | Medium | The D-16 lock-drift guard (Q8) has no criterion | Fixed: AC31-14 D-16 guard: compares the `cudarc-fathomdb` dependency set with the fork's latest CI receipt run with FathomDB's lock; RED fixture (`libloading` changed), GREEN W6 lock; fork re-runs first on a difference. The current CI receipt does not record the set, so the fork adds it before GREEN; R-DES fixes path and format |
| F7 | Medium | AC31-08's Candle half checks only the `.cargo_vcs_info.json` sha | Fixed: packaged files equal `git show K9:<path>`, sha1 = `K9` with no `dirty`, `.crate` SHA-256 equal to the final dry run (`K9` values recorded; index-dependent, the file check is authoritative) |
| F8 | Medium | AC31-13 leaves G9's arms, N, Python import and the RTX 3090 baseline undefined | Fixed: ratio new / pre-slice Slice 30 build, N = 20 per arm in interleaved randomised blocks, Python import per SD-4, rerank speed-up non-gating (Slice 30 revision 3.3), RTX 3090 baseline the pre-slice commit with N and CI |
| F9 | Medium | AC31-14's Candle pin-chain diff is against `K`; no no-rewrite check | Fixed: Candle diff against `K9` (publication commit and `candle-kernels` `rev`); the cudarc diff bears only on re-qualification; `git merge-base --is-ancestor` for `C`, `K`, `K8`, `K9` |
| F10 | Low | AC31-10's "Slice 30 product tests unchanged" is not checkable | Fixed: nine files listed, `git diff <slice base>..<tip>` empty, named exceptions (registry-build script, pinned-override test, D-17 renames without assertion changes, the D-15 teardown assertion if fixed) |
| F11 | Low | AC31-11 and AC31-12 lack commands; "(records)" and "(guard logs)" are not test-plan topics | Fixed: `gh pr/issue list -R … --state all` empty on both forks, ancestry against `origin/release/0.8.27` false, upstream push URLs disabled; R-REQ, R-DES, R-TEST recorded before the first W6 commit (Q19); I-10 no steward or orchestrator; topic T-PROCESS (plan § 7) |
| F12 | Low | Stale state: § 1 `K8` "running"; AC31-03a omits G5a; AC31-06 omits D-14 measurements; `status.md` publication text; D28-13 says "git dependency" | Fixed: `K8` passed (receipt at `ddff4cbd`) and superseded by `K9`; G5a receipts at `b29c41c` carried by R-FC-VERIFY; D-14 VmRSS measurements in `qualification.md`; `status.md` "Publication preparation" rewritten (cudarc published by owner direction, Q17); D28-13 in `dev/plans/0.8.28-draft-scope.md` names the registry crates |

**Open-point rulings adopted** (requirements § 7): (1) `candle-kernels`
at the publication commit, now `K9`, because the diff `K..K9` of
`candle-kernels` is empty; (2) K-G2 (13.0, at `K`) carries to `K8` and
`K9`, the manifest-only diff written into AC31-03b, replaced by T-WS-BUILD
13000 if the owner approves the VM session; (3) D-16 guard in AC31-14;
(4) T-TEARDOWN-PRODUCT in AC31-04 with named tests, fork-after-init cases
fixed by R-DES; (5) the registry-build check in W6; (6) the marker
feature left to D-11, with no comment claiming it guards against registry
`cudarc`; (7) REQ-068 as REQ31-16.

Next: W2 (`design.md`) → R-DES.

### R-DES (2026-10-13)

Opus subagent, read-only, on `design.md` revision 1 (DRAFT FOR R-DES) at
`1e0537856`, against `requirements.md`, plan revision 17, the code it
cites and both forks' records. Verdict: APPROVE-WITH-FIXES. The fixes are
in `design.md` revision 2 (REVISED AFTER R-DES), with matching changes to
`requirements.md` and plan revision 18.

| # | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| 1 | High | C4 needed the Quadro K620, which must never run; the floor check was conditional on C4 and the probe builds the device before it enumerates | Fixed (Q22): design § 9.2. The floor check is unconditional, at the three generic `probe_cuda` call sites (resolver `probe`, `diagnose_gpu`, reranker `probe`), on the enumerated device, before `probe_cuda` and so before `new_cuda_device`; the floor comes from the build's `CUDA_COMPUTE_CAP` contract value through `option_env!` and a provided `CudaProvider::kernel_compute_floor` method, no per-arch literal; RED with a fake provider reporting a 5.0 device; `auto` cites the lowest-ordinal rule (`device_policy.rs:464-466`) with fake-provider tests. § 0.1: every new `windchill3` process names an RTX 3090 by UUID (bus `41:00.0`), so the decoy test's RED `new_cuda_device(0)` is on the 3090 |
| 2 | Medium | Single-cudarc condition C: `cargo tree --prefix none` prints `cudarc-fathomdb v0.19.10 (*)` for repeated nodes | Fixed: § 3.5 strips a trailing `(*)` mark before comparing |
| 3 | Medium | Fixtures with lock-only entries are inconsistent; `cargo tree --offline --locked` fails on them | Fixed: § 3.5 splits lock fixtures (parsed with `tomllib`, no `cargo`) from tree fixtures (consistent workspaces with a directory-source stand-in for the registry crate, locks generated by `cargo generate-lockfile --offline`; measured in a scratch fixture); the lock-set guard's tests (§ 7.2) use hand-minimal `cargo metadata` JSON, so they need no lock |
| 4 | Medium | The marker-free `git grep` would match its own test file and the todos ledger (as AC30-10 did) | Fixed: § 3.3 excludes both ledgers and builds the pattern from pieces; the same rule is written into AC31-10 |
| 5 | Medium | D-18 named `cudarc::cublas::…`, but the direct dependency is aarch64-only with no `cublas` feature; the remedy omitted `LD_LIBRARY_PATH`; the variant name fit only one case | Fixed: § 9.1 uses `candle_core::cuda::cudarc::{cublas,curand}::sys::try_culib()` (unsafe) and `loaded_library()`; remedy `pip install nvidia-cublas-cu12` plus its `lib` directory on `LD_LIBRARY_PATH` (no site-packages search on Linux); `LibraryUnavailable` with reason `Missing` or `WrongMajor`, kind `cuda_library_unavailable`; hermetic RED test mapping an injected `LibLoadError` to the refusal and remedy |
| 6 | Medium | The embedder's direct `cudarc` dependency is unused | Fixed by removal (§ 3.2): no source imports it, its features are a subset of Candle's, and FathomDB's lock is checked exactly by the pinned-override checker; `tegra-pool = []`; the pin record lists the Candle crates by manifest and `cudarc-fathomdb` as a lock-only pin (`"dependency": null`). The downstream consequence (Candle's caret governs a consumer's `cudarc-fathomdb`) is stated |
| 7 | Medium | Lock-set closure over the whole lock is broader than what the fork compiles | Fixed (Q23): § 7.2 takes the Linux-target normal closure from `cargo metadata --filter-platform` for both Linux targets, records root features, schema `v2`; measured 20 packages including the root against 26 for the whole-lock closure; remaining cost and the push-before-fork-run sequencing (raw GitHub URL, fork workflow line 234 at `C`) stated; no cycle |
| 8 | Medium | REQ31-10 said product behaviour must not change, contradicting N1, D-18, the floor and the doctor verb; AC31-10 named a nonexistent `rerank_tests.rs` | Fixed: REQ31-10 lists the five deliberate, bounded product changes; AC31-10 names `src/rerank/tests.rs`; all nine paths checked to exist |
| 9 | Low | D-18 and the floor are CUDA-version-policy decisions; appendix A decision 2 implied published FathomDB crates | Fixed: appendix B drafts the amendment to `ADR-0.8.28-cuda-version-support-policy.md`; appendix A decision 5 points to it; decision 2 says T-REGISTRY builds a path copy of the embedder against the published fork crates |
| 10 | Low | § 0 row listed `fathomdb-private-pool` as resolved; item 6 removal needs a fork table change; the BF16/F32 guard is GREEN at once; the checker must read target tables; guard B duplicated other checks; the `CUDARC_CUDA_VERSION` citation was wrong; "27" did not say whether it counted the root | Fixed: § 0 restated (measured with the marker on; Candle's set after W6); fork todo FT-2; the dtype guard dropped (the RTX 3090 and T4 runs catch a BF16 change); the checker reads `[dependencies]` and every `[target.'cfg(…)'.dependencies]` table; the `cudarc-fathomdb` lock count, version and source are checked only by the pinned-override checker; citation now `scripts/gcp-lab/remote-fork-cell.sh:86` only; counts now state "including the root" |

**Open-point rulings adopted** (design § 13): (1) fork version numbering
after 0.19.10 is the fork's decision, fork todo FT-1, non-blocking; (2) the
3 GiB default stays, with the usable-capacity measurement and the
576 MiB return-to-owner trigger; (3) the fork's Run 2 should compile
FathomDB's resolved `cudarc-fathomdb` feature set (recommended; fork todo
FT-3, the fork decides); (4) N1 stays conditional on its RED; (5) D-18
keeps the typed error, the report field and the doctor verb, renamed per
finding 5, stderr only on refusal (Q24), and the interface documents W6
updates are listed (design § 9.1 item 7); (6) C4 is hermetic (Q22); (7) a
Rust engine-level Orin close test is recommended to W3.

**Owner rulings** (steward ledger seq 325–328; plan § 15):

- **Q22:** the Quadro K620 is never run, including C4; C4 is the hermetic
  floor test; `windchill3` processes name an RTX 3090 by UUID.
- **Q23:** the lock-set guard covers the Linux-target normal closure.
- **Q24:** the D-18 stderr line only on refusal; `doctor cuda-libraries`
  always reports.
- **Q25:** C3 runs as a GCP cell: Spot T4, driver 580 only, artifacts
  built on `windchill3`, about 30-45 minutes and $0.15-0.20; owner approval
  per session; UNMEASURED if declined; the hermetic test stays (design
  § 9.3).

Next: W3 (`test-plan.md`) → R-TEST.

### R-TEST (2026-10-13)

Opus subagent, read-only, on `test-plan.md` revision 1 (DRAFT FOR
R-TEST) at `72a471508`, against `requirements.md`, `design.md` revision 2,
plan revision 18 and the code and scripts it cites. Verdict:
APPROVE-WITH-FIXES. The fixes are in `test-plan.md` revision 2 (REVISED
AFTER R-TEST, ready for W6), with matching changes to `design.md`
(revision 3), `requirements.md` and plan revision 19.

| # | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| 1 | High | No embed or rerank CLI verb: `fathomdb-cli` has only `recover` and `doctor` (`lib.rs:100-103`), and the CLI-scope ADR rejects a query surface; `c-s13-run`, `c-s13-libcuda1`, `c-c2`, § 4.6, CUDA-08, CUDA-09, CMP-C5 and design § 9.3 steps 3 and 5 embedded through the CLI | Fixed: on x86 12.6 the CUDA wheel from CUDA-02 in a scratch venv plus `dev/plans/runs/0.8.28-slice-31/harness/c2-embed-rerank.py` (new (W6)); on S-13 and S-C3 the embedder's live test binaries built `--no-run` (`tests/candle_bge.rs`, `candle_reranker::gpu_tests::gpu_loads_and_scores_finite`) with `FATHOMDB_EMBED_DEVICE=cuda:0` and `FATHOMDB_REQUIRE_LIVE=1`; `doctor gpu`, `doctor reranker-gpu` and `doctor cuda-libraries` kept for refusal and floor checks |
| 2 | High | CUDA-09 on S-13 would refuse: the VM has only the 13.0 cuBLAS | Fixed: CUDA-09 moved to S-C3, after the remedy step: divert `libcuda.so`, repeat the forced embed (cmd `c-c3-libcuda1`; design § 9.3 step 5) |
| 3 | Medium-High | cuRAND: the remedy named only cuBLAS; cuRAND's major is 10; C2's cuRAND source unrecorded | Fixed: the remedy names `nvidia-cublas-cu12` and `nvidia-curand-cu12` and puts `nvidia/cublas/lib` and `nvidia/curand/lib` on `LD_LIBRARY_PATH` (design § 9.1 item 2); no expectation asserts major 12 for cuRAND; C2 records cuRAND from `windchill3`'s loader cache |
| 4 | Medium | PRD-24 and CMP-D18 labelled "full", but the full gate builds no `embed-cuda` | Fixed: PRD-24 is build-only on `windchill3` with the exact command `c-prd24`; CMP-D18 is a cross-reference; one `scripts/test-feature-complete.sh` run on `windchill3` added (PRD-33, cmd `c-fc`) |
| 5 | Medium | Test counts: the C3-only test needs `#[ignore]`; PRD-26 has 7 `doctor_cuda_allocator_*` tests, not 5 | Fixed: `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` is `#[ignore = "C3 GCP cell"]`, allowlisted `ignored-by-design`, run on S-C3 with `--ignored --exact`; `windchill3` expects R-TEST's "4 passed; 1 ignored" plus the Q26 test, so "5 passed; 1 ignored"; PRD-26 lists the seven tests (`lib.rs:3170-3301`) |
| 6 | Medium | R12's RED reason was wrong | Fixed: RED stated as "the child panics in device construction (`cublasCreate_v2` 'Expected function…' via dynamic loading); no `LibraryUnavailable`"; GREEN requires the typed refusal in the child's output. The decoy is also installed as `libcublas.so` so the vendored 0.19.7, which tries the unversioned name first, opens it. Note: the cudarc sources (0.19.7 and `C`) word this panic `Missing symbol cublasCreate_v2: …` |
| 7 | Medium | `a_successful_load_writes_no_stderr_line` cannot fail on the base | Fixed: called a regression guard; R13's REDs are the report test and the refusal-line test |
| 8 | Medium | Release-contract arms check only a nonzero exit (`expect_fail`, lines 73-80) | Fixed: new helper `expect_fail_with <message>` used by R4 and R5; R5's RED is "`baseline CUDA contract agrees` fails on the Phase B sha" |
| 9 | Medium | PRD-21 did not reject every form of a direct dependency or check `tegra-pool = []`; design § 3.4 contradicted itself | Fixed: arm `a_direct_cudarc_fathomdb_dependency_is_rejected` (three fixtures) replaces the caret-only arm; `tomllib` check `features["tegra-pool"] == []` (R6 case, cmd `c-tegra-pool-feature`); design § 3.4: must not be declared, in any form |
| 10 | Medium | G9 used `G9_STRICT`, which does not exist; the S-sync rule was unstated | Fixed: `QUAL_QUIET_STRICT=1` (`run-gates.sh:21` defaults it to 0); speed-up over S uses the S-sync subset where present (`summaries/g9.txt` line 17) |
| 11 | Medium | Relative fork paths | Fixed: absolute `/home/coreyt/projects/fathomdb-forks/{cudarc,candle-fathomdb}` in PROC-06, `c-reg-crates`, `c-pin-chain` |
| 12 | Medium | S-13 sizing (owner Q27) | Fixed: § 1.3 and § 4.7: `--machine-type n1-standard-8` or larger for one build attempt (`gcp-cuda-lab.sh` lines 16-17, 71, 114-116, read only), many VM attempts, 2 h cap, receipts after every cell, owner go-ahead per session |
| 13 | Low-Medium | `c-pool-lib` and `c-pool-s31` lacked the device policy | Fixed: `FATHOMDB_EMBED_DEVICE=cuda:0` in both |
| 14 | Low | No Orin window at the pin commit for R14 and the R15/R16 characterization | Fixed: window P, about 20 min, before N1 is written (§ 2, § 4.2) |
| 15 | Low | PROC-02 and `c-ancestry` were vacuous | Fixed: `c-review-order` finds the commit that introduced the `\| R-TEST \|` row (the bare text `R-TEST` appears earlier) and checks it is an ancestor of the first W6 commit; `c-ancestry` iterates `<slice base>..<tip>` |
| 16 | Low | Inexact commands | Fixed: `c-c1` takes `OUT_DIR` from `--message-format=json`; `c-c2` pins `nvidia-cublas-cu12==12.1.3.1` and `==12.9.2.10` (PyPI, 2026-10-13) and records `pip freeze`; the 12.4 cell is dropped |
| 17 | Low | AC31-16's Tegra linkage had no row; CUDA-07 compared a symlinked path | Fixed: ORIN-15 (`scripts/release/check-tegra-wheel-linkage.sh`, cmd `c-tegra-linkage`); CUDA-07 compares real paths |

**Open-point rulings adopted** (test plan § 6): OP-1 the release contract
keeps only the `candle-kernels` git patch at `K9` and its single git lock
entry, reads the rev from `scripts/pinned-override-rot.json`, and its arms
assert messages (design § 1 and § 11 list the checker and its tests);
OP-2 AC31-10 excludes `dev/steward/steward-ledger.jsonl`; OP-3 the
feature-complete gate keeps indices 0 and 1 under
`CUDA_DEVICE_ORDER=PCI_BUS_ID` (the K620 at `61:00.0` is index 2), and
moving it to UUIDs is a main-thread todo; OP-4 TDN-03 has no counter
clause; OP-5 REG-c with `FATHOMDB_POOL_MODE=off`, `auto` recorded as
information; OP-6 confirmed; OP-7 no F4 mutation; OP-8 as written plus
window P; OP-9 the x86 perf harness in scope with the Node addon only,
built by `npm run build:native:cuda`, idle check `nvidia-smi --id=<UUID>`,
no nvgpu fd check; OP-10 `--lock`; OP-11 three Orin windows A, B and C
(G9 last, `QUAL_QUIET_STRICT=1`, `NO_SWAP=1`, no `cargo` or `rustc`), about
1 h extra for doubling N.

**Proportionality cuts adopted:** ORIN-11 merged into G1; the CMP-C2 12.4
cell dropped; REG-e at R-VERIFY cites the Candle FK5 receipt (104, 42 and
206 files) and runs once, at R-CODE-4; TDN-04, TDN-05, CUDA-10, ORIN-10,
CMP-C1a, CMP-C4, CMP-D18 and PRD-29 are cross-references only.

**Owner rulings** (steward ledger seq 329–331; plan § 15):

- **Q26:** a CUDA 12 cuBLAS or cuRAND that lacks a symbol FathomDB or
  Candle needs gives a typed refusal (`LibraryUnavailable`, reason
  `MissingSymbol`), never a panic. Designed in design § 9.1: FathomDB
  checks, through the fork's existing `is_symbol_present`, a checked-in
  set taken from cudarc's `cublas` and `curand` `result` layers (13 and
  14 names), guarded by `scripts/check-cuda-required-symbols.py`; RED
  tests R12b (a decoy missing `cublasSgemmStridedBatched`) and R18; no
  fork change.
- **Q27:** S-13 may use `n1-standard-8` or larger for one build attempt;
  many VM attempts; 2 h per session; receipts after every cell; owner
  go-ahead per session.
- **Q28:** G5's equality to the Slice 30 hash is report-only, evaluated
  after the work.

Next: W6.
