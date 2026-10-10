---
title: Slice 117 Jetson agent qualification handoff
status: DRAFT
target_release: 0.8.27
---

# Slice 117 — Jetson agent qualification handoff

This is the execution contract for an agent working on a classic Jetson Orin
after the `slice-117-delivery-shape` ruling and distribution ADR are accepted.
It covers local candidate qualification and the later Pages-installed smoke.
The [plan](plan.md) owns acceptance; the [design](design.md) owns why these
checks are needed. The current checkout has no Tegra Node build wrapper,
package tarball or Node smoke command. This handoff is not yet an executable
test run. The implementing agent must add the commands below, with failing
contract tests first, before a Jetson agent can use them.

## Source and host preflight

1. Use an isolated, clean checkout of the exact candidate commit. Record
   `git rev-parse HEAD`, branch/ref and `git status --porcelain=v1`. Do not
   label a local dirty build, a Slice 110 binary or a Slice 135 artifact as
   the Slice 117 candidate. The workflow dispatch requires the exact remote
   `release/0.8.27` head and matching 40-character `candidate_sha`.
2. Run the classic-Orin preflight in
   `.github/workflows/jetson-tegra-cuda-evidence.yml`: Linux `aarch64`, a
   device-tree or `/etc/nv_tegra_release` Tegra-family signal, and an
   approved absolute `nvidia-smi` reporting `nvgpu`. Record GPU name, UUID,
   driver, JetPack/L4T, `uname -srm`, available disk and the host load. A
   Thor-like or indeterminate signal fails this Jetson qualification.
3. Match `scripts/release/cuda-artifact-contract.sh`: CUDA 12.6.68 at
   `/usr/local/cuda-12.6`, GCC/G++ 11.4.0, Rust 1.95.0, Node 25.9.0 and npm
   11.12.1. Assert the offline model cache used by the wheel route exists
   (`FATHOMDB_TEGRA_HF_HOME`, otherwise `~/.cache/huggingface`). Do not accept
   a model-download or missing-cache skip as a GPU pass.
4. Keep other GPU consumers off the integrated GPU while measuring the
   allocation witness. Use a new evidence directory outside the source tree;
   retain host preflight, tool versions, source identity and command logs.

## Candidate run before publication

The implementing agent must make the following interface real. These commands
are **planned**, not available in the current checkout:

```bash
bash scripts/release/build-napi-cuda-tegra.sh \
  --base-version 0.8.27 --assert-only
bash scripts/release/build-napi-cuda-tegra.sh \
  --base-version 0.8.27 --out "$EVIDENCE_DIR/node"
bash scripts/release/smoke/smoke-tegra-node-package.sh \
  --source local --main-tarball "$MAIN_TARBALL" \
  --tegra-tarball "$TEGRA_TARBALL" --candidate-sha "$CANDIDATE_SHA" \
  --evidence-dir "$EVIDENCE_DIR/node-local-smoke"
```

The build command must assert the pinned host and feature tuple, prove the
`.node` glibc floor and CUDA linkage, pack the main and Tegra packages, and
record their SHA-256 values. The smoke must install those tarballs into a new
consumer outside the source tree with lifecycle scripts disabled, clear CUDA
library search-path overrides for loadability, and prove that the installed
Tegra addon was selected. It must exercise `cpu`, `auto` and `cuda:0` for both
embedding and reranking, require effective CUDA for forced policy, reject a
CPU fallback, and retain the policy results and GPU allocation witness. No
prerequisite or test may quietly report SKIP as PASS.

The smoke must validate its witness with
`scripts/release/verify-tegra-gpu-witness.py --nvidia-smi`. It must also run
the existing heap-growth regression against its installed main package:

```bash
FATHOMDB_REQUIRE_LIVE=1 \
FATHOMDB_TEGRA_NODE_PACKAGE="$CONSUMER/node_modules/fathomdb" \
  bash scripts/tests/test_tegra_node_early_cuinit.sh
```

The smoke owns setting `CONSUMER` to its fresh install root and retaining the
heap-test output. `FATHOMDB_REQUIRE_LIVE=1` makes missing live prerequisites
fail. On another Jetson model, the existing heap test excludes the host
because its 64 GiB AGX Orin failure size is not established; report that
limit instead of claiming the heap row passed.

Run the full `./scripts/agent-verify.sh` gate at the exact source used to
build the artifacts, with Rust, Python, TypeScript and Markdown tooling
installed. Record every pass, skip and failure; a skipped required component
is not a full gate. Run the applicable CUDA-feature and publication-contract
checks identified in the plan and confirm `actionlint` on the changed
workflow. Record independent review and the exact install docs before the
Slice 140 handoff. Do not rerun all Slice 135 cells unless this candidate
changed query or retrieval behavior beyond loader and packaging; if it did,
name and repeat the affected paired cells at this candidate source.

After the reviewed candidate is available as the remote release-branch head,
the canonical prepublication receipt is the manual `Jetson Tegra CUDA
evidence` workflow at that exact SHA with `candidate_version=0.8.27` and
`publish_to_pages=false`. Its Jetson job must run the same Node rows and retain
the wheel and tarball in one run. Local shell results are useful during
development; they do not replace this exact-SHA workflow receipt.

## Checks outside the Jetson

The Jetson cannot alone prove generic Linux ARM64 behavior or a driverless
AArch64 import. The release workflow and a driverless ARM64 runner must prove
the generic package retains its hosted build, default-only features and 2.28
glibc floor, the loader ignores the Tegra package on non-Tegra and Thor-like
hosts, and a fresh installed Tegra addon imports without a CUDA driver.
Contract tests must prove that `npm-inject-optional-deps.sh` never adds the
Tegra package and both Pages deployers retain every manifest-listed wheel and
tarball. Attach these off-host receipts to the same candidate SHA; do not
claim them from a Jetson-only run.

## After authorized Pages publication

The operator uses `scripts/release/publish-tegra-pages.sh` only after the
separate release publication ruling. The extended script must install the
main package from npm and the Tegra tarball from its exact Pages URL in a
fresh Jetson consumer, compare the downloaded SHA-256 with retained candidate
evidence, then repeat forced-CUDA embedding and reranking, the GPU witness
and the installed-package heap test. Retain the Pages deployment run, URL,
downloaded hash, smoke logs and witness. A skipped or failed post-publication
smoke leaves Slice 117 INCOMPLETE.

## Receipt to return

Return one concise receipt with: candidate commit and clean-source proof;
host/toolchain identity; workflow run or local command log paths; main,
Tegra `.node` and tarball SHA-256 values; CPU/auto/forced-CUDA embed and
rerank outcomes; witness verification; heap repetitions; loader and generic
ARM64 results; driverless result; full-gate and review results; exact install
docs; and each limitation or skip. After publication append the Pages URL,
deployment run, downloaded digest and repeated smoke outcomes. Keep Slice
135's pinned-source measurements and Slice 150's final-candidate qualification
in their own receipts.
