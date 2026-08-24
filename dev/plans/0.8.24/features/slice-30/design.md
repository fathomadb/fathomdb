---
title: 0.8.24 Slice 30 — corrected Tegra publication design
status: REVIEWED-BLOCKED
target_release: 0.8.24
---

# Slice 30 corrected Tegra publication design

## Design-review disposition

The independent review rejected the draft's distinct `fathomdb-tegra`
distribution / separate-import premise because it contradicts accepted
D-80.6-3. This document adopts the existing architecture and narrows the open
decision to transport and deployment. The design is architecturally aligned
but cannot become READY until the concrete first-party index and publisher
route are declared and re-reviewed.

## Package topology

| Consumer | Distribution | Version | Source | Platform |
| --- | --- | --- | --- | --- |
| Generic CPU | `fathomdb` | `0.8.24` | PyPI | Existing supported CPU wheels |
| Jetson Orin CUDA | `fathomdb` | `0.8.24+tegra` | First-party PEP 503 endpoint, TBD | `cp310-abi3-linux_aarch64` |

There is one import package and one distribution identity. Installing one
version replaces the other normally; there are never two distributions
claiming the same files. The retained `fathomdb-tegra` co-install tripwire is
only for locally invented sibling packages and is not public topology.

## Supported target and failure boundary

The public target is classic Jetson Orin (`sm_87`), AArch64, L4T R36 / JetPack
6, CUDA 12.6, Ubuntu 22.04, glibc 2.35, Python abi3. The measured reference host
is L4T R36.5.2, nvcc 12.6.68, GCC 11.4, driver 540.5.0.

Generic ARM64/SBSA and Thor are detected as non-classic Tegra and never receive
the install endpoint. Unsupported JetPack/CUDA generations receive no
compatibility claim. The bare Linux tag is intentional and never submitted to
PyPI.

## Selection and repair contract

The only safe pip guidance is an exact requirement and a detection-gated
alternate source:

```text
fathomdb==0.8.24+tegra
```

The endpoint portion remains absent until declared. `--extra-index-url` may be
shown only after classic-Tegra confirmation because pip does not prioritize
indexes. A uv explicit-index/source mapping should be documented alongside it
to bind only `fathomdb` to the first-party source. A later generic PyPI upgrade
can displace the Tegra build; the existing version-aware warning/doctor path
must provide the exact repairing command once the endpoint exists.

## Build and provenance

The dedicated Jetson job checks out the immutable candidate with persisted
credentials disabled, asserts the host/toolchain contract, builds with
`build-python-cuda-tegra.sh`, verifies `+tegra` metadata and glibc 2.35, installs
the wheel into a clean environment, runs CPU/auto/forced-CUDA policies, validates
the in-process witness, hashes the wheel, and uploads retained evidence.

Existing proof: run 32296395639 passed on the dedicated runner and retained the
host, source, wheel, digest, and three policy results. That is confirmed runner
and functionality evidence. A new 0.8.24 candidate run is needed only after
implementation; historical evidence is not repurposed as release evidence.

The workflow is currently release-pinned, not reusable as-is: both its
validation and evidence jobs require `refs/heads/release/0.8.23`. The Slice 30
implementation must first encode one bounded ref contract for both jobs that
admits the authorized `release/0.8.24` candidate, rejects unrelated refs, and
preserves the exact `candidate_sha == github.sha` identity check. A broad
all-branches relaxation is outside the design.

## Trust and artifact transfer

The target-controlled Jetson builder must remain credentialless. Its output is
an immutable Actions artifact plus digest. A separate hosted publisher consumes
that exact artifact and deploys only to the selected first-party PEP 503
service. The publisher's environment, endpoint, authentication, immutable-path
rules, and retry behavior are parameters awaiting owner decision.

No current repository/GitHub setting supplies them: no Actions variable names
exist; environments are only `pypi` and `cuda-unmerged-preflight`; repository
secret names are only `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`. The `pypi`
environment is for PyPI CPU publication and is not silently reused for a
different service.

## Architectural fit

- Aligns with D-80.6-3 and D-80.7 detection/displacement behavior.
- Preserves the current Python module layout and avoids shared-file ownership
  corruption from a sibling distribution.
- Preserves generic CPU packaging, npm, Rust targets, and existing release
  idempotency.
- Uses the proven Jetson-native artifact and evidence boundary.
- Requires a narrow workflow compatibility correction before any 0.8.24 target
  evidence claim; a skipped job is never a passing target proof.
- Keeps endpoint/service-specific deployment outside target-controlled code.

## Challenging aspects after endpoint selection

1. Prove the service emits PEP 503-normalized project pages and immutable file
   links for `fathomdb` without conflating it with PyPI ownership.
2. Prove authentication can be limited to the selected path/project and that a
   retry cannot overwrite published bytes.
3. Bind artifact name, SHA-256, source SHA, workflow identity, and environment
   across build and publisher jobs without trusting mutable metadata.
4. Ensure installation docs never expose a floating alternate-index command to
   generic AArch64 users.

These are re-review inputs, not authorization to configure a provider.
