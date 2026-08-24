---
title: 0.8.24 Slice 30 — package and target research disposition
status: COMPLETE
target_release: 0.8.24
---

# Slice 30 package and target research disposition

## Answered questions

### Which repository owns the work?

The owner clarified that it is this repository, `fathomadb/fathomdb`. Earlier
references to looking for another “Fathom” repository were a false branch.

### What naming convention is authoritative?

D-80.6-3 is authoritative: distribution and import remain `fathomdb`; the
Tegra build uses a PEP 440 local segment, `0.8.24+tegra`, on a first-party PEP
503 index. `fathomdb-tegra` and a separate import package are not options for
this slice.

### Why not PyPI?

The real artifact is bare `linux_aarch64`. Warehouse rejects that platform tag
under any distribution name. Renaming cannot make it PyPI-uploadable, and
relabeling it manylinux would overclaim portability and external-library
compatibility. The first-party index can honestly serve the host-bound tag.

### How is dependency confusion constrained?

pip gives no priority to `--extra-index-url`; all candidates participate in
version selection. The accepted control is therefore detection-gating plus an
exact `==0.8.24+tegra` pin. A documented uv explicit-index/source binding is a
preferred additional form. A naked floating alternate-index command is
rejected.

### What target has already been proved?

GitHub run 32296395639 succeeded on the dedicated Orin at exact commit
`59c1033e229838632b5d0fe1ecd48845f3007fa0`. It retained a
`fathomdb-0.8.22+tegra-cp310-abi3-linux_aarch64.whl` and showed CPU, auto-CUDA,
and forced-CUDA installed-wheel behavior with a validated Tegra GPU witness.
This proves the route and matrix, not 0.8.24 publication readiness.

## Primary references retained by the repository

- Python packaging version and name normalization specifications:
  <https://packaging.python.org/en/latest/specifications/version-specifiers/>
  and <https://packaging.python.org/en/latest/specifications/name-normalization/>.
- PyPA distribution/import distinction:
  <https://packaging.python.org/en/latest/discussions/distribution-package-vs-import-package/>.
- pip index behavior:
  <https://pip.pypa.io/en/stable/cli/pip_install/>.
- Simple repository API:
  <https://packaging.python.org/en/latest/specifications/simple-repository-api/>.
- NVIDIA CUDA Linux installation/target guidance:
  <https://docs.nvidia.com/cuda/cuda-installation-guide-linux/>.
- Owning repository records:
  `dev/tegra-platform-reference.md`,
  `dev/design/0.8.23-aarch64-tegra.md`, and
  `dev/plans/runs/0.8.23-slice-80-status.md`.

## Remaining research cannot precede the missing choice

The endpoint service determines deployment APIs, authentication, immutable
layout, cache behavior, and how PEP 503 pages are generated. Researching one
provider and encoding its mechanics before the owner selects it would invent
architecture. The next review must receive the concrete URL/service/owner and
then verify its primary deployment documentation, OIDC support or scoped
credential model, overwrite semantics, and simple-index conformance.
