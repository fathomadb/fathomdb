---
title: 0.8.24 Slice 40 — Windows x64 CUDA distribution
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 — Windows x64 CUDA distribution

## Reviewed disposition

**REVIEWED — BLOCKED / READY FOR OWNER DECISION.** This slice has completed
its discovery, draft-contract, and architectural-fit work. It must not begin
product, packaging, workflow, executor, or publication work until both owner
inputs below are recorded:

1. **P24-09:** select the supported Windows CUDA SDK surface: Python, npm, or
   both, including unsupported-route behavior.
2. **P24-10:** name a real remote Windows x64 CUDA executor and approve its
   build-to-publisher trust boundary.

No local Windows compilation is required or authorized. A hosted
`windows-latest` CPU build and the local Windows VM are validation resources,
not substitutes for the approved CUDA executor.

## Goal

Deliver the owner-selected prebuilt Windows x64 CUDA SDK surface without
changing existing Windows CPU identities or requiring an end user or release
operator to compile Windows CUDA locally. A completed route supplies Slice 60
with sealed candidate bytes and a clean installed-package Windows GPU smoke.

## Established facts

| Area | Existing / observed | Net new after decisions |
| --- | --- | --- |
| Python | `fathomdb` is a Windows `x86_64-pc-windows-msvc` wheel in the ordinary CPU matrix. | A distinct CUDA artifact identity/selection and its build/proof path, if Python is selected. |
| npm | Thin `fathomdb` resolves the CPU package `fathomdb-native-win32-x64-msvc` through version-pinned optional dependencies. | A CUDA package/loader topology, if npm is selected. |
| CI | Hosted Windows builds, publishes, and post-publish CPU smokes exist. | A remote Windows CUDA build and target smoke route. |
| Runner inventory | Existing self-hosted CUDA route is Linux; no observed/approved Windows CUDA builder, GPU, toolkit, or transfer route exists. | An observed and approved Windows builder contract. |
| Local Windows VM | The local VM is a Windows CPU validation environment with virtual display and no NVIDIA host-device passthrough. | It cannot supply CUDA build or runtime proof. |

The current npm CPU name is accepted by ADR-0.8.22. It cannot be silently
renamed, replaced, or treated as a CUDA variant.

## Recommended decision shape

The minimum proposal is **Python-only**, using a first-party PEP 503 route and
an exact Python local version for the Windows CUDA artifact. This is a
recommendation for owner review, not a selected or final identity. It has the
smallest public surface because Python can retain the existing `fathomdb`
import while explicit selection avoids replacing the CPU artifact at PyPI.

The selected CUDA installation is an **exclusive CPU/CUDA replacement or
upgrade in one environment**, never a co-install with the CPU `fathomdb`
artifact. Installation must use the complete exact local-version pin and only
the selected first-party index/mapping. A floating `--extra-index-url` route
and upload of the Windows CUDA wheel to PyPI are forbidden. The generic CPU
artifact remains separately available from PyPI.

Selecting npm, or both SDKs, requires a separately reviewed package identity
and deterministic loader policy. The current npm loader has one Windows x64
CPU package identity. A second CUDA package is not selected merely by adding
another matching optional dependency; precedence, forced/automatic behavior,
co-install/upgrade behavior, and unsupported errors become public contract.
That is ADR-level work before implementation.

## Required executor and artifact boundary

P24-10 must provide an executor record with all of the following before any
implementation or candidate dispatch:

- an explicitly trusted, owner-approved Windows GPU builder: either an
  owner-operated external builder that is not registered with Actions, or an
  Actions builder restricted to a dedicated runner group for the selected
  repository and selected workflow only. Labels are routing metadata, not
  access control;
- Windows version/architecture; NVIDIA GPU model and compute capability;
  driver and CUDA toolkit; MSVC/Windows SDK; Rust; and selected Python/Node
  versions;
- a main-owned control plane that accepts an immutable candidate SHA rather
  than candidate-controlled privileged workflow logic. An Actions builder
  never runs for a pull request, fork, or `pull_request_target`, and carries
  no secrets, OIDC token, or publishing credential;
- dependency/DLL inspection, build manifest, artifact SHA-256 values, and
  retained GPU runtime witness; and
- a credentialless build-to-hosted-publisher transfer. Registry credentials,
  OIDC environment claims, and publisher authority stay in a reviewed hosted
  job, never on the GPU builder.

The same executor must perform the clean candidate-installed lifecycle proof:
install sealed bytes in a fresh environment; open, write, search, close, and
exit; then prove device resolution selected the same GPU UUID that has the
active computation process PID. Retain the selected UUID, GPU model, driver,
toolkit, candidate SHA, artifact digest, and PID evidence. A compile, upload,
bare `nvidia-smi`, or a process on a different GPU fails this proof.

## Work after owner decisions

1. Record P24-09 and P24-10 in `decision.md`; promote only the chosen draft
   contract. Do not edit canonical needs, requirements, or acceptance before
   that decision.
2. Write the selected architecture decision/ADR and public interface/install
   contract. Add RED package identity, loader, unsupported-route, manifest,
   and CPU-preservation tests.
3. Implement the smallest selected Python and/or npm route GREEN. Preserve the
   existing CPU wheel and `fathomdb-native-win32-x64-msvc` package unchanged.
4. Add a main-owned, credentialless remote build route and sealed artifact
   handoff. Run source/workflow and package inspections locally.
5. On the approved Windows GPU executor, build the selected artifact and run
   the candidate-installed smoke. No local VM or hosted CPU substitution.
6. Hand Slice 60 the exact identity, digest, executor record, install command,
  unsupported behavior, and smoke evidence including selected UUID and active
  PID correlation. Slice 60 owns post-publication installed proof and
  publisher-preservation matrix; Slice 70 owns final release integration and
  owner-authorized publishing.

## Verification

- RED/GREEN structural tests for selected package identity, selection, errors,
  CPU preservation, workflow privilege split, and artifact manifest.
- `actionlint`, relevant PowerShell/shell checks, package metadata inspection,
  and documentation/plan lint for the changed files.
- On the approved executor only: CUDA/toolchain/DLL evidence, sealed digest,
  and fresh installed-artifact GPU lifecycle smoke whose selected UUID matches
  the GPU UUID with the active computation PID.

## Non-goals and stop conditions

- No local Windows compilation, VM passthrough work, runner registration,
  hosted Windows CPU substitution, package publication, or GitHub setting
  change in this planning state.
- If P24-09 or P24-10 is absent, retain this slice as blocked; do not infer a
  combined SDK surface or an executor from labels, VM availability, or a
  generic hosted job.
- If the selected identity or loader changes a public platform contract,
  create the required ADR/interface/docs before code.

## Definition of done

Slice 40 is complete only when the owner-selected Windows CUDA surface is
implemented under TDD; existing CPU artifacts remain intact; an approved remote
Windows CUDA executor has produced provenance-bound bytes; and a clean
installed-package Windows GPU smoke has passed. Publication and
registry-installed smoke remain Slice 60/70 work and require separate owner
authorization.
