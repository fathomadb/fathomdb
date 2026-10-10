# Tegra Pages publication

`fathomdb==0.8.27+tegra` and the `fathomdb-linux-arm64-gnu-tegra` 0.8.27 Node
tarball are an unpublished classic Jetson Orin CUDA candidate. The
exact-commit route below can publish both, from one Jetson run, to the
first-party GitHub Pages site only after separate release authorization. The
published `0.8.26` generic artifacts are historical. Neither Tegra artifact
is published to PyPI or the npm registry. Generic Linux AArch64 wheels and
npm packages, tags, and PyPI/npm/crates.io artifacts are outside this
procedure.

## Retained artifacts

Every Pages deployment rebuilds the whole site, from both the docs deployer
(`docs-pages.yml`) and this route. Each published Tegra artifact therefore
stays online only while `scripts/release/tegra-pages-retained.json` names it
with its URL and SHA-256. Both deployers download every listed artifact,
check its digest, and republish it. Removing a published artifact needs a
separate decision.

Today the list holds one artifact, the `fathomdb-0.8.26+tegra` wheel. No Node
tarball has been published yet. After a publication, the script prints the
new wheel and tarball entries; add them to the list before the next
deployment.

## Publish

Publication is deliberately manual: it requires an exact reviewed commit on
`release/0.8.27`, then uses the dedicated Jetson workflow. From a clean
checkout on the classic Jetson host with authenticated `gh`, run only after
publication is authorized:

```bash
scripts/release/publish-tegra-pages.sh \
  --candidate-version 0.8.27 \
  --candidate-sha <remote-release-0.8.27-head>
```

The script refuses a SHA that is not the current remote release-branch head,
dispatches the opt-in Pages workflow, waits for it, verifies success at that
same SHA, and downloads the retained evidence.

In one run, the workflow:

- builds the `+tegra` wheel with Maturin 1.14.1 and checks that the
  extension has no unresolved or dynamically linked CUDA runtime;
- tests CPU, auto and forced-CUDA paths and retains an in-process CUDA
  witness;
- builds and packs the Tegra Node package, then smokes every policy from a
  fresh installed consumer;
- on a hosted, driverless, non-Tegra ARM64 runner, installs that tarball
  without a CUDA driver, imports it, and checks that the loader ignores it;
- deploys the matching wheel and tarball beside every retained artifact.

The site serves:

```text
https://fathomadb.github.io/fathomdb/tegra/simple/
https://fathomadb.github.io/fathomdb/tegra/node/
https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz
```

The script then runs two post-publication smokes on the Jetson:

- the installed Pages-wheel forced-CUDA smoke;
- a Node smoke that installs `fathomdb@0.8.27` from npm and the tarball from
  its Pages URL. It checks the downloaded tarball's SHA-256 against the
  Jetson build digest before installing. It then repeats the forced-CUDA
  embedding and reranking, GPU witness and heap-growth checks.

It reports completion only after both pass. `--skip-pages-smoke` records an
INCOMPLETE publication check with a nonzero exit. A failed smoke also reports
INCOMPLETE.

## Troubleshoot

- If the wrapper rejects the host, use a classic Jetson Orin with JetPack 6 /
  CUDA 12.6; generic ARM64/SBSA and Thor are unsupported.
- If Maturin or CUDA checks fail, repair
  `scripts/release/build-python-cuda-tegra.sh` or
  `scripts/release/build-napi-cuda-tegra.sh` with a failing contract test
  first; do not preload or dynamically link `libcudart`.
- If the driverless ARM64 import fails, read its retained
  `tegra-node-driverless-arm64` evidence. Do not publish around it.
- If CI fails, use the run URL printed by the script, fix the exact
  release-branch commit, then rerun the same command with its new SHA.
- If Pages propagation is delayed, rerun only the final script after
  deployment has completed; do not upload a wheel or tarball by hand.
