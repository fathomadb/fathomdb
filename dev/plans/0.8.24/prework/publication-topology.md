---
title: 0.8.24 Slice 0 — publication topology finding
status: COMPLETE
target_release: 0.8.24
---

# Publication topology finding

**Observed:** 2026-08-23. Public registry queries are metadata-only.

## Existing artifacts

| Surface | Source identity | Public observation |
| --- | --- | --- |
| Python | `fathomdb==0.8.23` | PyPI reports 0.8.23 present and current. |
| npm main | `fathomdb@0.8.23` | `latest` and `next` are 0.8.23. |
| npm Linux x64 | `fathomdb-linux-x64-gnu@0.8.23` | 0.8.23 is `next`; `latest` remains 0.8.20. |
| npm Linux ARM64 | `fathomdb-linux-arm64-gnu@0.8.23` | 0.8.23 is `next`; `latest` remains 0.8.21. |
| npm Windows CPU | `fathomdb-native-win32-x64-msvc@0.8.23` | 0.8.23 is `next`; `latest` is the bootstrap 0.8.22 package. |
| Cargo workspace | Tiered crates | Public crates.io query was unavailable: HTTP 403 policy response; do not infer versions from it. |

The source names five npm platform packages: Linux x64/ARM64, macOS x64/ARM64,
and Windows x64 MSVC. The thin main package injects their exact versions as
optional dependencies immediately before publication.

## Tegra conclusion

The current Tegra script deliberately creates a host-bound wheel named
`fathomdb-<base>+tegra-...whl`, uses `--compatibility linux` and
`--auditwheel skip`, then performs local proof. It has no upload operation.
This remains an appropriate local build/proof form, but is not the public PyPI
shape for 0.8.24.

At query time, PyPI returned HTTP 404 for `fathomdb-tegra`; it is currently
unregistered. That is evidence for a separately named distribution candidate,
not a reservation or authorization to publish it. Publishing such a project
would require its own trusted-publisher configuration.

Uploading two same-name `fathomdb` Linux ARM64 wheels—one CPU and one
CUDA-capable—does not encode CUDA preference in pip's platform tags. Slice 0
therefore rules out that ambiguous shape for the release proposal.

## Existing idempotency controls

| Registry | Mechanism | Safety property |
| --- | --- | --- |
| PyPI | `pypa/gh-action-pypi-publish` with `skip-existing: true`; local `pypi-publish-if-new.sh` equivalent | Existing files are no-ops; query/upload endpoint split is explicit. |
| npm | `npm-publish-if-new.sh` | Queries the exact version; a retry skips it. A registry error fails closed rather than blindly publishing. |
| crates.io | `cargo-publish-if-new.sh` | Existing versions skip; query/publish registry divergence fails closed. |

The existing release workflow names the GitHub environment `pypi` and uses OIDC
for PyPI and npm jobs. Slice 0 could inspect the workflow, but GitHub API rate
limiting prevented confirmation of remote environment rules and secret names.
No secret value was requested or read.

## Slice 0 disposition

The separate PyPI distribution is the technically sound leading Tegra option.
Its final name, whether an alternate public index is preferable, and the
trusted-publisher configuration remain owner decisions for Slice 6. Existing
CPU lanes and all three idempotency mechanisms are preservation requirements
for Slice 60.

## Evidence

- `scripts/release/build-python-cuda-tegra.sh:150-237`.
- `.github/workflows/release.yml:1185-1525`.
- `scripts/release/{pypi,npm,cargo}-publish-if-new.sh`.
- `npm view` for the listed packages and PyPI JSON queries on 2026-08-23.
