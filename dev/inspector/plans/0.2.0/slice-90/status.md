---
title: Inspector 0.2.0 Slice 90 status
date: 2026-11-21
status: COMPLETE
---

# Slice90 — COMPLETE

Inspector0.2.0 is released locally at **`dist/inspector/0.2.0/`** in
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0` on
`inspector/dev/0.2.0`. The exact qualified80 candidate was promoted through
verified sibling staging and atomic no-overwrite rename, then verified at the
final destination. This is the actual local distribution, not a rehearsal.
[INS-020-LOCAL-RELEASE](../release-decisions.md) authorizes it; the five-person
study was removed, no sessions occurred and no human pass is claimed.

## Deliverables and provenance

Final directory contains the tar, directly usable extracted bundle, license,
install README, build description, inner/outer checksums and `candidate.json`.
All8 final files are Git-ignored; `git ls-files dist` is empty. No payload is
tracked. Project, user and contributor documentation now states actual local
availability and exact installation/support limits.

| Identity | Value |
| --- | --- |
| Source/build packaging revision | `7449ee0108d503cbed3b6a33e2c841072d6de0ce` |
| Pre-promotion clean documentation HEAD | `b7b0ce1df4a3780026ba4cc93d6c2f459eda804b` |
| Archive | `dist/inspector/0.2.0/fathomdb-inspector-0.2.0-linux-x86_64.tar.gz` |
| Archive SHA256 | `0a731d3243b2c5b2e2611578ac3a9c06b4f12ba488af558c4a9530c341dc8e0f` |
| Executable | `dist/inspector/0.2.0/fathomdb-inspector-0.2.0-linux-x86_64/fathomdb-inspector` |
| Executable SHA256 | `4fce1f5b03bde56021721d48f990b1427e33a712c40231c9fb15266ee856d2d9` |
| Actual embedded build | `f1e9337dacdf79f77b60dd382a197eea340aa816aa3f1cc96992ab059ddbf7d1` |
| Selected unchanged Engine / schema | `6edc1155f039f87cd0a37660cb27ae4373bbce71` /37 |

The Git commit adding this release status is the release-record commit, distinct
from the source/build revision above. The final `candidate.json` retains the
actual qualified source SHA; later Markdown closure does not relabel it.
Inspector0.2.0 is GNUx86_64 release, cached CPU true, fixture control false,
Rust1.95.0. Unsigned local metadata/checksums do not claim a registry signature
or reproducible compilation.

Qualified pairing: Ubuntu22.04/glibc2.35 userland, kernel7.0.0-43-generic,
LandlockABI8 and Chromium156.0.8078.4; maximum ELFGLIBC2.34. Stock22/kernel5.15
cannot provide cached-mode ABI>=3. Existing pinned BGE assets remain prerequisites,
not bundled/downloaded/repaired. No broader OS/browser/platform claim is made.

## Review and final-destination verification

Fresh direct Sol-high design review APPROVE: requirements/AC R90-01–06 and80's
method already cover the slice. [Plan reconciliation](plan.md#execution-reconciliation--2026-11-21)
enumerates the completed work and closure deltas. No new product requirement,
architecture, source, executable script or test change was needed; no artificial
RED/GREEN was created for artifact copying or Markdown updates. Final Sol-high
release-diff/metadata review approved after clarifying existing-destination refusal
and the approved design link. No material finding remains.

- Candidate `verify --source`, actual promotion and final `verify --source` PASS
  from clean pre-promotion HEAD. Independent final outer/inner hashes, fresh final
  archive extraction, executable byte equality, help/build identity, ELF maximum,
  all8 ignored paths and zero tracked dist files PASS.
- Direct independent Terra final installed tests **2/2 PASS**, fail0/skipped0:
  real person biography/indexed text/head vector/L2/nativeCPU/edge/source/history/
  exact Back; separate A/B union/intersection/origin/Back. Actual fresh extraction
  of the **final dist tar** ran in the qualified Ubuntu22 pairing with normal
  Chromium sandbox. Node/Python were external test tools, not product runtimes.
- Normal native Alt+F4 closes: **194ms** and **159ms**; successful supervisor,
  removed private state, independent nonblocking lock reacquisition and all
  recorded owned descendants gone. At25ms sampling, person service peak
  **289,034,240B**, browser tree **1,643,151,360B**, model conservative upper bound
  **289,034,240B**; comparison service **29,708,288B**, browser **1,510,518,784B**.
  Samples do not claim instantaneous peaks.
- Final installed person trace:52 files, **0 Internet-family socket/connect
  attempts**, **3 pinned asset opens O_RDONLY**, no cache mutation calls.
  Original published review store SHA remains
  `597b909dca5f487e909b37bcc6d2bd1f70e4b9fd582524fbd3d15bb277aa7e4e`.
- Scoped Markdown/public-doc lint, strict MkDocs and Git whitespace checks PASS.
  The unchanged80 source phase retains its actual source SHA:206/206 strict
  suites,12 tooling,10JS units and81 installed checks PASS.90 does not claim a
  new full source gate at its later documentation commit.

Raw independent evidence: `/tmp/inspector90-terra-final/network-unconfined/`
and `/tmp/inspector90-terra-final/comparison.log`. Initial sandbox Docker-socket
refusal occurred before application launch and remains in `network/runtime.log`;
the unchanged qualification then passed on the authorized unconfined executor.
`--version` is unsupported and returned `invalid_arguments`; supported `--help`
and `--build-description` supplied version/build identity. An intermediate
Markdown link check found the not-yet-written90 status; it passed after this
record was added. No failed attempt is counted as a positive test.

## Cleanup and boundary

Owned test containers/processes and staging are gone; final ignored distribution
remains available. Original fixtures/databases/cache, prior70/75/80 artifacts and
unrelated dist entries are preserved. The existing release checkout, target
symlink and checkout Python environment are retained; no extra worktree/branch
was created, so no extra integration/cleanup is needed. Source/build/runtime
ownership returns to the parent after clean normal-hook commit and final verify.
Primary checkout, Engine/SDK/MCP/API/schema/ranker/pins/other versions are unchanged.
No tag, push, GitHub Release, registry upload, remote deployment or main merge.
