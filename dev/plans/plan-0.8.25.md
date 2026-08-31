---
status: ACTIVE
target_release: 0.8.25
---

# 0.8.25 plan — driverless CUDA-capable Linux artifacts

## Objective

Correct the immutable 0.8.24 Linux x86_64 CUDA-capable artifact defect: normal
Python and npm packages must be usable in CPU mode on a host with no CUDA
runtime or driver. No tag, registry publication, or release promotion is in
this plan.

## Deliverables

- A reviewed pin to a FathomDB-maintained Candle revision for core, nn,
  transformers, and `candle-kernels`, with no dynamic CUDA/NVIDIA dependency
  in any shipped Linux x86_64 ELF member.
- Driverless Python and npm installed-package smokes that mount no CUDA runtime
  files and prove unset/`auto` plus explicit-CPU lifecycle behavior.
- Complete archive/ELF dependency assertions and retained evidence for Python
  and N-API, plus retained forced-CUDA, explicit-CPU non-use, and selected-GPU
  evidence.
- Requirement, acceptance, architecture, ADR, and design records aligned with
  the corrected contract.

## Quality process

The implementation starts with failing contract tests (RED), turns green only
against actual generated CUDA artifacts, and refactors only after green. An
independent design review is required before fork/pin changes. An independent
code review follows implementation; address findings and re-review for up to
three FIX-n cycles. The final verification includes the real trusted GPU runner
and a genuinely driverless container, not only static workflow tests.

## Release boundary

The corrective version must be new because npm, PyPI, and crates.io entries are
immutable. Version selection, tagging, publication, and post-publish release
closure require separate HITL direction.
