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
