---
title: 0.8.24 Slice 0 — environment and project-infrastructure baseline
status: PROPOSED
target_release: 0.8.24
---

# Slice 0 — environment and project-infrastructure baseline

## Objective

Produce the evidence and decision brief needed to start 0.8.24 safely, without
changing product code, CI, runner configuration, registry settings, dependency
versions, or the existing 0.8.23 release artifacts. The only setup action is
the already-created isolated `release/0.8.24` worktree.

## Baseline

| Item | Recorded state |
| --- | --- |
| Release branch | `release/0.8.24` |
| Worktree | `/tmp/fathomdb-release-0.8.24` |
| Base | `origin/main` at `ee9bb753e6847daa0e72902eed972a5498f4f556` |
| Shared checkout | Not an editing surface; it was behind `origin/main` at setup and has owner untracked files. |
| Preflight | Passed; the worktree was cut from current `origin/main`. |
| Existing release topology | `release.yml` uses hosted publish jobs, a `pypi` environment, trusted-publishing/OIDC permissions, and Linux x64 CUDA self-hosted jobs. |
| Current main CI evidence | Main advanced through `23258cb2`, `0985cfcd`, and `ee9bb753`; Slice 0 must inspect their workflow contract before proposing any CI work. |

## Boundaries

- **Allowed:** source inspection, GitHub/registry metadata queries, runner
  status/label queries, package metadata queries, locally generated evidence
  records, and the final decision brief.
- **Not allowed:** starting or approving workflows; registering/editing runners;
  editing repository/environment secrets; editing trusted publishers; publishing
  packages; changing Dependabot; creating a tag; or changing `main`.
- **Secret handling:** record only the existence/name of a secret or environment
  setting where GitHub exposes it. Never retrieve, print, copy, or rotate a
  secret value.
- **No false readiness:** an online runner is not a CUDA-capable executor until
  its labels, OS/toolchain/GPU facts, and intended workflow route have all been
  recorded.

## Work plan

### 0.1 Revalidate branch and main interface

1. Fetch `origin/main`; record the release worktree SHA, current `origin/main`
   SHA, and merge-base relation.
2. Inspect the main-only CI delta beginning at `23258cb2` and its documentation
   follow-up. Record affected workflow files, trigger rules, path filters,
   changed-job routing, artifact use, and whether the work is already complete.
3. Identify files that any later release slice could contend for, especially
   `.github/workflows/{ci,release}.yml`, release scripts, package metadata, and
   compatibility documentation.
4. State the integration rule: main-owned CI changes land on main by their own
   path; this release branch may only rebase/integrate a verified main result.

**Output:** `main-ci-interface.md`, including a no-change conclusion if the
new CI workflow work already satisfies its intended contract.

### 0.2 Inventory publication topology and package identities

1. Read the release workflow and release scripts to map each artifact to its
   build job, upload mechanism, registry project/package name, trusted-publisher
   identity, environment, and post-publish smoke.
2. Query public package metadata only for existing FathomDB npm, PyPI, and
   crates.io targets. Do not create a placeholder project or attempt upload.
3. Record the current Tegra build contract, including the local `+tegra` form,
   JetPack/glibc assumptions, and why it is not a PyPI upload candidate.
4. Compare three possible public Tegra routes at the packaging/installer level:
   separately named PyPI distribution, another approved public index, or an
   alternative explicitly selected by the owner. Reject any route that depends
   on pip inferring CUDA from two same-name ARM64 wheels.
5. Record the existing CPU Python/npm artifact families and their ABI/platform
   contracts so later slices can prove preservation.
6. Trace the current idempotency guards for cargo, PyPI, and npm, including
   registry-query failure behavior and retry semantics.

**Output:** `publication-topology.md`, with a package matrix and an explicit
list of unknown external configuration prerequisites.

### 0.3 Inventory target executors and environments

1. Query GitHub’s runner metadata for the repository/organization as permitted:
   runner names, online state, labels, and group—not secrets or tokens.
2. Correlate each release job’s `runs-on` selector with an actual available
   executor. This includes the existing Linux x64 CUDA selector, GitHub-hosted
   Windows jobs, hosted ARM64 jobs, and the Jetson/Tegra route.
3. For the intended Jetson route, record OS/L4T/JetPack, Python, Rust, CUDA,
   GPU compute capability, glibc, network/cache prerequisites, and the command
   that will later perform a clean installed-package smoke. Do not build or
   publish during Slice 0.
4. For the intended Windows route, determine whether an approved remote Windows
   CUDA executor exists. If it does, record labels, GPU, CUDA toolkit/MSVC/Python
   versions, signing/trust boundary, and artifact transfer route. If it does
   not, record that as a blocking decision—not a request to compile locally.
5. Verify that candidate target routes can preserve the CPU lane independently
   from CUDA selection, at the contract level only.

**Output:** `executor-inventory.md`, with every selector classified available,
unavailable, unknown, or unsuitable and the evidence source/time.

### 0.4 Inspect current CI and release controls

1. Establish how the new proportional CI routing on main treats documentation,
   release scripts, workflow files, Windows paths, and platform artifacts.
2. Inspect release-workflow permissions, environment names, OIDC/trusted
   publisher setup points, upload/download artifact boundaries, and dry-run
   behavior without dispatching a run.
3. Map current Windows WAL diagnostics/attribution jobs in `ci.yml` to the
   separately requested Python-SDK WAL review. Record what they prove, what they
   do not prove, and the relationship to the Memex job
   `32587291032/97065598178`.
4. Identify the smallest possible CI change surface, if any, for Slice 10;
   record “none” when main already provides sufficient coverage.

**Output:** `ci-and-release-controls.md`, with an explicit distinction between
hosted CI evidence, self-hosted CUDA evidence, and registry publication.

### 0.5 Locate benchmark evidence and performance decision inputs

1. Identify the performance-benchmark branch and its authoritative committed or
   retained result files. Do not infer an engine change from a branch name or a
   stale narrative.
2. Record baseline hardware, corpus/workload, parameters, metric, variance,
   current result, and regression constraints needed for an engineering decision.
3. Decide whether the evidence is sufficient to plan a measured Slice 20 or
   requires an owner-provided branch/ref, artifact, or decision rule.

**Output:** `benchmark-evidence-index.md`, containing evidence references only;
no new benchmark run is part of Slice 0.

### 0.6 Create the owner decision brief

1. Consolidate 0.1–0.5 into `slice-0-decision-brief.md`.
2. For every candidate action, state evidence, uncertainty, risk, effort,
   recommended destination slice, and whether it is blocked on an owner choice.
3. Present exactly these decisions for later Slice 6 review:

   - public Tegra package identity and installer contract;
   - Windows CUDA SDK surface: Python, npm, or both;
   - remote Windows CUDA executor/trust route;
   - whether new main CI work needs any Slice 10 follow-on; and
   - benchmark evidence source and performance decision rule.

No decision is treated as accepted until the Slice 6 interactive HITL session.

## Exit criteria

- The five evidence records above exist, cite concrete sources, and contain no
  secret material.
- The public distribution options state why `+tegra` cannot be the PyPI shape.
- Every relevant job selector has a runner classification; a missing Windows
  CUDA executor is visible as a decision/blocker rather than hidden in a plan.
- The existing CPU, publisher-idempotency, and installed-smoke contracts are
  mapped before any new artifact is designed.
- The main CI work is classified accurately as sufficient, insufficient, or
  not yet understood—never assumed.
- The benchmark source is concrete or explicitly recorded as missing.
- The owner receives the decision brief as Slice 6 input; no external setting
  or product file was changed by this slice.

## Verification

- Check each evidence link/path against the recorded SHA or external query
  timestamp.
- Run the plan/documentation lint after writing the records.
- Re-run `scripts/preflight.sh --worktree /tmp/fathomdb-release-0.8.24` before
  any later implementation worktree is created.
