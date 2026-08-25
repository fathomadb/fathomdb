---
title: 0.8.24 Slice 30 — corrected Tegra publication design
status: IMPLEMENTING
target_release: 0.8.24
---

# Slice 30 corrected Tegra publication design

## Design-review disposition

The independent review rejected the draft's distinct `fathomdb-tegra`
distribution / separate-import premise because it contradicts accepted
D-80.6-3. This document adopts the existing architecture and narrows the open
decision to transport and deployment. The owner selected an interim GitHub
Pages route on 2026-08-25; implementation is intentionally limited to 0.8.24
and must be re-reviewed before a later Tegra release chooses durable hosting.

## Package topology

| Consumer | Distribution | Version | Source | Platform |
| --- | --- | --- | --- | --- |
| Generic CPU | `fathomdb` | `0.8.24` | PyPI | Existing supported CPU wheels |
| Jetson Orin CUDA | `fathomdb` | `0.8.24+tegra` | `https://fathomadb.github.io/fathomdb/tegra/simple/` | `cp310-abi3-linux_aarch64` |

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

The endpoint is the interim GitHub Pages base above. `--extra-index-url` may be
shown only after classic-Tegra confirmation because pip does not prioritize
indexes. A uv explicit-index/source mapping should be documented alongside it
to bind only `fathomdb` to the first-party source. A later generic PyPI upgrade
can displace the Tegra build; the existing version-aware warning/doctor path
must provide the exact repairing command for this endpoint.

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

The workflow was release-pinned to 0.8.23. Slice 30 encodes one bounded 0.8.24
ref contract for both jobs that
admits the authorized `release/0.8.24` candidate, rejects unrelated refs, and
preserves the exact `candidate_sha == github.sha` identity check. A broad
all-branches relaxation is outside the design.

## Trust and artifact transfer

The target-controlled Jetson builder must remain credentialless. Its output is
an immutable Actions artifact plus digest. A separate hosted publisher consumes
that exact artifact, verifies `Name: fathomdb`, exact `Version`, filename, and
SHA-256, and deploys the static PEP 503 tree only through the `github-pages`
environment with `pages: write` and `id-token: write`. The Jetson has no
publication credential. Pages redeploys are explicit owner-authorized actions,
not a claim of immutable multi-version storage.

## Architectural fit

- Aligns with D-80.6-3 and D-80.7 detection/displacement behavior.
- Preserves the current Python module layout and avoids shared-file ownership
  corruption from a sibling distribution.
- Preserves generic CPU packaging, npm, Rust targets, and existing release
  idempotency.
- Uses the proven Jetson-native artifact and evidence boundary.
- Requires a narrow workflow compatibility correction before any 0.8.24 target
  evidence claim; a skipped job is never a passing target proof.
- Keeps Pages deployment outside target-controlled code.

## Deferred hosting/distribution review

1. Choose durable multi-version artifact retention and endpoint/domain policy
   before a later Tegra release.
2. Re-evaluate whether Pages should continue to serve wheels or only index
   immutable release assets.
3. Bind artifact name, SHA-256, source SHA, workflow identity, and environment
   across build and publisher jobs without trusting mutable metadata.
4. Ensure installation docs never expose a floating alternate-index command to
   generic AArch64 users.

These are re-review inputs, not authorization to configure a provider.
