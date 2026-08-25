---
title: 0.8.24 Slice 40 — Windows CUDA draft design
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 Windows CUDA draft design

## Design status

**Conditional design only.** The architecture is deliberately incomplete until
the owner selects the SDK surface and executor. This prevents a CPU package
identity or a generic Windows runner from becoming an accidental CUDA contract.

## Common architecture

```text
immutable candidate SHA
        |
        v
main-owned Windows GPU build harness  --> toolchain/GPU/DLL/build manifest
        |                                             |
        +--> sealed selected artifact + SHA-256 ------+
                              |
                              v
               hosted publisher verifies sealed input
                              |
                              v
        fresh Windows GPU install -> lifecycle + CUDA witness
                              |
                              v
                   Slice 60 candidate/post-publish matrix
```

The GPU builder has no registry credential. The hosted publisher receives only
the sealed artifact and verifies the manifest/digests. The source ref and
privileged harness remain main-owned; candidate code cannot redefine the
publisher boundary.

## Candidate surface options

| Selection | Design consequence |
| --- | --- |
| Python only (recommended minimum) | Propose a distinct first-party PEP 503 exact-local-version CUDA route while retaining the `fathomdb` import and existing CPU PyPI wheel. The exact name/index/version remains owner-selected. |
| npm only | Define an additional CUDA package and a deterministic loader policy. An ADR must state package identity, selection precedence, force/auto behavior, and CPU compatibility. |
| Both | Apply both contracts independently; neither surface implies the other. Their target evidence and publication rows remain separate. |

No option may overwrite the existing CPU Windows wheel or
`fathomdb-native-win32-x64-msvc` bytes. The existing CPU optional-dependency
loader must remain deterministic for non-CUDA users.

## Executor acceptance design

The proposed executor becomes usable only after a retained observation names
its GitHub selector/state, OS, GPU/compute capability, driver, CUDA toolkit,
MSVC/SDK, Rust, and selected runtime versions. It must build the immutable
candidate, inspect dependencies, seal artifacts, and run the clean installed
GPU smoke. The local VM and hosted CPU jobs intentionally fail this acceptance
test.

## Error and verification policy

- Unsupported platform, missing selected artifact, and forced-CUDA on a
  CPU-only artifact must fail clearly and be tested.
- Candidate proof records package identity/version/digest, source SHA,
  executor/toolchain/GPU facts, install command, lifecycle output, and device/
  process evidence.
- Slice 60 subsequently verifies public exact-version installation and CPU
  publisher preservation. Slice 70 uses that evidence; neither performs the
  missing feature design.

## Architecture review result

The current architecture fits a separate CUDA distribution/selection route,
not a mutation of existing CPU artifacts. Python-only has the smallest new
public selection surface; npm/both expands public loader semantics and needs an
ADR-level decision. No implementation should proceed before those choices and
the executor evidence exist.
