# Slice 120 test map and acceptance

Proposed human-intent tests; no tests executed by this design. Each changed
behavior gets its own committed/staged real RED before implementation. Keep tests
unchanged through GREEN; genuine failures, not missing tools/imports or hand-built
oracle snapshots. Existing regressions remain. Unit seams may inject loader or
probe results; lifecycle tests use a real database and real providers.

Owner seq-378 moves the new delivery/compiler tests below to
[.31](../../../0.8.31/cuda-delivery-plan.md). They are future proposals, not .29
mandatory gates or granted execution. R1's bounded quantitative disposition is
accepted by seq-379; its quantitative tests/evidence are also .31. Current package,
CPU/driverless, installed-consumer and independent130 obligations remain under
their actual selected inputs. Historical coordinates need future reselection.

| Requirement | RED assertion and implementation seam | Qualification / acceptance |
| --- | --- | --- |
| R1 | Pure query-result aggregation fixture with complete/missing windows, alias mapping and0.0.1 exclusion, only if collection code is added. Do not write a test of prose. | Saved existing review plus actual comparable response/share calculation or explicit owner bounded-review disposition. No invented share. |
| R2 | Extend `src/python/tests/test_coinstallation_guard.py`: two distinct same-name metadata dirs reject; repeated same canonical path does not; both12/cu130 marker mismatches metadata/version fail before native import; generic andcu130 remnants conflict. Sentinel native import hook proves no early dlopen. After mapping, both variants reject absent native identity, 12↔13 displacement and stale same-major native RELEASE even when CUDA identity matches; provider-call sentinel stays zero. | Isolated wheel-only venvs for each variant; native release/CUDA identity, member digest and metadata agree. Swap12 native into13 and13 into12; also substitute an older same-major native release; each refuses before provider work. Missing identity on an installed wheel refuses. Explicit source/editable development exception retains tests and cannot qualify a wheel. |
| R3 | Extend `src/ts/tests/npm-platform-split.test.ts` and existing Tegra tests: unset remains12; explicit13 requires only13; both installed select exactly requested; selected version/name mismatch, missing/malformed13 manifest, invalid selector and non-x86 explicit13 fail with zero native require calls. Local-dev candidate cannot override explicit12/13; addon-load failure never requires other variant. Unset/default12, explicit12 and explicit13 all reject absent native identity, opposite-major members and stale same-major native release after mapping with zero provider calls and no second addon require. Only unset/local-dev may omit identity. | Installed tarballs outside checkout with actual main@version; default12 works with or without13 package; explicit13 absence gives loader error even if12 is usable. Displace native members in both12↔13 directions and substitute a stale same-major release; all installed routes refuse before provider work. Existing Tegra selector unchanged. |
| R4/R6 | `cuda_libraries.rs` tests:13 missing/missing-symbol remedy does not prescribe CUDA12; curand required-major10 does not choose CUDA12 remedy; compiled-major diagnostic is13 without loaded libs. Reuse device/reranker policy seams: missing/wrong/symbol errors under auto/forced, explicit CPU makes no probe call, pool failure remains error. | Existing public Python/TS/Rust CUDA details and CLI doctor-v1 fields/exit mapping unchanged; actual wrong-major/symbol cases use existing approved qualification fixtures. No physical K620 test. |
| R5/R7 | Extend existing `scripts/tests/test_cuda_release_contract.sh`, `test_cuda_artifact_linkage_inspector.py`, preflight/PTX fixtures and package rehearsal tests: selecting13 cannot use12 toolkit/features/image/native member; wrong PTX/target/ELF floor or dynamic CUDA member refuses;12 baseline constants unchanged. Semantic fixtures, not whole-file snapshots. | Actual compiler command/version, cudarc CUDA_VERSION, extracted PTX and static-runtime provenance agree. Actual emitted wheel ABI and every ELF/glibc dependency pass existing inspectors; Node version/N-API floor is supported. Licence gate inspects the exact cu130 ZIP METADATA/licence text and Node13 tarball package.json/licence text, not ordinary source/proxy checks or a separately rebuilt wheel. |
| R8 | A demonstrated compiler/source defect needs a discriminating fork-local test and independent review; do not generate a false compiler oracle. Existing12 pipeline does not get retested merely to create a receipt. | Exact reviewed compiler-safety disposition covering consumed/generated kernels; no13 build before it. Finite provider smoke is supplementary, not non-exposure proof. |
| R9/R10 | Extend existing installed package smoke/rehearsal harness, not a second framework. Candidate mismatch/missing-native-member input refuses; bind native file digest to candidate member. Frozen RED only for changed harness behavior. If retained-archive licence input support is added to the existing gate, extend `scripts/tests/test_check_license_consistency.sh`: exact local-version wheel/tarball accepted; missing/wrong LICENSE, wrong SPDX/name/version reject. If selected SBOM correlation behavior changes, use existing survey/rehearsal tests for a missing selected native/fork/static-runtime component or mismatched candidate identity; no new framework. | Real embedding+reranking, write/read, close, separate-process reopen/read/close/exit for wheel, Node and external Rust SDK; actual selected allocation/PID/UUID and loaded paths for claimed CUDA. Exact new wheel/tarball pass licence-shipping assertions and existing SBOM/schema checks, with selected dependency closure/static-runtime inventory correlated to actual archive members and candidate hashes. Repository inventory alone does not qualify artifact SBOM. No source imports or mocked DB; no repeat of unchanged passing checks. |
| R11 | Existing `scripts/check-single-cudarc.sh` tests plus a minimal external consumer manifest: consumer-root patch is required; conflicting concrete CUDA features/resolved extra cudarc rejected before supported build. | External Rust12/13 projects resolve exact qualified pins/kernels and run SDK lifecycle. Source-built matching CLI doctor cuda-libraries/gpu/reranker-gpu handles all outcomes with existing typed schemas/exits; public13 CLI archive remains explicitly deferred if ruled. |
| R12 | Existing selected-candidate input checks refuse substituting a different source/artifact before130. No synthetic test of release-state prose. |130 runs affected final-artifact cells after integration;160 receives exact unpublished candidate map and explicit unavailable cells. No assertion that120 alone qualified all CUDA platforms. |

## Minimum installed environments

For Python/Node and external Rust, use the same selected-major behavioral table in
design.md: compatible12-only,13-only,both,neither and driverless. In each authorized
cell run embedder and reranker `cpu`, `auto`, forced selected `cuda:N` separately;
verify their reports independently. Hermetic loader/policy tests cover the full
cross-product; actual environments require permitted venues and are coordinated
with130 to avoid duplicate campaigns. An unavailable real cell stays UNMEASURED,
not a hermetic pass. Both-library success must record actual cuBLAS/cuBLASLt and
cuRAND versions/paths; shared curand soname requires deliberate search ordering.

Driverless x86 containers receive no CUDA mounts/search paths or device nodes.
Explicit CPU on a GPU host must also show no CUDA driver/library access using the
existing CPU proof; passing driverless CPU alone does not prove it. Forced CUDA
must fail typed without running CPU work. Do not swap artifacts after failure.

For any numerical qualification use existing independent CPU/reference behavior
and established equivalence tolerances; compare padded/nonpadded batches,
sequence boundaries (1/31/32/33 and511/512 tokens), repeated token IDs, normal
BGE and CE cases. Kernel-level mitigation tests add only demonstrated branch
patterns and boundary shapes. No regenerated goldens or new byte-equality policy.
Persistent format37 lifecycle and sane finite outputs are necessary but cannot
alone clear silent compiler corruption.

## Future .31 checks and exit

While implementing, run affected loader, CUDA policy, artifact/preflight and
interface tests in latency order. Final executable/source change gets one
`./scripts/agent-verify.sh` from its owned checkout, plus applicable package,
Markdown/public docs validators; use actionlint if workflows change. A scoped
pass is not workspace green. Source-only contracts may become reviewed before
native windows open, but all build/runtime-dependent outcomes remain unqualified.

Exit requires owner contract ruling, independent design and implementation reviews,
true RED/GREEN for changed behavior, exact selected source/pins/compiler/ABI proof,
licence/SBOM checks on exact candidate archives, installed consumer results and130
handoff. Missing compiler safety, actual library
cells or artifact bytes are remaining work. Registry and Pages publication and
post-publication consumers remain separate160 work, never implied by this exit.
