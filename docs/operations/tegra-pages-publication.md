# Tegra Pages publication

`fathomdb==0.8.27+tegra` is an unpublished classic Jetson Orin CUDA
candidate. The exact-commit route below can publish it to the first-party
GitHub Pages index only after separate release authorization. The published
`0.8.26` generic artifacts and any earlier Tegra Pages wheel are historical.
The `+tegra` wheel is not published to PyPI; generic Linux AArch64 wheels,
tags, and PyPI/npm/crates.io artifacts are outside this procedure.

## Publish

Publication is deliberately manual: it requires an exact reviewed commit on
`release/0.8.27`, then uses the dedicated Jetson workflow. From a clean checkout on the classic Jetson host with authenticated `gh`,
run only after publication is authorized:

```bash
scripts/release/publish-tegra-pages.sh \
  --candidate-version 0.8.27 \
  --candidate-sha <remote-release-0.8.27-head>
```

The default script path includes the installed Pages-wheel smoke and only reports
completion after it passes. `--skip-pages-smoke` records an incomplete
publication check with a nonzero exit. The script refuses a SHA that is not the current remote release-branch head,
dispatches the opt-in Pages workflow, waits for it, verifies success at that
same SHA, downloads retained evidence, and runs the installed Pages-wheel
forced-CUDA smoke. The workflow builds with Maturin 1.14.1, checks that the
extension has no unresolved or dynamically linked CUDA runtime, tests CPU,
auto, and forced-CUDA paths, retains an in-process CUDA witness, and deploys
only the matching `+tegra` wheel to:

```text
https://fathomadb.github.io/fathomdb/tegra/simple/
```

## Troubleshoot

- If the wrapper rejects the host, use a classic Jetson Orin with JetPack 6 / CUDA 12.6; generic ARM64/SBSA and Thor are unsupported.
- If Maturin or CUDA checks fail, repair `scripts/release/build-python-cuda-tegra.sh` with a failing contract test first; do not preload or dynamically link `libcudart`.
- If CI fails, use the run URL printed by the script, fix the exact release-branch commit, then rerun the same command with its new SHA.
- If Pages propagation is delayed, rerun only the final script after deployment has completed; do not upload a wheel by hand.
