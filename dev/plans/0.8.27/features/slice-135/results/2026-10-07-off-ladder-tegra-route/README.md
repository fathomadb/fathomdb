---
title: Slice 135 off-ladder Tegra install-route focused qualification
status: PARTIAL_PHASE1_CHECK
target_release: 0.8.27
---

# Published Tegra install route — focused Phase 1 check

The integrated Slice 135 candidate `db4348bdc08fb386eee3407381abfd062e63e016`
contains the release commit `b65283317`, its base `ef4bb42da`, the
embedder-close merge `96796fe04`, and the Tegra red/fix commits `5ceab8624`
and `c23e2d23f`. The 0.8.27 docs Pages workflow retains the already
published `0.8.26+tegra` wheel
`fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl` with pinned SHA-256
`728df8628f45e12c0c04c0a971c174764ef5c988c7d05f0737794aa6f5c5d1fa`.
The Python classic-Tegra warning and `fathomdb doctor gpu --help` recommend
`'fathomdb==0.8.26+tegra'` from the first-party Pages index.

| Focused check | Result | Retained output |
| --- | --- | --- |
| `bash scripts/tests/test_pages_site.sh` | 19 pass, zero fail; workflow pin, hash, URL and publication route checked | [pages-site.log](pages-site.log) |
| `PYTHONPATH=src/python PYTHONDONTWRITEBYTECODE=1 python3 -m pytest -p no:cacheprovider -q src/python/tests/test_coinstallation_guard.py` | 15 pass; classic-Tegra warning includes exact version | [python-warning.log](python-warning.log) |
| `cargo test -p fathomdb-cli --test operator_cli doctor_gpu_help_contains_the_exact_tegra_pages_install_procedure -- --nocapture` from `src/rust/` | One pass, 27 filtered; exact CLI help command checked | [cli-help.log](cli-help.log) |
| `actionlint .github/workflows/docs-pages.yml` | Pass; empty stdout/stderr | [actionlint.log](actionlint.log) |

The host is x86_64, so this receipt does **not** install or run the AArch64
CUDA wheel and cannot validate the live Pages index or GPU behavior. Slice 150
must run the final installed-artifact and Tegra-route smoke using the
`0.8.26+tegra` route. Publication of a later 0.8.27 Tegra wheel updates the
pin again. [SHA256SUMS](SHA256SUMS) seals the focused output.
