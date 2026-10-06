---
title: FathomDB 0.8.27 Slice 130 - Python SDK decomposition plan
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
---

# Slice 130 - Python SDK decomposition

## Draft reconciliation

| Change or assigned item since the release draft | Decision |
| --- | --- |
| Slices 30 and 100 established the Python public and native baselines. Slice 100 split the PyO3 binding and corrected subscriber delivery. | Use those authorities; do not reopen native registration, subscriber behavior, or the immutable Slice 30 baseline. |
| Slice 90 added immutable runtime configuration. Slice 120 split the TypeScript root into private write, search, graph, evidence, projection, open, instrumentation, and admin owners, while retaining a thin `Engine`. | Preserve configuration semantics. Give Python's remaining facade logic comparable domain owners without forcing an identical module taxonomy or new public imports. |
| The current `engine.py` has 2,868 lines: about 1,800 lines of conversion/validation helpers and 940 lines of `Engine` methods. `read.py`, `graph.py`, `admin.py`, `filter.py`, `config.py`, `types.py`, and `errors.py` already own their public concerns. | Move the root-owned helpers and substantial method bodies to narrow private domains. Keep one `Engine` identity, signatures, lifecycle wiring, and existing public-domain modules. |
| Existing tests directly import several private `engine.py` mappers. The Python package has a literal `__all__`; the native stub and documented namespace imports are contractual. | Retain private mapper aliases needed by existing tests, without promoting them to package exports. Compare imports, signatures, `__all__`, stub, errors, and representative runtime behavior at the package boundary. |
| Slice 132 owns Rust SDK parity, Slice 140 final architecture/documentation convergence, and Slice 150 integrated qualification. No unresolved item from Slice 120 was allocated to 130. | Keep this a behavior-preserving Python source move. Do not add capabilities, Rust changes, schema changes, package subpaths, or release-wide qualification work. |

The draft is **approved with these adjustments**. `dev/acceptance.md` stays
locked; the IDs below are slice-local. The public interface is held constant,
so the design records internal ownership rather than changing the interface.
The first `gpt-6.1-sol` high design review found two P2 gaps, both corrected;
the subsequent `gpt-6-sol` high review passed without a blocker.

## Needs, requirements, and acceptance

| ID | Need and requirement | Acceptance |
| --- | --- | --- |
| R27-130A | Maintainers can locate Python SDK behavior by concern. | AC27-130A: `engine.py` contains the sole `Engine` facade, public signatures/docstrings, and lifecycle wiring; substantial validation, mapping, and operation bodies have named private owners. No product test asserts filenames or line counts. |
| R27-130B | Existing Python consumers retain the same surface and behavior. | AC27-130B: package root and documented namespace imports, literal `__all__`, callable signatures, exception identities, native-stub agreement, and pre-move public-surface comparator rows remain equal. Focused error/wire/functional fixtures and executable public examples pass. |
| R27-130C | Python and TypeScript SDKs remain similarly organized at the concern level. | AC27-130C: the design maps write, search, graph, evidence, projection, open/configuration, and instrumentation to owners, reusing existing public modules where appropriate. No additional public facade or database semantics are introduced. |
| R27-130D | The refactor is reviewable and verified. | AC27-130D: a non-vacuous pre-move characterization guard is RED under a plausible defect and GREEN before movement; focused checks protect each batch; independent design, code, and verification reviews cover the actual candidate; required source-change gates pass. |

## RED / GREEN / REFACTOR implementation

1. Record the exact base SHA, public surface and relevant focused test results.
   Add a package-boundary characterization check for the root, documented
   namespace imports, `__all__`, signatures, and exception identity. Show it
   fails under a temporary plausible export or delegation defect; restore the
   source before the move. Do not edit tests to accommodate a failure.
2. Move related private helpers and substantial facade method bodies into
   private domain modules in small batches. Keep public `Engine` methods as
   explicit, signature-preserving delegates. Keep existing test-imported
   private mappers reachable from `engine.py`; do not expose new public names.
   Run owner tests and type/lint checks after each batch (GREEN).
3. Refactor only within the new boundaries after GREEN. Preserve validation
   order, native arguments, synchronous exceptions, object identity, and
   return mapping. Compare exact pre/post public rows and inspect hidden
   surface/test-inventory changes per the release cadence.
4. Run the relevant Python suites and repository-required source gate on the
   final candidate. Request independent `gpt-6-sol` high code review and
   Terra verification; resolve findings with focused RED/GREEN where needed.
5. Record evidence and exact SHA in status; merge into `release/0.8.27`,
   advance its state and generated views, verify from Git, then remove the
   temporary worktree and branch.
