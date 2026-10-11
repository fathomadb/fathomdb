# Slice 103 Track T — Tegra continuity receipt

- Source baseline: `8fb6fdac9d2f1655a62355e0ca8af51d42dd1fb7` on `release/0.8.27`.
- Track branch: `llm/slice-103-tegra`.
- Reviewed branch code and gate commit: `86ab7d61992b122e0820fbe062d92321e051c3b7`. The host-native wheel was built from the earlier source commit `a8c7e2f12ab049073dee00887414c6400f164194`.
- Historical comparison: `b35212f678626e09ea7293e00ee337288ce74ff1..8c4fdfa9bb263552d7ffc1c019bc553169e2d266` on `origin/release/0.8.26`.
- This receipt is branch-local evidence. Integrated-candidate qualification remains for the coordinator.

## Five-commit changed-file inventory

`1a131e78` supplied the build/dependency/artifact checks; `600c598c` retained a historical review transcript; `9c7c3da7` corrected installed-wheel import provenance; `91b28587` supplied the Pages route and documentation; `8c4fdfa9` corrected the supported `gh run list` interface.

| Historical file | 0.8.27 disposition |
| --- | --- |
| `Cargo.lock` | Carried four Candle Git sources at `1aefdd008ad1c994635b688b8e6f2ae5a5a920ae`. |
| `Cargo.toml` | Carried all four Candle patch pins at that revision. |
| `README.md` | Excluded 0.8.26 general publication prose; baseline already states published 0.8.26 accurately. |
| `dev/plans/prompts/0.8.26-TEGRA-PAGES-PUBLICATION.md` | Adapted into `0.8.27-TEGRA-PAGES-PUBLICATION.md` as a future, separately authorized route. |
| `dev/plans/runs/codex/0.8.26/tegra-artifact-20261003T165309Z.log` | Historical review evidence only; cannot qualify a 0.8.27 artifact. |
| `docs/compatibility/index.md` | Excluded 0.8.26 publication rewrite; current published 0.8.26 facts remain and 0.8.27 Tegra is unpublished. |
| `docs/getting-started/index.md` | Excluded 0.8.26 publication rewrite; baseline already documents published 0.8.26. |
| `docs/index.md` | Excluded 0.8.26 publication rewrite; baseline already documents published 0.8.26. |
| `docs/install/python.md` | Excluded historical 0.8.26 Pages install claim; 0.8.27 candidate is unpublished and the baseline published-version guidance remains. |
| `docs/install/rust.md` | Excluded 0.8.26 general publication rewrite; baseline is current. |
| `docs/install/typescript.md` | Excluded 0.8.26 general publication rewrite; baseline is current. |
| `docs/operations/index.md` | Adapted with a link to the 0.8.27 candidate Pages runbook. |
| `docs/operations/tegra-pages-publication.md` | Adapted to distinguish the unpublished 0.8.27 candidate and require separate authorization. |
| `mkdocs.yml` | Carried the Pages runbook navigation entry. |
| `scripts/agent-test.sh` | Carried the operator test registration while preserving all 0.8.27-only registrations. |
| `scripts/check-cuda-release-contract.py` | Carried the shared Candle revision contract. |
| `scripts/release/build-python-cuda-tegra.sh` | Carried static CUDA symbol/dependency checks and clean-venv installed import outside staged source. |
| `scripts/release/publish-tegra-pages.sh` | Carried guarded route and supported `gh run list`; adapted explicit smoke skip to return nonzero `INCOMPLETE`. |
| `scripts/tests/test_cuda_release_contract.sh` | Carried static-CUDA and installed-import guard, adjusted mutation fixtures to the new pin. |
| `scripts/tests/test_public_doc_truth.sh` | Excluded 0.8.26 general publication fixture changes; baseline already tests the published-version truth. |
| `scripts/tests/test_tegra_publication_operator_path.sh` | Adapted for 0.8.27 and added functional remote-SHA, workflow-result, and smoke-skip rejection cases. |

## 0.8.27-only governed pin gate

The current branch has an additional offline, independently pinned Candle override gate that the five 0.8.26 commits did not change. The first capable full verifier exposed its stale `cf02edbc` pin. Track T updated `scripts/check-pinned-override-rot.py`, `scripts/pinned-override-rot.json`, and `scripts/tests/test_check_pinned_override_rot.sh` to the new immutable revision. The fixture failed before implementation and passed afterward. The metadata retains an `external-source-unassessed` posture; a human advisory assessment is not claimed here.

## Local test sequence

- RED `test_tegra_publication_operator_path.sh`: exit 1; missing publisher, runbook, and handoff.
- RED `test_cuda_release_contract.sh`: exit 1; wrapper lacked static-CUDA and installed-wheel import proof.
- RED explicit Pages smoke-skip fixture: exit 1 because historical route returned success for an incomplete check.
- GREEN `test_cuda_release_contract.sh`: exit 0.
- GREEN `test_tegra_publication_operator_path.sh`: exit 0.
- GREEN `check-cuda-release-contract.py`: exit 0.
- GREEN `agent-lint-md.sh` and `agent-lint-docs.sh`: exit 0.
- Full `agent-verify.sh` first exited 1 at checkout-owned `.venv` preflight; isolated `.venv` was then prepared.
- Full `agent-verify.sh` second exited 1 at Cargo lint because the sandbox could not resolve GitHub for the new Candle pin.
- Full `agent-verify.sh` third, capable run exited 1 at the stale governed Candle override pin; this is the RED gate for the follow-up fix.
- GREEN `test_check_pinned_override_rot.sh`: exit 0.
- Full `agent-verify.sh` fourth, capable run exited 1 after 1,452 seconds: 179/181 registered test suites passed; Rust, TypeScript, lint, typecheck, and security passed. `test-python` exited 4 and `test-python-native-receipt` exited 1 because the test-hook candidate gate required a clean checkout while the governed-pin and receipt files were still uncommitted. Those files were committed as `fc44d9e5` before the clean-head rerun.
- Full `agent-verify.sh` fifth, clean-head run at `fc44d9e5d95885530b5d10b2a6ff891c7d3300b2` exited 1 after 1,041 seconds: 180/181 registered suites passed, including the native candidate receipt. Python had 9 subprocess import failures (`ModuleNotFoundError` for `fathomdb` or `eval`), 1,562 passes, and 30 skips because the isolated venv invocation lacked the checkout's `src/python` path. A process-local `PYTHONPATH="$PWD/src/python"` resolves both imports without an editable install.
- RED `test_tegra_wheel_linkage.sh`: exit 1 because the fail-closed checker did not exist. RED `test_cuda_release_contract.sh`: exit 1 because the wrapper did not invoke it.
- The two RED command outputs were observed in the implementer tool session but were not saved as raw log files before the checker fix; this receipt records their commands and exits without claiming a retained transcript.
- GREEN `test_tegra_wheel_linkage.sh`: exit 0, including CUDA registration, cuBLAS, `nm` failure, and `readelf` failure fixtures. GREEN `test_cuda_release_contract.sh`: exit 0. ShellCheck and Bash syntax checks: exit 0.
- Full `agent-verify.sh` sixth, clean-head run at `744656d68cb9f2c42633c8a0a028cdd99d21e3c0` exited 1 after 1,014 seconds: 181/182 suites passed. Python had only one remaining failure: the Slice 100 subprocess uses `python -I`, which ignores `PYTHONPATH`; 1,573 tests passed and 27 skipped. A non-editable CPU wheel built from this exact branch was installed into the checkout-owned `.venv`. Its SHA-256 is `be931480e0b1ecf90e96ec44e0a0ce270f64be3258ea9637ebc99fe7f8c805bd`; isolated `python -I` import and the exact failing test then passed.
- Full `agent-verify.sh` final clean-head rerun at `744656d68cb9f2c42633c8a0a028cdd99d21e3c0`: **exit 0**, all **182/182 suites passed** with zero skips/exclusions; security reported zero violations, blockers, and downgrades. Process-local `PYTHONPATH="$PWD/src/python"` supplied the eval harness, and the branch-built non-editable CPU wheel supplied isolated `python -I` imports. The exact 1,770-byte output is `evidence/verify-full.log`, SHA-256 `ded3eb2a69f53da2f46049cb32f5772cf19efaf6dfb196cd4d4169d429b32bb4`.

## Host-native build

The owner approved transferring the 69 MB complete-history bundle SHA-256 `7c15aa976362a9d961bac458fa23bbe250e25c881fa920a97adcb7f5453143e3` to established Jetson host `10.83.10.13`. The isolated Jetson checkout is at the code commit above. The host reports Linux AArch64 and `Orin (nvgpu)`. Its pinned-toolchain `--assert-only` check passed using checkout-local Python 3.12 and Maturin 1.14.1. The branch-local build **passed at `a8c7e2f12`**. Its wheel is `fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl` (9,061,384 bytes), SHA-256 `35ba47501ce20cbb7b05edf897c226d167aca0272c02bf48fe28c0270093873c`. The wrapper passed glibc floor 2.35, and extracted-extension `nm -D --undefined-only` and `readelf -d` checks both produced zero disallowed CUDA/NVIDIA findings. A fresh installed runtime venv loaded `/home/coreyt/projects/fathomdb-worktrees/slice-103-tegra-a8c7e2f12/target/tegra-runtime-venv/lib/python3.12/site-packages/fathomdb/__init__.py` outside source with `LD_LIBRARY_PATH`, `LIBRARY_PATH`, and `PYTHONPATH` removed. Wheel metadata was `Version: 0.8.27+tegra`.

Installed `Engine.open(..., use_default_embedder=True)` smokes in this venv returned CPU with no witness under `cpu`, CUDA with no requested witness under `auto`, and CUDA with an in-process witness under `cuda:0`. The unchanged `verify-tegra-gpu-witness.py --nvidia-smi` passed; the captured witness reports Orin capability 8.7, 384-vector forward pass, 143,618,048-byte allocation delta over the 67,108,864-byte floor, and 1,077,309,440-byte control delta. My first receipt serialization used noncanonical JSON and the verifier rejected only its formatting; canonical serialization of the same captured object then passed. This formatting correction changed no wheel or product code. The host branch checkout remained clean. No Pages workflow was dispatched.

The host has a CUDA driver, so this receipt's clean installed import and zero dynamic CUDA/NVIDIA dependencies support the driverless CPU-loadability contract structurally; it is not a measured import on a driverless Jetson. This wheel is bound to `a8c7e2f12` and remains provisional relative to the follow-up governed-pin commit and the eventual integrated SHA. The coordinator must rebuild and repeat the platform checks from the final integrated code SHA.

The branch-local smoke reports and canonical allocation witness are retained under `evidence/`; `evidence/build-wheel.log` is the exact host-native build transcript. The witness JSON SHA-256 is `75b0c4d1d3bba2318287247a9a1f69385db9ed43f925ce059bff50d3b0c4d655`.

Raw extracted-extension inspection is retained as `evidence/extension-nm-undefined.txt` (`nm -D --undefined-only`, exit 0, SHA-256 `0f342451f0b93ab50884087d7642a32aaba834034722d9ff1ef848f5f6b3a871`) and `evidence/extension-readelf-dynamic.txt` (`readelf -d`, exit 0, SHA-256 `b9f740dc8e277c0cfc727b343bfb023f4f9f9c6f60b158a3fad8b30e5584c2e5`). The dynamic output lists only `libstdc++`, `libgcc_s`, `libm`, `libc`, and `ld-linux-aarch64`; no CUDA/NVIDIA dependency appears. These raw results belong to the `a8c7e2f12` wheel. The stricter checker at `86ab7d619` was verified by regression fixtures and must be applied again to the final integrated-SHA wheel.
