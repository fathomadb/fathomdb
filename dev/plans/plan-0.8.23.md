---
title: FathomDB 0.8.23 — CUDA-capable artifacts and Memex integration feedback
status: ACTIVE
target_release: 0.8.23
---

# FathomDB 0.8.23 — CUDA-capable artifacts and Memex integration feedback

0.8.23 makes the published Linux x86_64 Python wheel and Node native binary
capable of using NVIDIA CUDA. CPU remains the runtime default; CUDA is an
explicit opt-in through `FATHOMDB_EMBED_DEVICE=cuda:N`.

This release packages existing CUDA embedder support and makes embedding-dependent
graph integration diagnosable to SDK consumers. It does not introduce a new GPU
backend, automatic GPU selection, or a change to the CPU-only, deterministic
retrieval path.

## Goal and scope

1. Ship a GPU-capable Linux x86_64 binary in the normal PyPI and npm packages.
2. Preserve install and CPU operation on hosts without an NVIDIA driver.
3. Require a real CUDA smoke before a 0.8.23 publish can proceed.
4. Keep ordinary GitHub-hosted CI CPU-only; GPU work runs only on a
   workflow-restricted organization or enterprise runner group.
5. Make graph writes that need embedding observable and actionable across the
   Rust, Python, and TypeScript SDKs, rather than leaving clients to infer a
   configuration mistake from retry exhaustion.

**Out of scope:** Windows CUDA artifacts, macOS Metal artifacts, dynamic GPU
selection, NVML-based scheduling, memory budgeting, and ONNX execution-provider
work. Their capability must remain explicitly unavailable until each platform
has a reproducible build and hardware smoke.

## Release requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| G23-1 | Linux x86_64 npm and PyPI release artifacts compile the existing CUDA embedder. | Contract tests inspect the exact release build arguments and prove the N-API feature path forwards `embed-cuda`. |
| G23-2 | A CUDA-built artifact remains usable without an NVIDIA driver when CPU is selected. | A driverless Linux container or host runs the release-equivalent open/write/search/close/exit CPU smoke. |
| G23-3 | An explicit CUDA request uses a real NVIDIA device in the shipped-artifact path. | On `windchill3-fathomdb-cuda`, a release-equivalent artifact runs with `FATHOMDB_EMBED_DEVICE=cuda:0`; the smoke PID is observed on the selected CUDA device. |
| G23-4 | GPU release evidence cannot be silently replaced by ordinary CI or untrusted code. | A workflow-restricted organization or enterprise runner group admits only the verified release workflow ref used for publication and requires the CUDA build-and-smoke job. |
| G23-5 | Users can tell what is supported. | Platform capability metadata and public installation/embedder documentation name Linux x86_64 CUDA only and retain CPU fallback semantics. |
| G23-6 | A client can distinguish absent embedder configuration from scheduler/runtime failure. | Rust, Python, and TypeScript expose a stable typed code, remediation, and readiness/report surface; a body-bearing graph edge without an embedder does not consume retry backoff before giving feedback. |
| G23-7 | Memex graph integration has a tested, documented supported configuration. | Direct public-SDK controls classify body-bearing and structural edges, live versus absent embedder, retrieval-candidate policy, sequential stores, and child-process lifecycle; an actual current-main runtime failure receives a separate remediation slice. |

## Slice ladder

```text
0 → 10 → 5
```

| Slice | Title | Depends on |
| ---: | --- | --- |
| 0 | CUDA artifact contract, dependency probe, and protected runner gate | — |
| 10 | Memex integration feedback, readiness, and lifecycle characterization | 0 |
| 5 | Build, package, release-gate, and installed-artifact CUDA smoke | 0, 10 |

## Reserved-gap policy

Any reproduction that shows a live configured embedder still leaves graph work
undrainable, or that a fresh parent process aborts in a public-SDK reproduction,
is a release-blocking reliability defect. It receives a separate RED/GREEN
remediation slice before Slice 5; it is not folded silently into the CUDA
packaging work. Expected no-embedder deferred work is Slice 10's typed,
actionable SDK-feedback contract, not a generic drain timeout.

## Slice 0 — CUDA artifact contract, dependency probe, and protected runner gate

### Requirements

- Freeze the Linux x86_64-only CUDA artifact policy and CPU-default behavior.
- Establish whether the actual shared objects require a CUDA runtime at process
  load, then prove the CPU-without-NVIDIA case in a genuinely driverless
  environment rather than infer it from `ldd`.
- Verify the runner group admits the actual release workflow ref; the dedicated
  host already belongs to the restricted organization group. A repository-level
  runner is not an adequate boundary for this public repository.

### Acceptance criteria

- A red-first contract test fails against the current release configuration:
  N-API has no `embed-cuda` forwarding feature and Linux release jobs omit it.
- The design review records the `readelf`/`ldd` dependency result for the
  release-equivalent Node binary and Python extension, and the driverless
  container/host command that passes the G23-2 CPU smoke.
- The selected-repository, workflow-restricted `fathomdb-gpu-release` group in
  `fathomadb` permits the trusted `release.yml` main reference. Before a tagged
  release, it must permit the actual protected publication ref: either a
  main-pinned dispatch path or a pre-tag allow-list entry for the immutable
  `v0.8.23` release workflow ref.
- A non-publishing preflight proves the chosen workflow restriction accepts the
  release ref and rejects a pull-request workflow ref before the runner starts.
- CUDA 12.6, the installed driver, `nvcc`, and the exact manylinux/maturin
  build strategy are captured in a preflight witness before Slice 5 starts.

### Design and design review

The design of record is
`dev/design/0.8.23-gpu-artifacts.md`. Its review must close these questions
before Slice 5 starts: runtime-library behavior on driverless Linux, exact
artifact ownership, release-workflow dependency graph, runner trust boundary,
the observable GPU-engagement witness. Slice 10 separately owns the public
client-feedback and graph-integration contract.

### Implementation discipline — TDD (RED/GREEN)

1. **RED:** add the release-artifact contract test. It must fail for the
   missing CUDA path.
2. **GREEN:** make only the minimum contract/probe and protected-runner changes
   required for the test and design witnesses to pass.
3. **REFACTOR:** remove duplicated build-feature spelling so one checked source
   owns each artifact's feature set.

## Slice 10 — Memex integration feedback, readiness, and lifecycle characterization

### Requirements

- Reproduce and classify the six Memex findings through public FathomDB SDK
  calls with safe, owned-synthetic data: sequential edges, sequential fresh
  stores, child-process then fresh-parent lifecycle, structural-edge graph
  expansion, independent embedder-versus-vector-candidate controls, and the
  requested graph-expansion-without-vector public query policy.
- Preserve compatibility for accepted deferred writes, but provide immediate,
  typed, non-retryable feedback whenever `drain()` can prove pending embedding
  work cannot progress because the session has no usable embedder.
- Provide a public, cross-SDK readiness/report surface so a client can inspect
  pending/blocked embedding work immediately after open, write, or an
  unsuccessful drain without parsing an opaque error string or trace output.
- Define a stable remediation contract: diagnostic code, affected operation,
  blocked/deferred state, safe remediation choices, and a documentation URL;
  do not expose edge-body content in diagnostics or traces.
- Document the supported Memex configuration: graph-edge projection needs an
  embedder independently of whether Memex elects to include vector candidates
  in its own ranking policy.

### Acceptance criteria

- RED-first public-SDK tests cover body-less and body-bearing graph edges with
  absent and usable embedders, two fresh stores in one process, and child then
  fresh-parent process control. A live-embedder drain failure or reproducible
  parent abnormal exit blocks the release and creates a dedicated remediation
  slice.
- The no-embedder body-bearing control returns `FDB_EMBEDDER_REQUIRED` before
  retry backoff. Rust, Python, and TypeScript expose the same machine-readable
  code, affected operation, state, remediations, and stable documentation link.
- Each SDK exposes an additive embedding-readiness/report method returning
  ready/processing/deferred/blocked state, usable-embedder availability,
  affected kinds, pending count, and the same diagnostic payload when blocked.
- A bounded lifecycle diagnostic/event and counters use that same code without
  leaking edge body text. Trace data may assist diagnosis, but trace collection
  is not required for a caller to receive configuration feedback.
- Direct public-SDK controls establish and document whether a structural
  body-less edge participates in the graph-expansion path. Memex's supported
  graph witness uses a live embedder for body-bearing edge projection while
  independently suppressing vector candidates in Memex when it needs a
  graph-only ranking arm.
- The graph-on/vector-off request receives a closed public-contract disposition:
  demonstrate an existing supported query shape across bindings, add a typed
  arm-selection/query-options surface with truthful `explain` counts, or
  explicitly document and validate that graph expansion requires vector
  candidates. No client may be left to infer this policy from fused results.
- Rust, Python, and TypeScript interface documents and a consumer guide show
  the readiness/report API and remediation. The external Memex findings receive
  a bounded response identifying either the corrected configuration or the
  FathomDB remediation commit.

### Design and design review

The designs of record are
`dev/design/0.8.23-embedding-configuration-feedback.md` and
`dev/design/0.8.23-memex-integration.md`. Review freezes the non-breaking
write-versus-drain behavior, error-envelope parity, report shape, lifecycle
event privacy, structural-edge traversal promise, and the boundary between
FathomDB embedder availability and Memex retrieval-candidate policy.

### Implementation discipline — TDD (RED/GREEN)

1. **RED:** add failing public-SDK characterizations and parity tests for the
   missing typed drain outcome and readiness report; preserve each initial
   failure command in the slice witness.
2. **GREEN:** implement the minimum Rust domain outcome, PyO3/N-API envelope,
   SDK methods, lifecycle diagnostic, and documentation to make the contract
   pass. Do not change a test merely to conceal an existing scheduler or
   lifecycle failure.
3. **REFACTOR:** centralize code/remediation/report conversion so all bindings
   retain the same contract and one source owns its documentation URL.

## Slice 5 — build, package, release-gate, and installed-artifact CUDA smoke

### Requirements

- Forward `embed-cuda` through the N-API crate and select it for the Linux
  x86_64 release NPM artifact and Python wheel.
- Build the Linux CUDA artifacts on the dedicated runner using the Slice 0
  verified CUDA 12.6/manylinux strategy; preserve all non-Linux-CUDA artifact
  builds unchanged.
- Gate publication on CPU and GPU smoke of release-equivalent artifacts; after
  publication, gate npm `latest` promotion, GitHub Release creation, and release
  completion on registry-installed CPU and GPU smokes.

### Acceptance criteria

- The N-API package exposes an `embed-cuda` feature that forwards to the engine;
  a release-equivalent Node build has that feature enabled only for Linux x86_64.
- The Linux x86_64 wheel is built with `pyo3/extension-module`,
  `default-embedder`, and `embed-cuda`; its driverless CPU smoke passes without
  a CUDA device request.
- The GPU smoke runs an installed npm package and installed PyPI wheel on
  `cuda:0`, verifies correct open/write/search/close/exit behavior, and records
  the spawned smoke PID on `cuda:0` as its CUDA engagement witness.
- The release workflow blocks before publish when the CUDA build, driverless CPU
  smoke, or GPU smoke fails. Registry-installed failures block npm `latest`, the
  GitHub Release, and release completion; they cannot retroactively block the
  immutable registry upload.
- Platform metadata, npm documentation, and public embedder documentation agree
  that CUDA is supported for Linux x86_64 only in this release.

### Design and design review

Slice 5 implements only the approved Slice 0 artifact contract. Review checks
the feature graph, artifact linkage, CPU fallback, CUDA witness, workflow label
isolation, and package/docs agreement. A finding that requires a new backend,
runtime-bundling policy, or Windows/macOS support returns to a reserved-gap
proposal; it does not widen this slice.

### Implementation discipline — TDD (RED/GREEN)

1. **RED:** land failing tests for N-API feature forwarding, Linux-only release
   arguments, runner-group restriction, driverless CPU fallback, and smoke-PID
   GPU witness parsing.
2. **GREEN:** implement the smallest Cargo, package-script, workflow, capability,
   and documentation changes that make each test pass; run the real CUDA smoke
   on the self-hosted runner.
3. **REFACTOR:** centralize release feature lists and smoke invocation so Python,
   npm, CI, and release workflows cannot silently drift.

## Cross-cutting DoD

- Every implementation change is red → green → refactor, with the initial
  failing command preserved in the slice witness.
- The CPU query/retrieval path remains CPU-only, deterministic, and unchanged.
- The final release rehearsal passes the Memex integration controls, all ordinary
  platform builds, the driverless CPU smoke, and the CUDA build/smoke. A release is not complete until
  registry-installed Python and npm smokes pass and npm `latest` plus the GitHub
  Release are promoted.
- The self-hosted runner may be used only by an organization or enterprise
  runner group with explicit workflow restriction; labels alone are not access
  control.

## Decisions recorded

- 2026-08-10 — HITL selected Linux x86_64 CUDA support as mandatory for 0.8.23;
  Windows CUDA follows only after its own reproducible build and hardware smoke.
- 2026-08-10 — CPU remains the default runtime device. CUDA is an explicit
  `FATHOMDB_EMBED_DEVICE=cuda:N` opt-in.
- 2026-08-10 — `coreyt/fathomdb` transferred to `fathomadb/fathomdb` so CUDA
  proof can use the selected-repository, workflow-restricted
  `fathomdb-gpu-release` organization group. It contains the persistent
  `windchill3-fathomdb-cuda` runner and the one-concurrent GitHub-hosted
  `fathomdb-gpu-t4` runner.

## Open configuration gate

The organization placement and runner group are complete. Before Slice 0 can
close, it must prove the publication-reference policy: the group currently
allows trusted `release.yml` on `main`, while normal publication evaluates the
workflow from an immutable `v*` tag. The preflight must test the selected
main-dispatch or exact-tag allow-list procedure before the first CUDA release.

## Immediate next slice

**Slice 0 — CUDA artifact contract, dependency probe, and protected runner
gate.** Commission it from a fresh `main` worktree after the 0.8.23 release
state is opened.

<!-- BEGIN GENERATED release-state:0.8.23:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 0** (`CUDA-CONTRACT`) — CUDA artifact contract, dependency probe, and protected runner gate

**Remaining ladder:** 0 → 10 → 5.<!-- END GENERATED release-state:0.8.23:plan-immediate-next -->
