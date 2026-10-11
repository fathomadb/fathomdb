# Tegra Pages publication

The 0.8.27 release delivers `fathomdb==0.8.27+tegra` and the matched-version
`fathomdb-linux-arm64-gnu-tegra` Node tarball through first-party GitHub Pages
for classic Jetson Orin. The wheel is not published to PyPI and the tarball is
not published to npm. This exact-commit procedure performs that publication
only after release authorization; it does not publish generic artifacts or
registry packages.

## Retained artifacts

Every Pages deployment rebuilds the whole site, from both the docs deployer
(`docs-pages.yml`) and this route. Each published Tegra artifact therefore
stays online only while `scripts/release/tegra-pages-retained.json` names it
with its URL and SHA-256. Both deployers download every listed artifact,
check its digest, and republish it.

Before deploying, each runs
`scripts/release/check-tegra-pages-retention-live.sh`. It reads the live
`tegra/simple/fathomdb/` and `tegra/node/` index pages over https and fails
if any artifact they link is missing from the list and was not added by
this run, or is linked with a different SHA-256. A deployment from a branch
whose list lags the live site therefore stops instead of withdrawing
artifacts. An index page that answers 404 is accepted only while the list
holds no artifact of that kind, which is the first deployment of that kind.
Otherwise the 404 means the live site is already damaged, and the
deployment stops. Withdrawing a live artifact on purpose needs a separate
decision. Name each withdrawn filename, comma-separated, in the
`withdraw_tegra_artifacts` input of a manual dispatch of either workflow. A
push-triggered docs deployment cannot withdraw anything. The run summary
records each withdrawal.

After deploying, each runs
`scripts/release/verify-tegra-pages-live.sh`. It downloads every listed
artifact, plus any the run added, from its public URL, checks each SHA-256,
and checks that the index pages link each one.

The list records previously published artifacts by exact URL and digest.
After a publication, the script prints the new wheel and tarball entries; add
them to the list before the next deployment. Do not add future artifacts to
the retained list before their publication is verified.

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
  fresh installed consumer. Forced-CUDA embedding must yield a GPU allocation
  witness; forced-CUDA reranking must resolve to CUDA at open and return
  finite scores, without falling back to CPU;
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
  Jetson build digest before installing. It then repeats forced-CUDA
  embedding with its GPU allocation witness, forced-CUDA reranking (checked
  by its open-time device resolution and finite scores, not a witness) and
  the heap-growth check.

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
